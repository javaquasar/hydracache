use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Read, Write};

use serde::Serialize;

use super::*;

pub(super) const WINDOW_INTERVAL: Duration = Duration::from_millis(100);

pub(super) struct Profile {
    pub duration: Duration,
    pub live_drain: bool,
    evidence: Option<PathBuf>,
}

#[derive(Debug, Default, Serialize)]
struct Round {
    sequence: u64,
    elapsed_ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    verified_at_ms: Option<u128>,
    put_value_sha256: String,
    cas_value_sha256: String,
    put_ack: bool,
    cas_ack: bool,
    delete_ack: bool,
    unknown: u64,
}

#[derive(Debug, Default, Serialize)]
pub(super) struct Counts {
    pub rounds: u64,
    pub acknowledgements: u64,
    pub verified: u64,
    pub unknown: u64,
}

#[derive(Serialize)]
pub(super) struct Fault {
    pub killed_pid: u32,
    pub killed_at_ms: u64,
    pub recovered_at_ms: u64,
    pub rejoined_pid: u32,
    pub rejoined_at_ms: u64,
}

const DRAIN_PROBE_VALUE: &[u8] = b"w10i-live-drain-probe";

#[derive(Debug, Serialize)]
pub(super) struct Drain {
    departed_pid: u32,
    returned_pid: u32,
    started_at_ms: u64,
    completed_at_ms: u64,
    audited_at_ms: u64,
    returned_at_ms: u64,
    partition: u32,
    certified_version: u64,
    value_sha256: String,
    before: [CopyObservation; 2],
    after: [CopyObservation; 2],
}

#[derive(Debug, Serialize)]
struct CopyObservation {
    member_slot: u8,
    epoch: u64,
    certified_version: u64,
    primary: bool,
    backup: bool,
}

impl Profile {
    pub fn from_env() -> Self {
        let (duration, evidence) = match std::env::var_os("HYDRACACHE_W10I_RUN_DIR") {
            Some(path) => {
                let seconds: u64 = std::env::var("HYDRACACHE_W10I_DURATION_SECONDS")
                    .unwrap()
                    .parse()
                    .unwrap();
                assert!((10..=60).contains(&seconds) || matches!(seconds, 21600 | 86400));
                if seconds == 86400 {
                    let expected_daemon =
                        std::env::var("HYDRACACHE_W10I_EXPECTED_DAEMON_SHA256").unwrap();
                    let expected_runner =
                        std::env::var("HYDRACACHE_W10I_EXPECTED_RUNNER_SHA256").unwrap();
                    assert_eq!(
                        expected_daemon,
                        digest_file(Path::new(env!("CARGO_BIN_EXE_hydracache-server"))).unwrap(),
                        "daemon changed since the six-hour run"
                    );
                    assert_eq!(
                        expected_runner,
                        digest_file(&std::env::current_exe().unwrap()).unwrap(),
                        "runner changed since the six-hour run"
                    );
                }
                (Duration::from_secs(seconds), Some(PathBuf::from(path)))
            }
            None => (Duration::from_secs(5), None),
        };
        let live_drain =
            evidence.is_some() && std::env::var("HYDRACACHE_W10I_LIVE_DRAIN").as_deref() == Ok("1");
        assert!(!live_drain || duration >= Duration::from_secs(45));
        Self {
            duration,
            evidence,
            live_drain,
        }
    }

    pub fn ledger(&self) -> io::Result<Option<BufWriter<File>>> {
        self.evidence
            .as_ref()
            .map(|root| {
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(root.join("ledger.jsonl"))
                    .map(BufWriter::new)
            })
            .transpose()
    }

    pub fn receipt(
        &self,
        started: tokio::time::Instant,
        counts: &Counts,
        fault: Fault,
        drain: Option<Drain>,
    ) -> io::Result<()> {
        let Some(root) = &self.evidence else {
            return Ok(());
        };
        assert_eq!(self.live_drain, drain.is_some());
        let mut receipt = serde_json::json!({
            "schema_version": if self.live_drain { 2 } else { 1 },
            "production_admitted": false,
            "source_sha": std::env::var("HYDRACACHE_W10I_SOURCE_SHA").unwrap(),
            "seconds": self.duration.as_secs(),
            "elapsed_ms": started.elapsed().as_millis(),
            "daemon_sha256": digest_file(Path::new(env!("CARGO_BIN_EXE_hydracache-server")))?,
            "runner_sha256": digest_file(&std::env::current_exe()?)?,
            "ledger_sha256": digest_file(&root.join("ledger.jsonl"))?,
            "fault": fault,
            "counts": counts
        });
        if let Some(drain) = drain {
            receipt["drain"] = serde_json::to_value(drain)?;
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join("worker.json"))?;
        serde_json::to_writer(&mut file, &receipt)?;
        file.sync_all()
    }
}

