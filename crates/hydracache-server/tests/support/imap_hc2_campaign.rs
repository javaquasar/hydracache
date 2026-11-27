use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Read, Write};

use hydracache_server::{
    IMapReplicaDigest, HYDRACACHE_MANAGEMENT_READ_HEADER, MANAGEMENT_IMAP_REPLICA_DIGEST_PATH,
};
use serde::Serialize;

use super::*;

pub(super) const WINDOW_INTERVAL: Duration = Duration::from_millis(100);

pub(super) struct Profile {
    pub duration: Duration,
    pub live_drain: bool,
    pub interrupt_copy: bool,
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
    pub audit: OwnerKillAudit,
}

#[derive(Serialize)]
pub(super) struct OwnerKillAudit {
    pub partition: u32,
    pub before: ReplicaCheckpoint,
    pub promoted: ReplicaCheckpoint,
    pub recovered: ReplicaCheckpoint,
    pub rejoined: ReplicaCheckpoint,
    pub tombstoned: ReplicaCheckpoint,
    pub returned_member: hydracache::PartitionReplicaIdentity,
    pub returned_epoch: u64,
}

#[derive(Serialize)]
pub(super) struct ReplicaCheckpoint {
    placement: [CopyObservation; 2],
    digests: [IMapReplicaDigest; 2],
}

impl ReplicaCheckpoint {
    pub fn epoch(&self) -> u64 {
        self.placement[0].epoch
    }

    pub fn primary(&self) -> &hydracache::PartitionReplicaIdentity {
        &self.digests[0].member
    }

    pub fn assert_same_image(&self, other: &Self) {
        assert_same_image(&self.digests[0], &other.digests[0]);
    }
}

const DRAIN_PROBE_VALUE: &[u8] = b"w10i-live-drain-probe";
static INTERRUPTED_COPY_VALUE: [u8; 96 * 1024] = [0x5a; 96 * 1024];
const COPY_INTERRUPTION_HOLD: Duration = Duration::from_millis(1_500);

#[derive(Debug, Serialize)]
pub(super) struct Drain {
    departed_pid: u32,
    returned_pid: u32,
    started_at_ms: u64,
    completed_at_ms: u64,
    audited_at_ms: u64,
    pub(super) returned_at_ms: u64,
    partition: u32,
    certified_version: u64,
    value_sha256: String,
    before: [CopyObservation; 2],
    after: [CopyObservation; 2],
    replica_digests: ReplicaReconciliation,
    #[serde(skip_serializing_if = "Option::is_none")]
    interrupted_copy: Option<InterruptedCopy>,
    #[serde(skip)]
    pub(super) probe: (Bytes, Bytes),
}

impl Drain {
    pub(super) fn interrupted_window(&self) -> Option<std::ops::Range<u64>> {
        let copy = self.interrupted_copy.as_ref()?;
        Some(copy.hit_at_ms..copy.resumed_at_ms)
    }
}

#[derive(Debug, Serialize)]
struct ReplicaReconciliation {
    before: [IMapReplicaDigest; 2],
    after: [IMapReplicaDigest; 2],
}

#[derive(Debug, Serialize)]
struct InterruptedCopy {
    hit_at_ms: u64,
    observed_at_ms: u64,
    resumed_at_ms: u64,
    blocked_chunk: u32,
    forbidden_restart_chunk: u32,
    held_placement: [CopyObservation; 3],
}

