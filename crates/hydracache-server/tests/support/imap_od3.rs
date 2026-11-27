//! Measure write probes and independent replica recovery after one process loss.

use std::collections::{BTreeMap, BTreeSet};

use futures_util::{stream, StreamExt};
use serde::Serialize;
use tokio::sync::Semaphore;
use tokio::time::Instant;

use super::*;
use imap_hc2_campaign::{observe_certified_pair, owner_checkpoint, ReplicaCheckpoint};

const VICTIM: u8 = 0;
const SURVIVORS: [u8; 2] = [1, 2];
const PARALLELISM: usize = 8;
const POLL_INTERVAL: Duration = Duration::from_millis(100);
const PROBE_TIMEOUT: Duration = Duration::from_secs(180);

struct Partition {
    id: u32,
    original: [u8; 2],
    keys: Vec<Bytes>,
}

impl Partition {
    fn surviving_pair(&self) -> [u8; 2] {
        match self.original {
            [VICTIM, 1] | [1, _] => [1, 2],
            _ => [2, 1],
        }
    }
}

#[derive(Serialize)]
struct WriteProbe {
    first_attempt_ms: u64,
    first_acknowledged_ms: u64,
    last_unavailable_reply_ms: Option<u64>,
    timeouts: u64,
}

#[derive(Serialize)]
struct PartitionEvidence {
    partition: u32,
    original: [u8; 2],
    surviving: [u8; 2],
    epoch_before: u64,
    epoch_after: u64,
    replica_proof_observed_ms: u64,
    probe: WriteProbe,
    baseline: ReplicaCheckpoint,
    recovered: ReplicaCheckpoint,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct Totals {
    affected_partitions: usize,
    control_partitions: usize,
    last_refusal_partition_ms: u64,
    first_ack_partition_ms: u64,
    last_replica_proof_observed_ms: u64,
}

fn totals(rows: &[PartitionEvidence]) -> Totals {
    let mut totals = Totals {
        affected_partitions: 0,
        control_partitions: 0,
        last_refusal_partition_ms: 0,
        first_ack_partition_ms: 0,
        last_replica_proof_observed_ms: 0,
    };
    for row in rows {
        if row.original.contains(&VICTIM) {
            totals.affected_partitions += 1;
            totals.last_refusal_partition_ms += row.probe.last_unavailable_reply_ms.unwrap_or(0);
            totals.first_ack_partition_ms += row.probe.first_acknowledged_ms;
            totals.last_replica_proof_observed_ms = totals
                .last_replica_proof_observed_ms
                .max(row.replica_proof_observed_ms);
        } else {
            totals.control_partitions += 1;
        }
    }
    totals
}

fn plan(names: &[String], all: bool, keys_per_partition: usize) -> Vec<Partition> {
    let members = placement_members(names);
    let mut pairs = BTreeSet::new();
    let mut selected = BTreeMap::new();
    for id in 0..D2_PARTITION_COUNT.get() {
        let placement = RendezvousClusterOwnership
            .replicas_for_key(&format!("partition-{id}"), &members, 2)
            .unwrap();
        let slot = |node| {
            members
                .iter()
                .position(|member| member.node_id == node)
                .unwrap() as u8
        };
        let original = [slot(placement.primary), slot(placement.backups[0].clone())];
        let first = pairs.insert(original);
        if all || first {
            selected.insert(
                id,
                Partition {
                    id,
                    original,
                    keys: Vec::new(),
                },
            );
        }
    }
    let scope = fixture_imap_scope();
    let mut needed = keys_per_partition * selected.len();
    for sequence in 0.. {
        let key = format!("od3-witness-{sequence}").into_bytes();
        let (id, _) = scope.locate(key.clone()).unwrap();
        if let Some(partition) = selected.get_mut(&id.value()) {
            if partition.keys.len() < keys_per_partition {
                partition.keys.push(Bytes::from(key));
                needed -= 1;
                if needed == 0 {
                    break;
                }
            }
        }
    }
    selected.into_values().collect()
}

async fn probe_write(
    client: &Hc2Client,
    partition: &Partition,
    replacement: &Bytes,
    killed: Instant,
    capacity: &Semaphore,
) -> WriteProbe {
    let key = &partition.keys[0];
    let mut permit = capacity.acquire().await.unwrap();
    let first_attempt_ms = millis(killed);
    let mut last_unavailable_reply_ms = None;
    let mut timeouts = 0;
    let mut sequence = 0;
    loop {
        sequence += 1;
        let reply = client
            .put(
                key.clone(),
                replacement.clone(),
                None,
                Some(RequestOptions {
                    timeout: Duration::from_millis(750),
                    idempotency_key: Bytes::from(format!("od3-probe-{}-{sequence}", partition.id)),
                }),
            )
            .await;
        drop(permit);
        match reply {
            Ok(reply) => {
                assert!(reply.applied);
                return WriteProbe {
                    first_attempt_ms,
                    first_acknowledged_ms: millis(killed),
                    last_unavailable_reply_ms,
                    timeouts,
                };
            }
            Err(err) if err.code() == ErrorCode::Unavailable => {
                last_unavailable_reply_ms = Some(millis(killed));
            }
            Err(err) if err.code() == ErrorCode::DeadlineExceeded => timeouts += 1,
            Err(err) => panic!("partition {} write probe failed: {err}", partition.id),
        }
        tokio::time::sleep(POLL_INTERVAL).await;
        permit = capacity.acquire().await.unwrap();
    }
}

fn millis(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_millis()).unwrap()
}