fn digest_file(path: &Path) -> io::Result<String> {
    let mut hash = Sha256::new();
    let mut file = File::open(path)?;
    let mut buffer = [0; 16 * 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(hex_digest(&hash.finalize()))
}

pub(super) async fn exercise_control(
    writer: Hc2Client,
    reader: Hc2Client,
    key: Bytes,
    start: tokio::time::Instant,
    duration: Duration,
    mut ledger: Option<BufWriter<File>>,
    record_completion: bool,
) -> Counts {
    let mut counts = Counts::default();
    while start.elapsed() < duration {
        let seq = counts.rounds;
        let value = Bytes::from(format!("control-put-{seq}"));
        let replacement = Bytes::from(format!("control-cas-{seq}"));
        let mut round = Round {
            sequence: seq,
            elapsed_ms: start.elapsed().as_millis(),
            put_value_sha256: sha256_hex(&value),
            cas_value_sha256: sha256_hex(&replacement),
            ..Round::default()
        };
        round.put_ack = acknowledged(
            writer.put(key.clone(), value.clone(), None, None).await,
            &mut round.unknown,
        );
        if round.put_ack {
            wait_w10i_get(&reader, key.clone(), Some(&value)).await;
            round.cas_ack = acknowledged(
                writer
                    .compare_and_set(key.clone(), value, replacement.clone(), None, None)
                    .await,
                &mut round.unknown,
            );
            if round.cas_ack {
                wait_w10i_get(&reader, key.clone(), Some(&replacement)).await;
                round.delete_ack =
                    acknowledged(reader.delete(key.clone(), None).await, &mut round.unknown);
                if round.delete_ack {
                    wait_w10i_get(&writer, key.clone(), None).await;
                }
            }
        }
        let verified =
            u64::from(round.put_ack) + u64::from(round.cas_ack) + u64::from(round.delete_ack);
        counts.rounds += 1;
        counts.acknowledgements += verified;
        counts.verified += verified;
        counts.unknown += round.unknown;
        if record_completion {
            round.verified_at_ms = Some(start.elapsed().as_millis());
        }
        if let Some(file) = ledger.as_mut() {
            serde_json::to_writer(&mut *file, &round).unwrap();
            file.write_all(b"\n").unwrap();
        }
        tokio::time::sleep(WINDOW_INTERVAL).await;
    }
    if let Some(file) = ledger.as_mut() {
        file.flush().unwrap();
        file.get_ref().sync_all().unwrap();
    }
    counts
}

pub(super) async fn exercise_live_drain(
    cluster: &mut MemberCluster,
    started: tokio::time::Instant,
    duration: Duration,
) -> Drain {
    // Leave five seconds for measured traffic after the drained member returns.
    tokio::time::timeout_at(started + duration - Duration::from_secs(5), async {
        use hydracache_client_hc2::BatchOperation;

        let http = w10h_http();
        let admins = &cluster.spec.addresses[6..9];
        let ready = wait_cluster_imap(&http, admins[1], |body| {
            body["ready"] == true && cluster_imap_members(body).len() == 3
        })
        .await;
        let seed_epoch = ready["epoch"].as_u64().unwrap();
        let seed_identity = Bytes::from(format!("w10e:{seed_epoch}:w10i-live-drain-seed"));

        // The earlier fault probe used primary 0/backup 1. This is a different, untouched partition.
        let key = cluster.key_for(0, Some(2), "w10i-live-drain/");
        let value = Bytes::from_static(DRAIN_PROBE_VALUE);
        let receipt = retry_w10i(|| async {
            let reply = cluster.clients[1]
                .batch_outcomes(
                    vec![BatchOperation::Put {
                        key: key.clone(),
                        value: value.clone(),
                        ttl: None,
                    }],
                    Some(RequestOptions {
                        timeout: Duration::from_secs(4),
                        idempotency_key: seed_identity.clone(),
                    }),
                )
                .await?;
            assert!(
                reply[0]
                    .result
                    .as_ref()
                    .map_err(Clone::clone)?
                    .mutation
                    .unwrap()
                    .applied
            );
            Ok(reply[0].receipt.unwrap())
        })
        .await;
        let partition = receipt.partition_id;
        let certified_version = receipt.applied.unwrap().version;
        println!("W10i live drain seed partition={partition} epoch={seed_epoch} version={certified_version}");
        let before = observe_copies(
            &http,
            admins,
            [0, 2],
            partition,
            certified_version,
            |pair| pair[0].primary && pair[1].backup,
        )
        .await;

        tokio::time::sleep_until(started + duration / 2).await;
        let departed_pid = cluster.daemons[0].0.id();
        let started_at_ms = started.elapsed().as_millis() as u64;
        let response = w10h_admin(http.post(format!("http://{}{ADMIN_DRAIN_PATH}", admins[0])))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json::<serde_json::Value>()
            .await
            .unwrap();
        let completed_at_ms = started.elapsed().as_millis() as u64;
        assert_eq!(response["drain"]["timed_out"], false);
        println!("W10i live drain completed started_at_ms={started_at_ms} completed_at_ms={completed_at_ms}");
        wait_success(&mut cluster.daemons[0].0, Duration::from_secs(8));
        wait_management_json(&http, admins[1], ADMIN_CLUSTER_OVERVIEW_PATH, |body| {
            body["members"].as_array().is_some_and(|members| {
                members.len() == 2
                    && members
                        .iter()
                        .all(|member| member["node_id"] != cluster.spec.names[0])
            })
        })
        .await;
        let after = observe_copies(
            &http,
            admins,
            [1, 2],
            partition,
            certified_version,
            |pair| pair[0].primary != pair[1].primary && pair[0].epoch > before[0].epoch,
        )
        .await;
        for client in &cluster.clients[1..] {
            wait_w10i_get(client, key.clone(), Some(&value)).await;
        }
        let audited_at_ms = started.elapsed().as_millis() as u64;
        println!("W10i live drain placement audited epoch={} at_ms={audited_at_ms}", after[0].epoch);
        cluster.daemons[0] = cluster.spec.spawn_after_drain(0);
        let returned = connect_w10i(&cluster.adapters[0], "w10i").await;
        wait_cluster_imap(&http, admins[1], |body| {
            body["ready"] == true && cluster_imap_members(body).len() == 3
        })
        .await;
        wait_w10i_get(&returned, key, Some(&value)).await;
        cluster.clients[0] = returned;
        let returned_at_ms = started.elapsed().as_millis() as u64;
        let returned_pid = cluster.daemons[0].0.id();
        Drain {
            departed_pid,
            returned_pid,
            started_at_ms,
            completed_at_ms,
            audited_at_ms,
            returned_at_ms,
            partition,
            certified_version,
            value_sha256: sha256_hex(&value),
            before,
            after,
        }
    })
    .await
    .expect("W10i live drain and rejoin exceeded the campaign window")
}

async fn observe_copies(
    http: &reqwest::Client,
    admins: &[SocketAddr],
    slots: [u8; 2],
    partition: u32,
    version: u64,
    matches: impl Fn(&[CopyObservation; 2]) -> bool,
) -> [CopyObservation; 2] {
    loop {
        let (first, second) = tokio::join!(
            observe_copy(
                http,
                admins[usize::from(slots[0])],
                slots[0],
                partition,
                version
            ),
            observe_copy(
                http,
                admins[usize::from(slots[1])],
                slots[1],
                partition,
                version
            ),
        );
        if let (Some(first), Some(second)) = (first, second) {
            let pair = [first, second];
            if pair[0].epoch == pair[1].epoch && matches(&pair) {
                return pair;
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

async fn observe_copy(
    http: &reqwest::Client,
    admin: SocketAddr,
    member_slot: u8,
    partition: u32,
    version: u64,
) -> Option<CopyObservation> {
    async fn page(http: &reqwest::Client, url: &str) -> Option<serde_json::Value> {
        w10h_admin(http.get(url))
            .timeout(Duration::from_secs(1))
            .send()
            .await
            .ok()?
            .error_for_status()
            .ok()?
            .json()
            .await
            .ok()
    }
    let url = format!("http://{admin}{MANAGEMENT_IMAP_PATH}?limit=1");
    let first = page(http, &url).await?;
    let body = if let Some(after) = partition.checked_sub(1) {
        let sequence = first["observation_sequence"].as_u64()?;
        page(http, &format!("{url}&after={after}&sequence={sequence}")).await?
    } else {
        first
    };
    let row = &body["partitions"][0];
    let primary = row["local_primary"].as_bool()?;
    let backup = row["local_backup"].as_bool()?;
    let certified_version = row["certified_version"].as_u64()?;
    let epoch = body["observation_epoch"].as_u64()?;
    (body["complete"] == true
        && row["id"] == partition
        && row["state"] == "healthy"
        && certified_version == version
        && primary != backup)
        .then_some(CopyObservation {
            member_slot,
            epoch,
            certified_version,
            primary,
            backup,
        })
}

fn acknowledged(
    result: Result<hydracache_client_hc2::MutationResult, hydracache_client_hc2::ClientError>,
    unknown: &mut u64,
) -> bool {
    match result {
        Ok(reply) => {
            assert!(
                reply.applied,
                "the uncontended control partition refused an exact mutation"
            );
            true
        }
        Err(error)
            if matches!(
                error.code(),
                ErrorCode::Unavailable | ErrorCode::DeadlineExceeded
            ) =>
        {
            *unknown += 1;
            false
        }
        Err(error) => panic!("control operation failed definitively: {error}"),
    }
}