#[derive(Debug, Serialize)]
struct CopyObservation {
    member_slot: u8,
    epoch: u64,
    certified_version: u64,
    primary: bool,
    backup: bool,
    #[serde(skip)]
    healthy: bool,
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
        let interrupt_copy = evidence.is_some()
            && std::env::var("HYDRACACHE_W10I_INTERRUPT_COPY").as_deref() == Ok("1");
        assert!(!live_drain || duration >= Duration::from_secs(45));
        assert!(!interrupt_copy || live_drain);
        Self {
            duration,
            evidence,
            live_drain,
            interrupt_copy,
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
        assert_eq!(
            self.interrupt_copy,
            drain.as_ref().is_some_and(|d| d.interrupted_copy.is_some())
        );
        let mut receipt = serde_json::json!({
            "schema_version": 6 + u16::from(self.live_drain) + u16::from(self.interrupt_copy),
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
    interrupt_copy: bool,
    hot_partition: hydracache::PartitionId,
    while_copy_held: impl std::future::Future<Output = ()>,
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

        // After the kill, slot 1 owns the promoted and control partitions. For the copy
        // scenario slot 1 holds the probe backup, so the less loaded slot 2 needs a snapshot.
        let backup_slot = if interrupt_copy { 1 } else { 2 };
        let key = cluster.pick_key(0, Some(backup_slot), |i| format!("w10i-live-drain/{i}").into_bytes(), |partition| partition != hot_partition);
        let value = Bytes::from_static(if interrupt_copy { &INTERRUPTED_COPY_VALUE } else { DRAIN_PROBE_VALUE });
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
        let ReplicaCheckpoint { placement: before, digests: before_digests } = capture_cut(
            &http,
            &cluster.spec,
            [0, backup_slot as u8],
            partition,
            certified_version,
            |pair| pair[0].primary && pair[1].backup,
        )
        .await;
        check_digest_refusals(&http, admins, partition, &before).await;

        tokio::time::sleep_until(started + duration / 2).await;
        let departed_pid = cluster.daemons[0].0.id();
        if interrupt_copy {
            set_copy_fault(&cluster.spec, 2, 1);
        }
        let started_at_ms = started.elapsed().as_millis() as u64;
        let (drained, interrupted_copy) = tokio::join!(async {
            let response = w10h_admin(http.post(format!("http://{}{ADMIN_DRAIN_PATH}", admins[0])))
                .send().await.unwrap().error_for_status().unwrap().json::<serde_json::Value>().await.unwrap();
            (response, started.elapsed().as_millis() as u64)
        }, async {
            if interrupt_copy {
                Some(interrupt_and_resume(&cluster.spec, &http, admins, started, partition, &before[0], while_copy_held).await)
            } else {
                None
            }
        });
        let (response, completed_at_ms) = drained;
        assert_eq!(response["drain"]["timed_out"], false);
        println!("W10i live drain completed started_at_ms={started_at_ms} completed_at_ms={completed_at_ms}");
        wait_success(&mut cluster.daemons[0].0, Duration::from_secs(8));
        if let Some(proof) = &interrupted_copy {
            assert!(proof.resumed_at_ms < completed_at_ms);
            assert!(!copy_fault_hit(&cluster.spec, 3, 0), "resumed transfer tried to replay its first chunk");
            cluster.spec.set_outbound_fault(0, 2, 4, None);
        }
        wait_management_json(&http, admins[1], ADMIN_CLUSTER_OVERVIEW_PATH, |body| {
            body["members"].as_array().is_some_and(|members| {
                members.len() == 2
                    && members
                        .iter()
                        .all(|member| member["node_id"] != cluster.spec.names[0])
            })
        })
        .await;
        let ReplicaCheckpoint { placement: after, digests: after_digests } = capture_cut(
            &http,
            &cluster.spec,
            [1, 2],
            partition,
            certified_version,
            |pair| pair[0].primary != pair[1].primary && pair[0].epoch > before[0].epoch
                && (!interrupt_copy || pair[1].primary),
        )
        .await;
        for client in &cluster.clients[1..] {
            wait_w10i_get(client, key.clone(), Some(&value)).await;
        }
        assert_same_image(&before_digests[0], &after_digests[0]);
        let audited_at_ms = started.elapsed().as_millis() as u64;
        println!("W10i live drain placement audited epoch={} at_ms={audited_at_ms}", after[0].epoch);
        cluster.daemons[0] = cluster.spec.spawn_after_drain(0);
        let returned = connect_w10i(&cluster.adapters[0], "w10i").await;
        wait_cluster_imap(&http, admins[1], |body| {
            body["ready"] == true && cluster_imap_members(body).len() == 3
        })
        .await;
        wait_w10i_get(&returned, key.clone(), Some(&value)).await;
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
            replica_digests: ReplicaReconciliation { before: before_digests, after: after_digests },
            interrupted_copy,
            probe: (key, value),
        }
    })
    .await
    .expect("W10i live drain and rejoin exceeded the campaign window")
}

fn digest_request(
    http: &reqwest::Client,
    admin: SocketAddr,
    partition: u32,
    epoch: u64,
    version: u64,
) -> reqwest::RequestBuilder {
    http.get(format!("http://{admin}{MANAGEMENT_IMAP_REPLICA_DIGEST_PATH}?partition={partition}&epoch={epoch}&version={version}"))
        .timeout(Duration::from_secs(3))
}

fn digest_reader(request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    request
        .header(HYDRACACHE_CLIENT_ID_HEADER, "w10i-digest")
        .header(HYDRACACHE_TENANT_HEADER, "system")
        .header(HYDRACACHE_MANAGEMENT_READ_HEADER, "true")
}