fn profile_size(name: &str, default: usize, limit: usize) -> usize {
    std::env::var(name).ok().map_or(default, |value| {
        let value = value.parse().unwrap_or_else(|_| panic!("invalid {name}"));
        assert!(
            (1..=limit).contains(&value),
            "{name} is out of diagnostic range"
        );
        value
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn measures_partition_write_outage_and_replica_recovery_on_hc2() {
    if std::env::var("HYDRACACHE_RUN_DAEMON_PROCESS_E2E").as_deref() != Ok("1") {
        return;
    }
    let _guard = async_daemon_process_guard().await;
    let mut cluster = MemberCluster::start("distributed-imap-od3").await;
    let http = w10h_http();
    let all = std::env::var("HYDRACACHE_OD3_ALL_PARTITIONS").as_deref() == Ok("1");
    let keys_per_partition = profile_size("HYDRACACHE_OD3_KEYS_PER_PARTITION", 1, 512);
    let payload_bytes = profile_size("HYDRACACHE_OD3_VALUE_BYTES", 256, 96 * 1024);
    let selected = plan(&cluster.spec.names, all, keys_per_partition);
    let logical_bytes = selected.len() as u64 * keys_per_partition as u64 * payload_bytes as u64;
    assert!(logical_bytes <= 256 * 1024 * 1024);
    let value = Bytes::from(vec![0x5a; payload_bytes]);
    let replacement = Bytes::from(vec![0x66; payload_bytes]);
    let version = keys_per_partition as u64;
    let baseline = stream::iter(&selected)
        .map(|partition| {
            let cluster = &cluster;
            let http = &http;
            let value = &value;
            async move {
                for (sequence, key) in partition.keys.iter().enumerate() {
                    assert!(
                        cluster.clients[1]
                            .put(
                                key.clone(),
                                value.clone(),
                                None,
                                Some(RequestOptions {
                                    timeout: Duration::from_secs(2),
                                    idempotency_key: Bytes::from(format!(
                                        "od3-baseline-{}-{sequence}",
                                        partition.id
                                    )),
                                }),
                            )
                            .await
                            .unwrap()
                            .applied
                    );
                }
                owner_checkpoint(
                    http,
                    &cluster.spec,
                    partition.original,
                    partition.id,
                    version,
                    1,
                )
                .await
            }
        })
        .buffered(PARALLELISM)
        .collect::<Vec<_>>()
        .await;

    let killed = Instant::now();
    cluster.daemons[usize::from(VICTIM)].0.kill().unwrap();
    cluster.daemons[usize::from(VICTIM)].0.wait().unwrap();
    let writes = Semaphore::new(PARALLELISM);
    let confirmations = Semaphore::new(PARALLELISM);
    let mut rows = tokio::time::timeout(PROBE_TIMEOUT, async {
        stream::iter(selected.iter().zip(baseline))
            .map(|(partition, baseline)| {
                let cluster = &cluster;
                let http = &http;
                let replacement = &replacement;
                let writes = &writes;
                let confirmations = &confirmations;
                async move {
                    let slots = partition.surviving_pair();
                    let probe =
                        probe_write(&cluster.clients[1], partition, replacement, killed, writes);
                    let recover = async {
                        loop {
                            let permit = confirmations.acquire().await.unwrap();
                            let cut = observe_certified_pair(
                                http,
                                &cluster.spec,
                                slots,
                                partition.id,
                                version,
                            )
                            .await;
                            drop(permit);
                            if let Some(cut) = cut {
                                return (millis(killed), cut);
                            }
                            tokio::time::sleep(POLL_INTERVAL).await;
                        }
                    };
                    let (probe, (replica_proof_observed_ms, recovered)) =
                        tokio::join!(probe, recover);
                    for slot in SURVIVORS {
                        wait_w10i_get(
                            &cluster.clients[usize::from(slot)],
                            partition.keys[0].clone(),
                            Some(replacement),
                        )
                        .await;
                    }
                    assert_eq!(baseline.rows(), keys_per_partition);
                    assert_eq!(recovered.rows(), keys_per_partition);
                    PartitionEvidence {
                        partition: partition.id,
                        original: partition.original,
                        surviving: slots,
                        epoch_before: baseline.epoch(),
                        epoch_after: recovered.epoch(),
                        replica_proof_observed_ms,
                        probe,
                        baseline,
                        recovered,
                    }
                }
            })
            .buffer_unordered(selected.len())
            .collect::<Vec<_>>()
            .await
    })
    .await
    .expect("partition write recovery did not complete within the diagnostic budget");
    rows.sort_by_key(|row| row.partition);
    let summary = totals(&rows);
    assert!(summary.affected_partitions > 0 && summary.control_partitions > 0);
    let report = serde_json::json!({
        "schema_version": 1,
        "production_admitted": false,
        "killed_member_slot": VICTIM,
        "keys_per_partition": keys_per_partition,
        "payload_bytes": payload_bytes,
        "concurrency": PARALLELISM,
        "totals": summary,
        "partitions": rows
    });
    if let Some(path) = std::env::var_os("HYDRACACHE_OD3_REPORT") {
        let file = std::fs::File::create_new(path).unwrap();
        serde_json::to_writer_pretty(&file, &report).unwrap();
        file.sync_all().unwrap();
    }
    println!("OD-3 HC/2 diagnostic: {summary:?}; profile={keys_per_partition} keys/partition x {payload_bytes} bytes");
}

#[test]
fn selects_every_placement_and_full_partition_coverages() {
    let names = ["member-1".into(), "member-2".into(), "member-3".into()];
    let representative = plan(&names, false, 2);
    let all = plan(&names, true, 2);
    assert_eq!(all.len(), D2_PARTITION_COUNT.get() as usize);
    let original_pairs = |partitions: &[Partition]| {
        partitions
            .iter()
            .map(|partition| partition.original)
            .collect::<BTreeSet<_>>()
    };
    let audited = original_pairs(&representative);
    assert_eq!(audited, original_pairs(&all));
    assert_eq!(representative.len(), audited.len());
    assert!(audited.iter().any(|pair| pair.contains(&VICTIM)));
    assert!(audited.iter().any(|pair| !pair.contains(&VICTIM)));
    for partition in all {
        assert_eq!(partition.keys.len(), 2);
        assert!(partition
            .surviving_pair()
            .iter()
            .all(|slot| *slot != VICTIM));
    }
}