pub(super) async fn owner_checkpoint(
    http: &reqwest::Client,
    spec: &MemberClusterSpec,
    slots: [u8; 2],
    partition: u32,
    version: u64,
    min_epoch: u64,
) -> ReplicaCheckpoint {
    tokio::time::timeout(Duration::from_secs(12), capture_cut(http, spec, slots, partition, version, |pair| {
        pair[0].primary && pair[1].backup && pair[0].epoch >= min_epoch
    }))
    .await
    .unwrap_or_else(|_| panic!("no matching IMap replicas: slots={slots:?} partition={partition} version={version} min_epoch={min_epoch}"))
}

async fn capture_cut(
    http: &reqwest::Client,
    spec: &MemberClusterSpec,
    slots: [u8; 2],
    partition: u32,
    version: u64,
    matches: impl Fn(&[CopyObservation; 2]) -> bool,
) -> ReplicaCheckpoint {
    loop {
        let placement = observe_copies(
            http,
            &spec.addresses[6..9],
            slots,
            partition,
            version,
            &matches,
        )
        .await;
        if let Some(digests) = reconcile_copies(http, spec, partition, &placement).await {
            return ReplicaCheckpoint { placement, digests };
        }
        // A concurrent Raft change can stale this epoch without changing the partition version.
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

async fn reconcile_copies(
    http: &reqwest::Client,
    spec: &MemberClusterSpec,
    partition: u32,
    placement: &[CopyObservation; 2],
) -> Option<[IMapReplicaDigest; 2]> {
    let capture = |copy: &CopyObservation| {
        let slot = usize::from(copy.member_slot);
        let (epoch, version, primary) = (copy.epoch, copy.certified_version, copy.primary);
        async move {
            let response = digest_reader(digest_request(
                http,
                spec.addresses[6 + slot],
                partition,
                epoch,
                version,
            ))
            .send()
            .await
            .unwrap();
            if matches!(
                response.status(),
                reqwest::StatusCode::SERVICE_UNAVAILABLE | reqwest::StatusCode::TOO_MANY_REQUESTS
            ) {
                return None;
            }
            let digest: IMapReplicaDigest =
                response.error_for_status().unwrap().json().await.unwrap();
            assert_eq!(digest.schema_version, 1);
            assert_eq!(digest.member.node.as_str(), spec.names[slot]);
            assert_eq!((digest.topology_epoch, digest.primary), (epoch, primary));
            assert_eq!(
                (digest.prefix.partition.value(), digest.prefix.version),
                (partition, version)
            );
            assert!(digest.rows > 0 && digest.retained_outcomes > 0);
            Some(digest)
        }
    };
    let (first, second) = tokio::join!(capture(&placement[0]), capture(&placement[1]));
    let [first, second] = [first?, second?];
    assert_same_image(&first, &second);
    Some([first, second])
}

pub(super) async fn observe_certified_pair(
    http: &reqwest::Client,
    spec: &MemberClusterSpec,
    slots: [u8; 2],
    partition: u32,
    min_version: u64,
) -> Option<ReplicaCheckpoint> {
    let (primary, backup) = tokio::join!(
        observe_latest_partition(http, spec.addresses[6 + usize::from(slots[0])], slots[0], partition),
        observe_latest_partition(http, spec.addresses[6 + usize::from(slots[1])], slots[1], partition),
    );
    let placement = [primary?, backup?];
    let [primary, backup] = &placement;
    if !(primary.primary && backup.backup && primary.healthy && backup.healthy
        && primary.epoch == backup.epoch && primary.certified_version >= min_version
        && primary.certified_version == backup.certified_version)
    {
        return None;
    }
    let digests = reconcile_copies(http, spec, partition, &placement).await?;
    Some(ReplicaCheckpoint { placement, digests })
}

fn assert_same_image(first: &IMapReplicaDigest, second: &IMapReplicaDigest) {
    assert_eq!(
        first.prefix, second.prefix,
        "certified prefix differs across replicas"
    );
    assert_eq!(
        first.image_sha256, second.image_sha256,
        "certified journal image differs across replicas"
    );
    assert_eq!(
        (first.rows, first.retained_outcomes),
        (second.rows, second.retained_outcomes)
    );
}

async fn check_digest_refusals(
    http: &reqwest::Client,
    admins: &[SocketAddr],
    partition: u32,
    placement: &[CopyObservation; 2],
) {
    use reqwest::StatusCode;

    let [owner, backup] = placement;
    let request = || {
        digest_request(
            http,
            admins[0],
            partition,
            owner.epoch,
            owner.certified_version,
        )
    };
    assert_eq!(
        request().send().await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request()
            .header(HYDRACACHE_CLIENT_ID_HEADER, "w10i-no-role")
            .header(HYDRACACHE_TENANT_HEADER, "system")
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    for (slot, part, epoch, version, expected) in [
        (
            0,
            D2_PARTITION_COUNT.get(),
            owner.epoch,
            owner.certified_version,
            StatusCode::BAD_REQUEST,
        ),
        (
            0,
            partition,
            owner.epoch - 1,
            owner.certified_version,
            StatusCode::SERVICE_UNAVAILABLE,
        ),
        (
            0,
            partition,
            owner.epoch,
            owner.certified_version + 1,
            StatusCode::SERVICE_UNAVAILABLE,
        ),
        (
            3 - usize::from(backup.member_slot),
            partition,
            owner.epoch,
            owner.certified_version,
            StatusCode::SERVICE_UNAVAILABLE,
        ),
    ] {
        let response = digest_reader(digest_request(http, admins[slot], part, epoch, version))
            .send()
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            expected,
            "slot={slot} partition={part} epoch={epoch} version={version}"
        );
    }
}

fn set_copy_fault(spec: &MemberClusterSpec, generation: u64, index: u32) {
    spec.set_outbound_fault(
        0,
        2,
        generation,
        Some(serde_json::json!({"kind":"value_chunk_drop", "index": index})),
    );
}

fn copy_fault_hit(spec: &MemberClusterSpec, generation: u64, index: u32) -> bool {
    let event = format!("HYDRACACHE_TEST_RAFT_OUTBOUND_FAULT_HIT generation={generation} from={} to={} action=ValueChunkDrop {{ index: {index} }}", spec.names[0], spec.names[2]);
    std::fs::read_to_string(spec.root.join("member-0.log"))
        .unwrap()
        .lines()
        .any(|line| line == event)
}

async fn interrupt_and_resume(
    spec: &MemberClusterSpec,
    http: &reqwest::Client,
    admins: &[SocketAddr],
    started: tokio::time::Instant,
    partition: u32,
    before: &CopyObservation,
    while_copy_held: impl std::future::Future<Output = ()>,
) -> InterruptedCopy {
    while !copy_fault_hit(spec, 2, 1) {
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let hit_at_ms = started.elapsed().as_millis() as u64;
    println!("W10i interrupted snapshot copy partition={partition} chunk=1 at_ms={hit_at_ms}");
    tokio::time::sleep(COPY_INTERRUPTION_HOLD).await;
    while_copy_held.await;
    let held_placement = loop {
        let version = before.certified_version;
        let (source, backup, target) = tokio::join!(
            observe_partition(http, admins[0], 0, partition, version),
            observe_partition(http, admins[1], 1, partition, version),
            observe_partition(http, admins[2], 2, partition, version),
        );
        if let (Some(source), Some(backup), Some(target)) = (source, backup, target) {
            assert!(
                source.primary
                    && !source.backup
                    && backup.backup
                    && !backup.primary
                    && !target.primary
                    && !target.backup,
                "copy changed placement before it could commit"
            );
            let placement = [source, backup, target];
            assert!(
                placement
                    .iter()
                    .all(|p| Some(p.epoch) == before.epoch.checked_add(1)),
                "copy was observed outside its Raft wave epoch"
            );
            break placement;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
    let observed_at_ms = started.elapsed().as_millis() as u64;
    // Keep denying chunk zero. A full restart cannot complete this transfer.
    set_copy_fault(spec, 3, 0);
    let resumed_at_ms = started.elapsed().as_millis() as u64;
    InterruptedCopy {
        hit_at_ms,
        observed_at_ms,
        resumed_at_ms,
        blocked_chunk: 1,
        forbidden_restart_chunk: 0,
        held_placement,
    }
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
            observe_partition(
                http,
                admins[usize::from(slots[0])],
                slots[0],
                partition,
                version
            ),
            observe_partition(
                http,
                admins[usize::from(slots[1])],
                slots[1],
                partition,
                version
            ),
        );
        if let (Some(first), Some(second)) = (first, second) {
            let pair = [first, second];
            if pair[0].epoch == pair[1].epoch
                && pair
                    .iter()
                    .all(|copy| copy.healthy && copy.primary != copy.backup)
                && matches(&pair)
            {
                return pair;
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

async fn observe_partition(
    http: &reqwest::Client,
    admin: SocketAddr,
    member_slot: u8,
    partition: u32,
    version: u64,
) -> Option<CopyObservation> {
    observe_latest_partition(http, admin, member_slot, partition)
        .await
        .filter(|observation| observation.certified_version == version)
}

async fn observe_latest_partition(
    http: &reqwest::Client,
    admin: SocketAddr,
    member_slot: u8,
    partition: u32,
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
    (body["complete"] == true && row["id"] == partition).then_some(
        CopyObservation {
            member_slot,
            epoch,
            certified_version,
            primary,
            backup,
            healthy: row["state"] == "healthy",
        },
    )
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
