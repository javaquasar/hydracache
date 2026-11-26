use super::*;

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::net::TcpListener as PortReservation;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Instant, SystemTime};

use hydracache::{
    ClusterGeneration, ClusterMember, ClusterNodeId, ClusterReplicationStrategy, ClusterRole,
    ClusterTimeAuthority, DurableValueStore, PartitionPromotion, PartitionReplicaRepair,
    PartitionValuePrefix, PrimaryRebalanceWave, RendezvousClusterOwnership, TimeObservation,
};
use hydracache_client_protocol::{
    CasExpectation, ClientErrorCode, ClientFrame, ClientRequest, ClientRequestEnvelope,
    ClientResponse, ClientWireMessage, LockConsistency, StructuredKey,
};
use hydracache_client_transport_axum::{
    CLIENT_DATA_PATH, HYDRACACHE_CLIENT_ID_HEADER, HYDRACACHE_TENANT_HEADER,
};
use hydracache_server::imap_topology::PartitionMember;
use hydracache_server::imap_value_plane::ReplicaClock;
use hydracache_server::{
    MemberReadHost, ReplicaRebalanceReport, ReplicaRepairOutcome, ServerConfig, ServerRole,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

const RUN: &str = "HYDRACACHE_RUN_DAEMON_PROCESS_E2E";
const CHILD: &str = "HYDRACACHE_IMAP_REPLICA_TEST_CHILD";
const LIVE: &str = "HYDRACACHE_IMAP_TEST_LIVE_WRITES";
const FAULT: &str = "HYDRACACHE_TEST_RAFT_OUTBOUND_FAULT_FILE";
const DEADLINE: &[u8] = b"-ERR HydraCache client error: deadline exceeded\r\n";

#[derive(Debug, Serialize, Deserialize)]
struct Status {
    pid: u32,
    sample: u64,
    members: usize,
    epoch: u64,
    generation: u64,
    receipt: u32,
    lookups: u64,
    version: u64,
    certificate: Option<PartitionValuePrefix>,
    applies: u64,
    owner_applies: u64,
    backup: Option<String>,
    repairs: BTreeMap<PartitionId, PartitionReplicaRepair>,
    promotions: BTreeMap<PartitionId, PartitionPromotion>,
    wave: Option<PrimaryRebalanceWave>,
}

#[derive(Serialize, Deserialize)]
struct PromotionRequest {
    serial: u32,
    partition: Option<PartitionId>,
    backups: Vec<PartitionMember>,
}

#[derive(Serialize, Deserialize)]
struct RebalanceCommand {
    partitions: Vec<PartitionId>,
    drained: BTreeSet<ClusterNodeId>,
}

#[derive(Serialize, Deserialize)]
enum WaveCommand {
    Start(Vec<(PartitionId, PartitionMember)>),
    Cancel(ClusterEpoch),
}

#[derive(Serialize, Deserialize)]
struct WaveRequest {
    serial: u32,
    command: WaveCommand,
}

#[derive(Serialize, Deserialize)]
struct WaveResponse {
    serial: u32,
    result: Result<PrimaryRebalanceWave, String>,
}

#[derive(Serialize, Deserialize)]
struct PromotionResponse {
    serial: u32,
    plan: Option<PartitionPromotion>,
}

#[derive(Serialize, Deserialize)]
struct RepairCommand {
    serial: u32,
    replaced: PartitionMember,
    target: PartitionMember,
}

#[derive(Serialize, Deserialize)]
struct RepairResponse {
    serial: u32,
    result: Result<ReplicaRepairOutcome, String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Receipt {
    serial: u32,
    epoch: u64,
    applied: u64,
}

#[derive(Deserialize)]
struct ClockSample {
    now_ms: u64,
    uncertainty_ms: u64,
}

struct FileClock(PathBuf);

impl ClusterTimeAuthority for FileClock {
    fn observe(&self) -> Option<TimeObservation> {
        let sample: ClockSample = read_json(&self.0)?;
        Some(TimeObservation {
            now_ms: sample.now_ms,
            uncertainty_ms: sample.uncertainty_ms,
        })
    }
}

#[tokio::test]
async fn imap_replica_child() {
    let Some(root) = std::env::var_os(CHILD).map(PathBuf::from) else {
        return;
    };
    assert_eq!(std::env::var(RUN).as_deref(), Ok("1"));
    let config: ServerConfig = read_json(&root.join("config.json")).unwrap();
    let identity = config.node_id.as_deref().unwrap();
    let host = MemberReadHost::start(&config).unwrap();
    let values = ReplicatedReadValues::<DurableValueStore>::new(scope());
    let replication = ReplicationConfig {
        replication_factor: 2,
        sync_backups: 1,
        ..ReplicationConfig::local_first()
    };
    if std::env::var(LIVE).as_deref() == Ok("1") {
        run_live(config.listen_addr, &root, identity, host, replication).await;
        return;
    }
    let dispatcher = host
        .dispatcher(Default::default(), replication, values.clone())
        .unwrap();
    let listener = TcpListener::bind(config.listen_addr).await.unwrap();
    let task = start_resp(listener, dispatcher);
    let partition = scope().locate(KEY.to_vec()).unwrap().0;
    let mut serial = 0;
    let mut sample = 0;
    loop {
        if let Some(receipt) = read_json::<Receipt>(&root.join("receipt.json")) {
            if receipt.serial > serial {
                let store = DurableValueStore::open(replica_path(&root, receipt.serial)).unwrap();
                values
                    .install(
                        partition,
                        prefix(ClusterEpoch::new(receipt.epoch), receipt.applied),
                        store,
                    )
                    .unwrap();
                serial = receipt.serial;
            }
        }
        if let Some(snapshot) = host.partition_snapshot() {
            sample += 1;
            let generation = snapshot
                .members
                .iter()
                .find(|member| member.node_id.as_str() == identity)
                .map_or(0, |member| member.generation.value());
            publish_json(
                &root.join("status.json"),
                &Status {
                    pid: std::process::id(),
                    sample,
                    members: snapshot.members.len(),
                    epoch: snapshot.epoch.value(),
                    generation,
                    receipt: serial,
                    lookups: values.store_lookups(),
                    version: 0,
                    certificate: None,
                    applies: 0,
                    owner_applies: 0,
                    backup: None,
                    repairs: BTreeMap::new(),
                    promotions: BTreeMap::new(),
                    wave: None,
                },
            );
        }
        assert!(!task.is_finished());
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

async fn run_live(
    address: SocketAddr,
    root: &Path,
    identity: &str,
    host: MemberReadHost,
    replication: ReplicationConfig,
) {
    let (dispatcher, values) = host
        .value_dispatcher_with_clock(
            &root.join("live-values"),
            Default::default(),
            replication,
            scope(),
            Some(ReplicaClock {
                authority: Arc::new(FileClock(root.join("clock.json"))),
                maximum_uncertainty_ms: 10,
            }),
        )
        .unwrap();
    let task = start_resp(
        TcpListener::bind(address).await.unwrap(),
        dispatcher.clone(),
    );
    let http_address: SocketAddr = read_json(&root.join("hc1-address.json")).unwrap();
    let http = TcpListener::bind(http_address).await.unwrap();
    let http_task =
        tokio::spawn(async move { axum::serve(http, dispatcher.routes()).await.unwrap() });
    let partition = scope().locate(KEY.to_vec()).unwrap().0;
    let mut sample = 0;
    let mut promotion_serial = 0;
    let mut repair_serial = 0;
    let mut rebalanced = false;
    let mut automatic = false;
    let mut wave_serial = 0;
    loop {
        if let Some(request) = read_json::<WaveRequest>(&root.join("wave.json")) {
            if request.serial > wave_serial {
                let result = match request.command {
                    WaveCommand::Start(targets) => host.start_primary_rebalance(&targets).await,
                    WaveCommand::Cancel(id) => host.cancel_primary_rebalance(id).await,
                }
                .map_err(|error| error.to_string());
                wave_serial = request.serial;
                publish_json(
                    &root.join("wave-reply.json"),
                    &WaveResponse {
                        serial: wave_serial,
                        result,
                    },
                );
            }
        }
        if let Some(request) = read_json::<PromotionRequest>(&root.join("promotion.json")) {
            if request.serial > promotion_serial {
                let plan = host
                    .promote_partition(request.partition.unwrap_or(partition), request.backups)
                    .await
                    .ok();
                promotion_serial = request.serial;
                publish_json(
                    &root.join("promotion-reply.json"),
                    &PromotionResponse {
                        serial: promotion_serial,
                        plan,
                    },
                );
            }
        }
        if !rebalanced {
            if let Some(command) = read_json::<RebalanceCommand>(&root.join("rebalance.json")) {
                let result = host
                    .rebalance_partition_backups(&command.partitions, &command.drained)
                    .await
                    .map_err(|error| error.to_string());
                publish_json(&root.join("rebalance-reply.json"), &result);
                rebalanced = true;
            }
        }
        if !automatic && root.join("automatic-rebalance.json").is_file() {
            host.start_automatic_replica_rebalance().unwrap();
            host.start_automatic_replica_rebalance().unwrap();
            automatic = true;
        }
        if let Some(command) = read_json::<RepairCommand>(&root.join("repair.json")) {
            if command.serial > repair_serial {
                let result = host
                    .repair_partition_backup(partition, command.replaced, command.target)
                    .await
                    .map_err(|error| error.to_string());
                repair_serial = command.serial;
                publish_json(
                    &root.join("repair-reply.json"),
                    &RepairResponse {
                        serial: repair_serial,
                        result,
                    },
                );
            }
        }
        if let Some(snapshot) = host.partition_snapshot() {
            sample += 1;
            let backup =
                PartitionTopology::from_snapshot(snapshot.clone(), D2_PARTITION_COUNT, replication)
                    .and_then(|topology| topology.placement(partition))
                    .ok()
                    .filter(|placement| placement.primary.node.as_str() == identity)
                    .and_then(|placement| {
                        placement
                            .backups
                            .first()
                            .map(|backup| backup.node.to_string())
                    });
            publish_json(
                &root.join("status.json"),
                &Status {
                    pid: std::process::id(),
                    sample,
                    members: snapshot.members.len(),
                    epoch: snapshot.epoch.value(),
                    generation: snapshot
                        .members
                        .iter()
                        .find(|member| member.node_id.as_str() == identity)
                        .map_or(0, |member| member.generation.value()),
                    receipt: 0,
                    lookups: values.store_lookups(),
                    version: snapshot
                        .value_prefixes
                        .get(&partition)
                        .map_or(0, |prefix| prefix.version),
                    certificate: snapshot.value_prefixes.get(&partition).copied(),
                    applies: values.replica_applies(),
                    owner_applies: values.owner_applies(),
                    backup,
                    repairs: snapshot.value_repairs,
                    promotions: snapshot.value_promotions,
                    wave: snapshot.primary_rebalance,
                },
            );
        }
        assert!(!task.is_finished() && !http_task.is_finished());
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

struct Node {
    root: PathBuf,
    address: SocketAddr,
    hc1_address: SocketAddr,
    identity: String,
    child: Option<Child>,
    live: bool,
}

impl Node {
    fn wave(&self, serial: u32, command: WaveCommand) -> Result<PrimaryRebalanceWave, String> {
        publish_json(
            &self.root.join("wave.json"),
            &WaveRequest { serial, command },
        );
        wait_for("primary wave control reply", || {
            read_json::<WaveResponse>(&self.root.join("wave-reply.json"))
                .filter(|reply| reply.serial == serial)
        })
        .result
    }

    fn start(&mut self) {
        assert!(self.child.is_none());
        let log = File::options()
            .create(true)
            .append(true)
            .open(self.root.join("child.log"))
            .unwrap();
        self.child = Some(
            Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "processes::imap_replica_child", "--nocapture"])
                .env(CHILD, &self.root)
                .env(LIVE, if self.live { "1" } else { "0" })
                .env(FAULT, self.root.join("fault.json"))
                .stdout(Stdio::from(log.try_clone().unwrap()))
                .stderr(Stdio::from(log))
                .spawn()
                .unwrap(),
        );
    }

    fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    fn status(&self) -> Option<Status> {
        let status = read_json::<Status>(&self.root.join("status.json"))?;
        (Some(status.pid) == self.child.as_ref().map(Child::id)).then_some(status)
    }

    fn refreshed_status(&self) -> Status {
        let sample = self.status().unwrap().sample;
        wait_for("an observation after the RESP reply", || {
            self.status().filter(|status| status.sample > sample)
        })
    }

    fn publish_replica(&self, serial: u32, epoch: u64, applied: u64, record_epoch: u64) {
        let path = replica_path(&self.root, serial);
        let mut store = DurableValueStore::open(&path).unwrap();
        if applied >= 8 {
            let (partition, key) = scope().locate(KEY.to_vec()).unwrap();
            store
                .upsert(
                    key,
                    ReplicatedValueRecord::value(
                        partition,
                        8,
                        ClusterEpoch::new(record_epoch),
                        VALUE,
                    ),
                )
                .unwrap();
        }
        store.flush().unwrap();
        drop(store);
        publish_json(
            &self.root.join("receipt.json"),
            &Receipt {
                serial,
                epoch,
                applied,
            },
        );
    }

    fn read(&self) -> io::Result<Vec<u8>> {
        resp(self.address, &[b"GET", KEY])
    }

    fn member(&self, generation: u64) -> PartitionMember {
        PartitionMember {
            node: ClusterNodeId::new(self.identity.as_str()),
            generation: ClusterGeneration::new(generation),
        }
    }

    fn promote(&self, serial: u32, backups: Vec<PartitionMember>) -> Option<PartitionPromotion> {
        self.promote_key(serial, None, backups)
    }

    fn promote_key(
        &self,
        serial: u32,
        partition: Option<PartitionId>,
        backups: Vec<PartitionMember>,
    ) -> Option<PartitionPromotion> {
        publish_json(
            &self.root.join("promotion.json"),
            &PromotionRequest {
                serial,
                partition,
                backups,
            },
        );
        wait_for("promotion control reply", || {
            read_json::<PromotionResponse>(&self.root.join("promotion-reply.json"))
                .filter(|reply| reply.serial == serial)
        })
        .plan
    }

    fn repair(
        &self,
        serial: u32,
        replaced: PartitionMember,
        target: PartitionMember,
    ) -> Result<ReplicaRepairOutcome, String> {
        self.start_repair(serial, replaced, target);
        self.repair_result(serial)
    }

    fn start_repair(&self, serial: u32, replaced: PartitionMember, target: PartitionMember) {
        publish_json(
            &self.root.join("repair.json"),
            &RepairCommand {
                serial,
                replaced,
                target,
            },
        );
    }

    fn repair_result(&self, serial: u32) -> Result<ReplicaRepairOutcome, String> {
        wait_for("snapshot transfer control reply", || {
            read_json::<RepairResponse>(&self.root.join("repair-reply.json"))
                .filter(|reply| reply.serial == serial)
        })
        .result
    }

    fn send_hc1(&self, request: ClientRequest, id: &str) -> TcpStream {
        let mut envelope = ClientRequestEnvelope::new(id, request);
        envelope.idempotency_key = Some(id.into());
        let frame = ClientFrame::from_message(&ClientWireMessage::Request(envelope))
            .unwrap()
            .encode()
            .unwrap();
        let mut stream = TcpStream::connect(self.hc1_address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        write!(stream,
            "POST {CLIENT_DATA_PATH} HTTP/1.1\r\nHost: {}\r\n{HYDRACACHE_CLIENT_ID_HEADER}: w10e\r\n{HYDRACACHE_TENANT_HEADER}: redis\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            self.hc1_address, frame.len(),
        ).unwrap();
        stream.write_all(&frame).unwrap();
        stream
    }

    fn hc1(
        &self,
        request: ClientRequest,
        id: &str,
    ) -> hydracache_client_transport_axum::ClientBackendOutcome {
        let mut reader = BufReader::new(self.send_hc1(request, id));
        let mut status = String::new();
        reader.read_line(&mut status).unwrap();
        assert!(status.starts_with("HTTP/1.1 200 "), "{status}");
        let mut length = None;
        loop {
            let mut header = String::new();
            assert_ne!(reader.read_line(&mut header).unwrap(), 0);
            if header == "\r\n" {
                break;
            }
            if let Some((name, value)) = header.split_once(':') {
                if name.eq_ignore_ascii_case("content-length") {
                    length = Some(value.trim().parse().unwrap());
                }
            }
        }
        let mut body = vec![0; length.unwrap()];
        reader.read_exact(&mut body).unwrap();
        let ClientWireMessage::Response(response) = ClientFrame::decode(&body, 1 << 20)
            .unwrap()
            .decode_message()
            .unwrap()
        else {
            panic!("expected HC/1 response");
        };
        response.result
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        self.stop();
    }
}

fn replica_path(root: &Path, serial: u32) -> PathBuf {
    root.join(format!("replica-{serial}"))
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

fn publish_json(path: &Path, document: &impl Serialize) {
    let temporary = path.with_extension("new");
    fs::write(&temporary, serde_json::to_vec(document).unwrap()).unwrap();
    fs::rename(temporary, path).unwrap();
}

fn wait_for<T>(what: &str, mut poll: impl FnMut() -> Option<T>) -> T {
    let deadline = Instant::now() + Duration::from_secs(25);
    loop {
        if let Some(result) = poll() {
            return result;
        }
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn all_members(nodes: &[Node], after: u64) -> Option<Vec<Status>> {
    let statuses = nodes.iter().map(Node::status).collect::<Option<Vec<_>>>()?;
    let epoch = statuses[0].epoch;
    (epoch > after
        && statuses.iter().all(|status| {
            status.members == nodes.len() && status.epoch == epoch && status.generation > 0
        }))
    .then_some(statuses)
}

fn expect_owner(nodes: &[Node], expected: &[u8]) -> usize {
    wait_for("one available partition owner through RESP", || {
        let mut owner = None;
        for (index, node) in nodes.iter().enumerate() {
            let response = node.read().ok()?;
            if response == expected {
                assert!(
                    owner.replace(index).is_none(),
                    "multiple owners returned the replica"
                );
            } else if !response.starts_with(b"-ERR ") {
                panic!("non-owner gave an unexpected reply {response:?}");
            }
        }
        owner
    })
}

fn fault_generation(nodes: &[Node], isolated: Option<usize>, generation: u32) {
    for (index, node) in nodes.iter().enumerate() {
        let rules: Vec<_> = nodes.iter().enumerate()
            .filter(|(peer_index, _)| *peer_index != index
                && (isolated == Some(index) || isolated == Some(*peer_index)))
            .map(|(_, peer)| json!({"from": node.identity, "to": peer.identity, "action": {"kind":"drop"}}))
            .collect();
        publish_json(
            &node.root.join("fault.json"),
            &json!({
                "schema_version": 1, "generation": generation, "rules": rules
            }),
        );
    }
}

fn start_nodes(live: bool) -> Vec<Node> {
    start_node_count(live, 3)
}

fn start_node_count(live: bool, count: usize) -> Vec<Node> {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::var_os("HYDRACACHE_IMAP_REPLICA_TEST_ARTIFACTS")
        .map_or_else(std::env::temp_dir, PathBuf::from)
        .join(format!("imap-replica-{}-{timestamp}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let reservations: Vec<_> = (0..3 * count)
        .map(|_| PortReservation::bind("127.0.0.1:0").unwrap())
        .collect();
    let addresses: Vec<_> = reservations
        .iter()
        .map(|socket| socket.local_addr().unwrap())
        .collect();
    let seeds: Vec<_> = addresses[..count].iter().map(ToString::to_string).collect();
    let mut nodes: Vec<_> = addresses[..count]
        .iter()
        .enumerate()
        .map(|(index, address)| {
            let identity = format!("member-{}", address.to_string().replace(['.', ':'], "-"));
            let node_root = root.join(format!("member-{index}"));
            fs::create_dir(&node_root).unwrap();
            let config = ServerConfig {
                role: ServerRole::Member,
                node_id: Some(identity.clone()),
                cluster_addr: *address,
                cluster_advertise_addr: Some(address.to_string()),
                listen_addr: addresses[index + count],
                seeds: seeds.clone(),
                storage_dir: Some(node_root.clone()),
                ..ServerConfig::default()
            };
            publish_json(&node_root.join("config.json"), &config);
            let hc1_address = addresses[index + 2 * count];
            publish_json(&node_root.join("hc1-address.json"), &hc1_address);
            Node {
                root: node_root,
                address: config.listen_addr,
                identity,
                hc1_address,
                child: None,
                live,
            }
        })
        .collect();
    fault_generation(&nodes, None, 0);
    drop(reservations);
    for node in &mut nodes {
        node.start();
    }
    println!("D2 process artifacts {}", root.display());
    nodes
}

fn compare_and_set(expected: &[u8], value: &[u8]) -> ClientRequest {
    compare_and_set_for_key(KEY, expected, value)
}

fn compare_and_set_for_key(key: &[u8], expected: &[u8], value: &[u8]) -> ClientRequest {
    let encoded = key
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    ClientRequest::CompareAndSet {
        ns: Namespace::new("redis").unwrap(),
        key: StructuredKey::new(vec![format!("redis-binary-v1-{encoded}")]).unwrap(),
        expected: CasExpectation::Exact(expected.to_vec()),
        new_value: value.to_vec(),
        level: LockConsistency::Quorum,
    }
}

fn with_rebalance_load<T>(
    address: SocketAddr,
    keys: &BTreeMap<PartitionId, Vec<u8>>,
    action: impl FnOnce() -> T,
) -> (T, BTreeMap<PartitionId, usize>) {
    let stop = AtomicBool::new(false);
    std::thread::scope(|scope| {
        let writer = scope.spawn(|| {
            let mut accepted = BTreeMap::new();
            let deadline = Instant::now() + Duration::from_secs(10);
            while !stop.load(Ordering::Acquire) && Instant::now() < deadline {
                for (partition, key) in keys {
                    match resp(address, &[b"SET", key, b"workload"]) {
                        Ok(reply) if reply == b"+OK\r\n" => {
                            *accepted.entry(*partition).or_insert(0usize) += 1
                        }
                        Ok(reply) => assert!(reply.starts_with(b"-ERR "), "{reply:?}"),
                        Err(error) => panic!("client transport failed: {error}"),
                    }
                    std::thread::sleep(Duration::from_millis(15));
                }
            }
            accepted
        });
        let result = action();
        stop.store(true, Ordering::Release);
        (result, writer.join().unwrap())
    })
}

fn fixture_members(nodes: &[Node], statuses: &[Status]) -> Vec<ClusterMember> {
    nodes
        .iter()
        .zip(statuses)
        .map(|(node, status)| ClusterMember {
            node_id: ClusterNodeId::new(&node.identity),
            generation: ClusterGeneration::new(status.generation),
            role: ClusterRole::Member,
            epoch: ClusterEpoch::new(status.epoch),
            endpoints: Default::default(),
            metadata: Default::default(),
        })
        .collect()
}

#[test]
fn w10f_primary_wave_moves_two_owners_and_fences_a_cancelled_copy_under_load() {
    if std::env::var(RUN).as_deref() != Ok("1") {
        return;
    }
    let nodes = start_node_count(true, 4);
    let initial = wait_for("four primary wave members", || all_members(&nodes, 0));
    let members = fixture_members(&nodes, &initial);
    let key_for = |source: usize, target: usize, seed_needed: bool| {
        (0..5000)
            .find_map(|index| {
                let key = format!("primary-wave-{index}").into_bytes();
                let partition = scope().locate(key.clone()).unwrap().0;
                let placement = RendezvousClusterOwnership
                    .replicas_for_key(&format!("partition-{}", partition.value()), &members, 2)
                    .unwrap();
                (placement.primary == members[source].node_id
                    && placement.backups.contains(&members[target].node_id) != seed_needed)
                    .then_some((partition, key))
            })
            .unwrap()
    };
    let first = key_for(0, 2, true);
    let second = key_for(1, 3, false);
    let routes = [
        (
            first.0,
            first.1.as_slice(),
            [nodes[0].address, nodes[2].address],
        ),
        (
            second.0,
            second.1.as_slice(),
            [nodes[1].address, nodes[3].address],
        ),
    ];
    for (index, (_, key, addresses)) in routes.iter().enumerate() {
        assert_eq!(
            resp(addresses[0], &[b"SET", key, b"seed"]).unwrap(),
            b"+OK\r\n"
        );
        assert_eq!(
            nodes[index]
                .hc1(
                    compare_and_set_for_key(key, b"seed", b"base"),
                    &format!("wave-seed-{index}")
                )
                .unwrap(),
            ClientResponse::CasApplied { new_version: 2 }
        );
    }
    let targets = vec![
        (first.0, nodes[2].member(initial[2].generation)),
        (second.0, nodes[3].member(initial[3].generation)),
    ];
    let set_fault = |source: usize, target: usize, generation: u32, action: serde_json::Value| {
        publish_json(
            &nodes[source].root.join("fault.json"),
            &json!({
                "schema_version": 1, "generation": generation, "rules": [{
                    "from": nodes[source].identity, "to": nodes[target].identity, "action": action
                }]
            }),
        );
    };
    set_fault(0, 2, 1, json!({"kind":"value_drop"}));
    let wave = nodes[3]
        .wave(1, WaveCommand::Start(targets.clone()))
        .unwrap();
    assert_eq!(wave.tasks[0].source.node, members[0].node_id);
    assert_eq!(wave.tasks[1].source.node, members[1].node_id);
    assert_eq!(nodes[1].wave(1, WaveCommand::Start(targets)).unwrap(), wave);
    assert!(nodes[3]
        .promote_key(1, Some(second.0), vec![wave.tasks[1].source.clone()])
        .is_none());

    let stop = AtomicBool::new(false);
    let writes_at_destination = [AtomicBool::new(false), AtomicBool::new(false)];
    let acknowledgements = std::thread::scope(|threads| {
        let writer = threads.spawn(|| {
            let deadline = Instant::now() + Duration::from_secs(15);
            let mut counts = [[0; 2]; 2];
            while !stop.load(Ordering::Acquire) && Instant::now() < deadline {
                for (index, (_, key, addresses)) in routes.iter().enumerate() {
                    for (side, address) in addresses.iter().enumerate() {
                        match resp(*address, &[b"SET", key, b"load"]) {
                            Ok(reply) if reply == b"+OK\r\n" => {
                                counts[index][side] += 1;
                                if side == 1 {
                                    writes_at_destination[index].store(true, Ordering::Release);
                                }
                                break;
                            }
                            Ok(reply) => assert!(reply.starts_with(b"-ERR "), "{reply:?}"),
                            Err(error)
                                if matches!(
                                    error.kind(),
                                    io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                                ) => {}
                            Err(error) => panic!("wave load lost its client transport: {error}"),
                        }
                    }
                }
                std::thread::sleep(Duration::from_millis(18));
            }
            counts
        });
        wait_for(
            "both owners accept writes while the head is blocked",
            || {
                routes
                    .iter()
                    .all(|(_, key, addresses)| {
                        resp(addresses[0], &[b"GET", key])
                            .is_ok_and(|reply| reply == b"$4\r\nload\r\n")
                    })
                    .then_some(())
            },
        );
        let blocked = nodes[1].refreshed_status();
        assert_eq!(blocked.wave.as_ref().unwrap().completed, 0);
        assert!(blocked.promotions.is_empty());
        set_fault(
            0,
            2,
            2,
            json!({"kind":"value_chunk_delay","index":0,"millis":650}),
        );
        wait_for(
            "both new primary endpoints accept foreground writes",
            || {
                writes_at_destination
                    .iter()
                    .all(|seen| seen.load(Ordering::Acquire))
                    .then_some(())
            },
        );
        stop.store(true, Ordering::Release);
        writer.join().unwrap()
    });
    for counts in acknowledgements {
        assert!(counts[0] > 0 && counts[1] > 0);
    }
    let completed = wait_for("the committed wave moves both owners", || {
        let status = nodes[3].status()?;
        let completed = status.wave.as_ref()?;
        (completed.id == wave.id && completed.completed == 2).then_some(status)
    });
    let first_cut = &completed.promotions[&first.0];
    let second_cut = &completed.promotions[&second.0];
    assert_eq!(first_cut.wave, Some(wave.id));
    assert_eq!(second_cut.wave, Some(wave.id));
    assert_eq!(first_cut.old_epoch.value(), wave.id.value() + 1);
    assert_eq!(first_cut.new_epoch, second_cut.old_epoch);
    assert!(first_cut.required.unwrap().version > 2 && second_cut.required.unwrap().version > 2);
    assert_eq!(first_cut.backups, vec![wave.tasks[0].source.clone()]);
    assert_eq!(second_cut.backups, vec![wave.tasks[1].source.clone()]);
    assert!(fs::read_to_string(nodes[0].root.join("child.log"))
        .unwrap()
        .contains("ValueChunkDelay { index: 0, millis: 650 }"));
    for (index, (partition, key, addresses)) in routes.iter().enumerate() {
        wait_for(
            "the former owner is fenced by its applied Raft view",
            || {
                (nodes[index].status()?.promotions.get(partition)?.new_owner
                    == wave.tasks[index].target)
                    .then_some(())
            },
        );
        assert_eq!(resp(addresses[0], &[b"GET", key]).unwrap(), UNAVAILABLE);
        assert!(resp(addresses[0], &[b"SET", key, b"stale"])
            .unwrap()
            .starts_with(b"-ERR "));
        assert_eq!(
            nodes[index + 2]
                .hc1(
                    compare_and_set_for_key(key, b"seed", b"base"),
                    &format!("wave-seed-{index}")
                )
                .unwrap(),
            ClientResponse::CasApplied { new_version: 2 }
        );
        let request = compare_and_set_for_key(key, b"load", b"verified");
        let id = format!("w10e:{}:after-wave-{}", completed.epoch, partition.value());
        let result = nodes[index + 2].hc1(request.clone(), &id).unwrap();
        assert!(matches!(result, ClientResponse::CasApplied { .. }));
        assert_eq!(nodes[index + 2].hc1(request, &id).unwrap(), result);
    }

    set_fault(
        2,
        3,
        1,
        json!({"kind":"value_chunk_delay","index":0,"millis":900}),
    );
    let next = nodes[3]
        .wave(
            2,
            WaveCommand::Start(vec![(first.0, nodes[3].member(initial[3].generation))]),
        )
        .unwrap();
    wait_for("a new snapshot is in flight before cancellation", || {
        fs::read_to_string(nodes[2].root.join("child.log"))
            .ok()?
            .contains("ValueChunkDelay { index: 0, millis: 900 }")
            .then_some(())
    });
    assert_eq!(
        resp(nodes[2].address, &[b"SET", &first.1, b"cancel-safe"]).unwrap(),
        b"+OK\r\n"
    );
    let cancelled = nodes[1].wave(2, WaveCommand::Cancel(next.id)).unwrap();
    assert!(cancelled.cancelled);
    assert_eq!(cancelled.completed, 0);
    assert_eq!(
        nodes[1].wave(3, WaveCommand::Cancel(next.id)).unwrap(),
        cancelled
    );
    std::thread::sleep(Duration::from_secs(1));
    let latest = nodes[2].refreshed_status();
    assert!(latest
        .wave
        .is_some_and(|wave| wave.cancelled && wave.id == next.id));
    assert_eq!(latest.promotions[&first.0].new_owner, wave.tasks[0].target);
    assert_eq!(
        resp(nodes[2].address, &[b"GET", &first.1]).unwrap(),
        b"$11\r\ncancel-safe\r\n"
    );
    assert_eq!(
        resp(nodes[3].address, &[b"GET", &first.1]).unwrap(),
        UNAVAILABLE
    );
    println!("W10f primary wave={} first={} second={} cut_epochs=[{},{}] acknowledged={acknowledgements:?} cancelled_wave={} old_owner_fenced=true",
        wave.id.value(), first.0.value(), second.0.value(), first_cut.new_epoch.value(), second_cut.new_epoch.value(), next.id.value());
}

#[test]
fn w10f_balances_three_live_partitions_and_repairs_a_failed_backup_automatically() {
    if std::env::var(RUN).as_deref() != Ok("1") {
        return;
    }
    let mut nodes = start_node_count(true, 4);
    let initial = wait_for("four W10f members", || all_members(&nodes, 0));
    let members = fixture_members(&nodes, &initial);
    let mut keys = BTreeMap::new();
    let mut failure_case = None;
    for index in 0..5_000 {
        let key = format!("rebalance-key-{index}").into_bytes();
        let partition = scope().locate(key.clone()).unwrap().0;
        let assignment = RendezvousClusterOwnership
            .replicas_for_key(&format!("partition-{}", partition.value()), &members, 2)
            .unwrap();
        if assignment.primary == members[0].node_id
            && assignment.backups == [members[1].node_id.clone()]
        {
            if keys.len() < 3 {
                keys.entry(partition).or_insert(key);
            }
        } else if assignment.primary == members[1].node_id
            && assignment.backups == [members[0].node_id.clone()]
        {
            failure_case = Some((partition, key));
        }
        if keys.len() == 3 && failure_case.is_some() {
            break;
        }
    }
    assert_eq!(keys.len(), 3);
    let (failure_partition, failure_key) = failure_case.unwrap();
    for key in keys.values() {
        assert_eq!(
            resp(nodes[0].address, &[b"SET", key, b"base"]).unwrap(),
            b"+OK\r\n"
        );
    }
    assert_eq!(
        resp(nodes[1].address, &[b"SET", &failure_key, b"base"]).unwrap(),
        b"+OK\r\n"
    );
    let old_backup = nodes[1].member(initial[1].generation);
    let promotion = nodes[0]
        .promote_key(1, Some(failure_partition), vec![old_backup.clone()])
        .unwrap();
    wait_for("members see the fixture promotion", || {
        all_members(&nodes, promotion.old_epoch.value())
    });
    let rules: Vec<_> = nodes[2..]
        .iter()
        .map(|target| {
            json!({
                "from": nodes[0].identity, "to": target.identity,
                "action": {"kind": "value_chunk_delay", "index": 0, "millis": 650}
            })
        })
        .collect();
    publish_json(
        &nodes[0].root.join("fault.json"),
        &json!({
            "schema_version": 1, "generation": 1, "rules": rules
        }),
    );

    let (report, accepted) = with_rebalance_load(nodes[0].address, &keys, || {
        publish_json(
            &nodes[0].root.join("rebalance.json"),
            &RebalanceCommand {
                partitions: keys.keys().copied().collect(),
                drained: BTreeSet::from([old_backup.node.clone()]),
            },
        );
        let reply: Result<ReplicaRebalanceReport, String> =
            wait_for("three planned replica transfers", || {
                read_json(&nodes[0].root.join("rebalance-reply.json"))
            });
        reply.unwrap()
    });
    assert!(report.deferred.is_empty(), "{report:?}");
    assert_eq!(report.committed.len(), 3, "{report:?}");
    assert_eq!(accepted.len(), 3);
    let targets: BTreeSet<_> = report
        .committed
        .iter()
        .map(|repair| repair.plan.target.node.clone())
        .collect();
    assert_eq!(
        targets,
        BTreeSet::from([members[2].node_id.clone(), members[3].node_id.clone()])
    );
    let mut epoch = promotion.new_epoch;
    for repair in &report.committed {
        assert_eq!(repair.plan.replaced, old_backup);
        assert_eq!(repair.plan.old_epoch, epoch);
        assert!(repair.plan.prior_backups.is_some());
        assert!(
            repair.deltas > 0,
            "missing foreground deltas for {repair:?}"
        );
        epoch = repair.plan.new_epoch;
    }

    nodes[1].stop();
    assert!(nodes[1].read().is_err());
    let key = keys.values().next().unwrap();
    wait_for(
        "the surviving voters can still serve the rebalanced keys",
        || {
            resp(nodes[0].address, &[b"GET", key])
                .ok()
                .filter(|reply| reply == b"$8\r\nworkload\r\n")
        },
    );
    let (automatic, failure_writes) = with_rebalance_load(nodes[0].address, &keys, || {
        publish_json(&nodes[0].root.join("automatic-rebalance.json"), &true);
        wait_for("automatic repair after the backup stops", || {
            nodes[0].status()?.repairs.get(&failure_partition).cloned()
        })
    });
    assert_eq!(failure_writes.len(), 3);
    assert_eq!(automatic.replaced, old_backup);
    assert!(targets.contains(&automatic.target.node));
    let epoch = automatic.new_epoch.value();
    for (partition, key) in keys
        .iter()
        .chain(std::iter::once((&failure_partition, &failure_key)))
    {
        let previous = if *partition == failure_partition {
            b"base".as_slice()
        } else {
            b"workload"
        };
        let cas = compare_and_set_for_key(key, previous, b"verified");
        let id = format!("w10e:{epoch}:verified-{}", partition.value());
        let res = wait_for("writes with only replacement backups", || {
            match nodes[0].hc1(cas.clone(), &id) {
                Ok(response) => Some(response),
                Err(error) if error.retryable => None,
                Err(error) => panic!("post-rebalance CAS failed: {error:?}"),
            }
        });
        assert!(matches!(res, ClientResponse::CasApplied { .. }), "{res:?}");
        assert_eq!(nodes[0].hc1(cas, &id).unwrap(), res);
        assert_eq!(
            resp(nodes[0].address, &[b"GET", key]).unwrap(),
            b"$8\r\nverified\r\n"
        );
    }
    println!("W10f multi initial_partitions=3 targets={} accepted={accepted:?} deltas={:?} automatic_partition={} writes_during_automatic_repair={failure_writes:?} stopped_backup_pid={} verified_partitions=4",
        targets.len(), report.committed.iter().map(|repair| repair.deltas).collect::<Vec<_>>(),
        failure_partition.value(), initial[1].pid);
}

#[test]
fn w10f_resumes_snapshot_and_restores_writes_with_old_owner_stopped() {
    if std::env::var(RUN).as_deref() != Ok("1") {
        return;
    }
    let mut nodes = start_nodes(true);
    let initial = wait_for("three W10f members", || all_members(&nodes, 0));
    let old = expect_owner(&nodes, b"$-1\r\n");
    let owner = nodes
        .iter()
        .position(|node| Some(&node.identity) == initial[old].backup.as_ref())
        .unwrap();
    let target = (0..nodes.len())
        .find(|index| *index != old && *index != owner)
        .unwrap();
    let old_identity = nodes[old].member(initial[old].generation);
    let next_identity = nodes[target].member(initial[target].generation);
    assert_eq!(
        resp(nodes[old].address, &[b"SET", KEY, b"seed"]).unwrap(),
        b"+OK\r\n"
    );
    let large = vec![0xaa; 20 * 1024];
    let retained = compare_and_set(b"seed", &large);
    assert_eq!(
        nodes[old].hc1(retained.clone(), "w10f-cert").unwrap(),
        ClientResponse::CasApplied { new_version: 2 }
    );
    wait_for("durable initial certified snapshot", || {
        let certificate = nodes[owner].status()?.certificate?;
        (certificate.version == 2).then_some(certificate)
    });

    nodes[old].stop();
    assert!(nodes[old].read().is_err());
    let source = &nodes[owner];
    let receiver = &nodes[target];
    let mut serial = 0;
    let promotion = wait_for("backup takes over while the old process is stopped", || {
        serial += 1;
        source.promote(serial, vec![old_identity.clone()])
    });
    let cut = promotion.new_epoch.value();
    wait_for("spare applied the owner promotion", || {
        (receiver.status()?.epoch == cut).then_some(())
    });

    publish_json(
        &source.root.join("fault.json"),
        &json!({
            "schema_version": 1, "generation": 1, "rules": [{
                "from": source.identity, "to": receiver.identity, "action": {"kind": "value_chunk_drop", "index": 1}
            }]
        }),
    );
    let interrupted = source.repair(1, old_identity.clone(), next_identity.clone());
    assert!(interrupted.is_err(), "{interrupted:?}");
    assert!(fs::read_to_string(source.root.join("child.log"))
        .unwrap()
        .contains("ValueChunkDrop { index: 1 }"));
    let partial = source.refreshed_status();
    assert_eq!(partial.epoch, cut);
    assert_eq!(
        partial.backup.as_deref(),
        Some(nodes[old].identity.as_str())
    );
    let mutation = compare_and_set(&large, VALUE);
    assert!(source
        .hc1(mutation.clone(), &format!("w10e:{cut}:before-repair"))
        .is_err());
    assert_eq!(source.refreshed_status().owner_applies, 0);
    assert_eq!(receiver.read().unwrap(), UNAVAILABLE);
    let mut expected = format!("${}\r\n", large.len()).into_bytes();
    expected.extend_from_slice(&large);
    expected.extend_from_slice(b"\r\n");
    assert_eq!(source.read().unwrap(), expected);

    fault_generation(&nodes, None, 2);
    let repair = source
        .repair(2, old_identity.clone(), next_identity.clone())
        .unwrap();
    assert_eq!(repair.resumed_from, 1);
    assert_eq!(repair.deltas, 0);
    assert!(repair.chunks >= 2);
    assert_eq!(repair.plan.owner, promotion.new_owner);
    assert_eq!(repair.plan.target, next_identity);
    assert_eq!(repair.plan.required, promotion.required);
    assert_eq!(repair.plan.old_epoch, promotion.new_epoch);
    let epoch = repair.plan.new_epoch.value();
    let repeated = source.repair(3, old_identity, next_identity).unwrap();
    assert_eq!(repeated.plan, repair.plan);
    assert_eq!(repeated.chunks, 0);
    wait_for("replacement applied the repair decision", || {
        (receiver.status()?.epoch == epoch).then_some(())
    });
    assert_eq!(
        source.refreshed_status().backup.as_deref(),
        Some(receiver.identity.as_str())
    );
    let id = format!("w10e:{epoch}:after-repair");
    assert_eq!(
        source.hc1(mutation.clone(), &id).unwrap(),
        ClientResponse::CasApplied { new_version: 3 }
    );
    assert_eq!(
        source.hc1(mutation, &id).unwrap(),
        ClientResponse::CasApplied { new_version: 3 }
    );
    assert_eq!(source.refreshed_status().owner_applies, 1);
    let final_cert = wait_for("spare applied and certified version 3", || {
        let status = receiver.status()?;
        (status.version == 3 && status.applies == 1).then_some(status.certificate?)
    });

    let promoted_replacement = receiver.promote(1, vec![promotion.new_owner]).unwrap();
    assert_eq!(promoted_replacement.required, Some(final_cert));
    assert_eq!(receiver.read().unwrap(), expected_value());
    assert_eq!(
        receiver.hc1(retained, "w10f-cert").unwrap(),
        ClientResponse::CasApplied { new_version: 2 }
    );
    assert!(nodes[old].read().is_err());
    println!("W10f old_pid={} owner_pid={} repaired_replica_pid={} repaired_epoch={epoch} resumed_from={} chunks={} snapshot_digest={:02x?} final_prefix={} final_digest={:02x?} old_process_stopped=true",
        initial[old].pid, initial[owner].pid, initial[target].pid, repair.resumed_from,
        repair.chunks, repair.plan.snapshot_digest, final_cert.version, final_cert.digest);
}

#[test]
fn w10f_streams_versioned_changes_while_owner_keeps_accepting_writes() {
    if std::env::var(RUN).as_deref() != Ok("1") {
        return;
    }
    let mut nodes = start_nodes(true);
    let initial = wait_for("three members before live repair", || {
        all_members(&nodes, 0)
    });
    let old = expect_owner(&nodes, b"$-1\r\n");
    let owner = nodes
        .iter()
        .position(|node| Some(&node.identity) == initial[old].backup.as_ref())
        .unwrap();
    let spare = (0..nodes.len())
        .find(|index| *index != old && *index != owner)
        .unwrap();
    let replaced = nodes[old].member(initial[old].generation);
    let target = nodes[spare].member(initial[spare].generation);
    let source = &nodes[owner];
    let receiver = &nodes[spare];
    assert_eq!(
        resp(nodes[old].address, &[b"SET", KEY, b"before"]).unwrap(),
        b"+OK\r\n"
    );
    let mut value = vec![0xaa; 20 * 1024];
    assert_eq!(
        nodes[old]
            .hc1(compare_and_set(b"before", &value), "live-seed")
            .unwrap(),
        ClientResponse::CasApplied { new_version: 2 }
    );
    wait_for("source inherited the seed certificate", || {
        (source.status()?.version == 2).then_some(())
    });
    let promotion = source.promote(1, vec![replaced.clone()]).unwrap();
    let old_epoch = promotion.new_epoch.value();
    wait_for("all members see the promoted placement", || {
        all_members(&nodes, old_epoch - 1)
    });

    publish_json(
        &source.root.join("fault.json"),
        &json!({
            "schema_version": 1, "generation": 1, "rules": [{
                "from": source.identity, "to": receiver.identity,
                "action": {"kind": "value_chunk_drop", "index": 1}
            }]
        }),
    );
    assert!(source.repair(1, replaced.clone(), target.clone()).is_err());
    assert!(fs::read_to_string(source.root.join("child.log"))
        .unwrap()
        .contains("ValueChunkDrop { index: 1 }"));
    assert_eq!(source.refreshed_status().epoch, old_epoch);

    let mut completed = Vec::new();
    for version in 3..=17 {
        if version == 6 {
            publish_json(
                &source.root.join("fault.json"),
                &json!({
                    "schema_version": 1, "generation": 2, "rules": [{
                        "from": source.identity, "to": receiver.identity,
                        "action": {"kind": "value_delta_delay", "version": 3, "millis": 1000}
                    }]
                }),
            );
            source.start_repair(2, replaced.clone(), target.clone());
            wait_for(
                "first ordered delta was sent while the old backup is active",
                || {
                    fs::read_to_string(source.root.join("child.log"))
                        .ok()?
                        .contains("ValueDeltaDelay { version: 3, millis: 1000 }")
                        .then_some(())
                },
            );
            assert_eq!(receiver.read().unwrap(), UNAVAILABLE);
        }
        let next = format!("during-repair-{version}").into_bytes();
        let request = compare_and_set(&value, &next);
        let id = format!("w10e:{old_epoch}:live-{version}");
        assert_eq!(
            source.hc1(request.clone(), &id).unwrap(),
            ClientResponse::CasApplied {
                new_version: version
            }
        );
        completed.push((request, id, version));
        value = next;
    }
    let repaired = source.repair_result(2).unwrap();
    let new_epoch = repaired.plan.new_epoch.value();
    assert_eq!(repaired.resumed_from, 1);
    assert_eq!(repaired.deltas, 15);
    assert_eq!(repaired.plan.required.unwrap().version, 17);
    assert_eq!(repaired.plan.target, target);
    assert_eq!(repaired.plan.old_epoch.value(), old_epoch);
    wait_for(
        "replacement applies the final version and activation",
        || {
            let status = receiver.status()?;
            (status.epoch == new_epoch && status.certificate == repaired.plan.required)
                .then_some(())
        },
    );
    nodes[old].stop();
    assert!(nodes[old].read().is_err());
    let after = compare_and_set(&value, VALUE);
    let id = format!("w10e:{new_epoch}:after-live-repair");
    let expected = [format!("${}\r\n", value.len()).as_bytes(), &value, b"\r\n"].concat();
    wait_for(
        "remaining voters elect a leader after the former backup stops",
        || nodes[owner].read().ok().filter(|reply| *reply == expected),
    );
    let written = wait_for("a write through the replacement backup", || {
        match nodes[owner].hc1(after.clone(), &id) {
            Ok(response) => Some(response),
            Err(error) if error.retryable => None,
            Err(error) => panic!("post-cutover CAS failed: {error:?}"),
        }
    });
    assert_eq!(written, ClientResponse::CasApplied { new_version: 18 });
    assert_eq!(
        nodes[owner].hc1(after, &id).unwrap(),
        ClientResponse::CasApplied { new_version: 18 }
    );
    let final_prefix = wait_for("new backup received a write after cutover", || {
        let status = nodes[spare].status()?;
        (status.version == 18 && status.applies == 1).then_some(status.certificate?)
    });
    let on_replacement = nodes[spare].promote(1, vec![promotion.new_owner]).unwrap();
    assert_eq!(on_replacement.required, Some(final_prefix));
    assert_eq!(nodes[spare].read().unwrap(), expected_value());
    for (request, id, version) in completed {
        assert_eq!(
            nodes[spare].hc1(request, &id).unwrap(),
            ClientResponse::CasApplied {
                new_version: version
            }
        );
    }
    assert_eq!(nodes[spare].refreshed_status().owner_applies, 0);
    println!("W10f live owner_pid={} prior_backup_pid={} replacement_pid={} resumed_from=1 snapshot=2 final_cut=17 deltas=15 live_acks_during_delta_send=12 post_cut=18 replayed=15 final_digest={:02x?}",
        initial[owner].pid, initial[old].pid, initial[spare].pid, final_prefix.digest);
}

#[test]
fn w10e_fences_old_owner_and_replays_only_certified_retry_across_cutover() {
    if std::env::var(RUN).as_deref() != Ok("1") {
        return;
    }
    let mut nodes = start_nodes(true);
    let initial = wait_for("three W10e members", || all_members(&nodes, 0));
    let owner = expect_owner(&nodes, b"$-1\r\n");
    let backup = nodes
        .iter()
        .position(|node| Some(&node.identity) == initial[owner].backup.as_ref())
        .unwrap();
    let spare = (0..nodes.len())
        .find(|index| *index != owner && *index != backup)
        .unwrap();
    let old = &nodes[owner];
    let successor = &nodes[backup];
    let old_identity = old.member(initial[owner].generation);
    let fallback = vec![old_identity.clone()];
    let seed = b"before-cutover";
    assert_eq!(resp(old.address, &[b"SET", KEY, seed]).unwrap(), b"+OK\r\n");
    let retained = compare_and_set(seed, VALUE);
    let unread = old.send_hc1(retained.clone(), "certified-with-lost-reply");
    wait_for(
        "certification despite the client leaving the reply unread",
        || (old.status()?.version == 2 && successor.status()?.version == 2).then_some(()),
    );
    drop(unread);
    assert!(nodes[spare].promote(1, fallback.clone()).is_none());
    let wrong_generation = old.member(initial[owner].generation + 1);
    assert!(successor.promote(1, vec![wrong_generation]).is_none());

    publish_json(
        &old.root.join("fault.json"),
        &json!({
            "schema_version": 1, "generation": 1, "rules": [{
                "from": old.identity, "to": successor.identity, "action": {"kind": "value_drop"}
            }]
        }),
    );
    let unknown = compare_and_set(VALUE, b"uncertified");
    let failed = old
        .hc1(unknown.clone(), "uncertified-at-old-owner")
        .unwrap_err();
    assert_eq!(failed.code, ClientErrorCode::BackendUnavailable);
    assert!(!failed.retryable);
    let failed_old = old.refreshed_status();
    assert_eq!(failed_old.version, 2);
    assert_eq!(failed_old.owner_applies, 3);
    assert_eq!(successor.refreshed_status().applies, 2);

    let cut_started = Instant::now();
    fault_generation(&nodes, Some(owner), 2);
    let mut serial = 2;
    let activation = wait_for("durable backup promotion on the majority side", || {
        serial += 1;
        successor.promote(serial, fallback.clone())
    });
    assert_eq!(activation.old_owner, old_identity);
    assert_eq!(
        activation.new_owner,
        successor.member(initial[backup].generation)
    );
    assert_eq!(activation.required.unwrap().version, 2);
    let epoch = activation.new_epoch.value();
    assert!(epoch > initial[owner].epoch);
    assert_eq!(
        successor.promote(serial + 1, fallback),
        Some(activation.clone())
    );
    assert_eq!(successor.read().unwrap(), expected_value());
    assert!(matches!(
        old.read().unwrap().as_slice(),
        UNAVAILABLE | DEADLINE
    ));
    assert_eq!(old.refreshed_status().lookups, failed_old.lookups);

    assert_eq!(
        successor
            .hc1(retained, "certified-with-lost-reply")
            .unwrap(),
        ClientResponse::CasApplied { new_version: 2 }
    );
    assert_eq!(
        successor
            .hc1(unknown.clone(), "certified-with-lost-reply")
            .unwrap_err()
            .code,
        ClientErrorCode::Conflict
    );
    for id in [
        "uncertified-at-old-owner".to_owned(),
        format!("w10e:{}:old", activation.old_epoch.value()),
    ] {
        let error = successor.hc1(unknown.clone(), &id).unwrap_err();
        assert_eq!(error.code, ClientErrorCode::BackendUnavailable);
        assert!(!error.retryable);
        assert!(error.message.contains("outcome_unknown"), "{error:?}");
    }
    let fresh_id = format!("w10e:{epoch}:fresh");
    let fresh = compare_and_set(VALUE, b"after-cutover");
    assert!(successor.hc1(fresh.clone(), &fresh_id).is_err());
    let cut = successor.refreshed_status();
    assert_eq!((cut.version, cut.owner_applies), (2, 0));
    assert_eq!(successor.read().unwrap(), expected_value());
    assert!(old
        .hc1(unknown.clone(), "uncertified-at-old-owner")
        .is_err());
    assert_eq!(old.refreshed_status().owner_applies, 3);

    fault_generation(&nodes, None, 3);
    wait_for("former owner applied its fence", || {
        (old.status()?.epoch == epoch).then_some(())
    });
    assert_eq!(old.read().unwrap(), UNAVAILABLE);
    assert!(old.hc1(unknown, "uncertified-at-old-owner").is_err());
    assert_eq!(old.refreshed_status().owner_applies, 3);
    let result = wait_for("synchronous prefix is available after healing", || {
        successor.hc1(fresh.clone(), &fresh_id).ok()
    });
    let cut_ms = cut_started.elapsed().as_millis();
    assert_eq!(result, ClientResponse::CasApplied { new_version: 3 });
    assert_eq!(successor.hc1(fresh, &fresh_id).unwrap(), result);
    assert_eq!(successor.read().unwrap(), b"$13\r\nafter-cutover\r\n");
    let healed = successor.refreshed_status();
    assert_eq!((healed.version, healed.owner_applies), (3, 1));
    let digest = activation.required.unwrap().digest;
    let final_digest = healed.certificate.unwrap().digest;
    nodes[owner].stop();
    nodes[owner].start();
    let restarted = wait_for("former owner rejoined with a different incarnation", || {
        all_members(&nodes, epoch)
            .filter(|status| status[owner].generation > initial[owner].generation)
    });
    assert_ne!(restarted[owner].pid, initial[owner].pid);
    assert_eq!(nodes[owner].read().unwrap(), UNAVAILABLE);
    assert_eq!(nodes[backup].read().unwrap(), b"$13\r\nafter-cutover\r\n");
    let restarted_id = format!("w10e:{}:after-restart", restarted[owner].epoch);
    assert!(nodes[backup]
        .hc1(
            compare_and_set(b"after-cutover", b"stale-backup"),
            &restarted_id
        )
        .is_err());
    let after_restart = nodes[backup].refreshed_status();
    assert_eq!((after_restart.version, after_restart.owner_applies), (3, 1));
    assert_eq!(after_restart.certificate, healed.certificate);
    println!("W10e old_pid={} restarted_pid={} successor_pid={} spare_pid={} epoch={epoch} required_prefix=2 required_digest={digest:02x?} final_prefix=3 final_digest={final_digest:02x?} isolated_old_lookups=0 duplicate_applies=0 observed_partition_write_unavailable_ms={cut_ms}",
        initial[owner].pid, restarted[owner].pid, initial[backup].pid, initial[spare].pid);
}

#[test]
fn isolated_and_restarted_owner_needs_fresh_read_authority_and_replica_prefix() {
    if std::env::var(RUN).as_deref() != Ok("1") {
        return;
    }
    let mut nodes = start_nodes(false);
    let initial = wait_for("three admitted member processes", || all_members(&nodes, 0));
    let epoch = initial[0].epoch;
    for node in &nodes {
        node.publish_replica(1, epoch, 8, epoch);
    }
    let value = expected_value();
    let owner = expect_owner(&nodes, &value);
    println!(
        "D2 owner={} pid={} epoch={epoch}",
        nodes[owner].identity, initial[owner].pid
    );
    let before = wait_for("the successful lookup to be observed", || {
        nodes[owner].status().filter(|s| s.lookups > 0)
    })
    .lookups;

    fault_generation(&nodes, Some(owner), 1);
    wait_for("a real Raft transport fault hit", || {
        let log = fs::read_to_string(nodes[owner].root.join("child.log")).ok()?;
        log.contains("HYDRACACHE_TEST_RAFT_OUTBOUND_FAULT_HIT generation=1")
            .then_some(())
    });
    let refused = nodes[owner].read().unwrap();
    assert!(refused == DEADLINE || refused == UNAVAILABLE);
    let during = nodes[owner].refreshed_status();
    assert_eq!(
        (during.pid, during.lookups, during.receipt),
        (initial[owner].pid, before, 1)
    );
    println!(
        "D2 isolation response={} backing_lookups_delta=0",
        String::from_utf8_lossy(&refused).trim()
    );
    fault_generation(&nodes, None, 2);
    assert_eq!(expect_owner(&nodes, &value), owner);

    nodes[owner].stop();
    assert!(nodes[owner].read().is_err());
    nodes[owner].start();
    let rejoined = wait_for("owner restart with a new generation", || {
        all_members(&nodes, epoch).filter(|statuses| {
            statuses[owner].generation > initial[owner].generation && statuses[owner].receipt == 1
        })
    });
    let new_epoch = rejoined[owner].epoch;
    assert_ne!(rejoined[owner].pid, initial[owner].pid);
    let stale = nodes[owner].read().unwrap();
    assert_eq!(stale, UNAVAILABLE);
    assert_eq!(nodes[owner].refreshed_status().lookups, 0);

    for node in &nodes {
        node.publish_replica(2, new_epoch, 7, epoch);
    }
    wait_for("all incomplete replica receipts", || {
        nodes
            .iter()
            .map(Node::status)
            .collect::<Option<Vec<_>>>()
            .filter(|statuses| statuses.iter().all(|status| status.receipt == 2))
    });
    for node in &nodes {
        let before = node.status().unwrap().lookups;
        assert_eq!(node.read().unwrap(), UNAVAILABLE);
        assert_eq!(node.refreshed_status().lookups, before);
    }
    for node in &nodes {
        node.publish_replica(3, new_epoch, 8, epoch);
    }
    let recovered = expect_owner(&nodes, &value);
    println!("D2 restart old_pid={} new_pid={} epoch={new_epoch} stale_lookups=0 lagging_lookups=0 recovered_owner={}",
        initial[owner].pid, rejoined[owner].pid, nodes[recovered].identity);
}

#[test]
fn live_write_requires_backup_and_recovery_never_reads_uncertified_value() {
    if std::env::var(RUN).as_deref() != Ok("1") {
        return;
    }
    let mut nodes = start_nodes(true);
    let initial = wait_for("three W10c member processes", || all_members(&nodes, 0));
    let owner = expect_owner(&nodes, b"$-1\r\n");
    let backup_id = initial[owner].backup.as_ref().unwrap();
    let backup = nodes
        .iter()
        .position(|node| &node.identity == backup_id)
        .unwrap();
    assert_eq!(
        resp(nodes[owner].address, &[b"SET", KEY, VALUE]).unwrap(),
        b"+OK\r\n"
    );
    wait_for("first Raft prefix and exact replica apply", || {
        (nodes[owner].status()?.version == 1 && nodes[backup].status()?.applies == 1).then_some(())
    });
    assert_eq!(nodes[owner].read().unwrap(), expected_value());

    publish_json(
        &nodes[owner].root.join("fault.json"),
        &json!({
            "schema_version": 1, "generation": 1, "rules": [{
                "from": nodes[owner].identity, "to": nodes[backup].identity, "action": {"kind": "value_drop"}
            }]
        }),
    );
    let newer = b"after-backup";
    assert_eq!(
        resp(nodes[owner].address, &[b"SET", KEY, newer]).unwrap(),
        UNAVAILABLE
    );
    let log = fs::read_to_string(nodes[owner].root.join("child.log")).unwrap();
    assert!(log.contains("action=ValueDrop"));
    assert_eq!(nodes[owner].refreshed_status().version, 1);
    assert_eq!(nodes[backup].refreshed_status().applies, 1);
    assert_eq!(nodes[owner].read().unwrap(), expected_value());
    println!("W10c missed replica application: SET refused, GET kept prefix=1");

    fault_generation(&nodes, None, 2);
    nodes[owner].stop();
    nodes[owner].start();
    let recovered = wait_for(
        "same owner with a new generation and retained certificate",
        || {
            all_members(&nodes, initial[owner].epoch).filter(|statuses| {
                statuses[owner].version == 1
                    && statuses[owner].generation > initial[owner].generation
            })
        },
    );
    assert_ne!(recovered[owner].pid, initial[owner].pid);
    assert_eq!(expect_owner(&nodes, &expected_value()), owner);
    assert_eq!(
        resp(nodes[owner].address, &[b"SET", KEY, newer]).unwrap(),
        b"+OK\r\n"
    );
    assert_eq!(nodes[owner].read().unwrap(), b"$12\r\nafter-backup\r\n");
    wait_for("second replica catch-up", || {
        (nodes[backup].status()?.version == 2).then_some(())
    });
    assert_eq!(
        resp(nodes[owner].address, &[b"HC.INVALIDATE", KEY]).unwrap(),
        b"+OK\r\n"
    );
    assert_eq!(nodes[owner].read().unwrap(), b"$-1\r\n");
    assert_eq!(nodes[owner].refreshed_status().version, 3);
    println!(
        "W10c recovered old pid={} new pid={} epoch={} SET prefix=2 tombstone prefix=3",
        initial[owner].pid, recovered[owner].pid, recovered[owner].epoch
    );
}

fn clock(node: &Node, now_ms: u64, uncertainty_ms: u64) {
    publish_json(
        &node.root.join("clock.json"),
        &json!({"now_ms":now_ms,"uncertainty_ms":uncertainty_ms}),
    );
}

#[test]
fn distributed_ttl_withholds_expiry_during_replica_cut_and_fences_restarted_owner() {
    if std::env::var(RUN).as_deref() != Ok("1") {
        return;
    }
    let mut nodes = start_nodes(true);
    let initial = wait_for("three W10d members", || all_members(&nodes, 0));
    let owner = expect_owner(&nodes, b"$-1\r\n");
    let backup = nodes
        .iter()
        .position(|node| Some(&node.identity) == initial[owner].backup.as_ref())
        .unwrap();
    clock(&nodes[owner], 1100, 5);
    for (index, node) in nodes.iter().enumerate() {
        if index != owner {
            clock(node, 9000, 1);
        }
    }
    let set = |node: &Node| resp(node.address, &[b"SET", KEY, VALUE, b"NX", b"PX", b"30"]).unwrap();
    assert_eq!(set(&nodes[owner]), b"+OK\r\n");
    assert_eq!(set(&nodes[owner]), b"$-1\r\n");
    assert_eq!(
        resp(nodes[owner].address, &[b"PTTL", KEY]).unwrap(),
        b":30\r\n"
    );
    assert_eq!(nodes[owner].read().unwrap(), expected_value());
    clock(&nodes[owner], 1134, 3);
    assert_eq!(nodes[owner].read().unwrap(), UNAVAILABLE);
    assert_eq!(nodes[owner].refreshed_status().version, 1);

    publish_json(
        &nodes[owner].root.join("fault.json"),
        &json!({
            "schema_version": 1, "generation": 1, "rules": [{
                "from": nodes[owner].identity, "to": nodes[backup].identity, "action": {"kind": "value_drop"}
            }]
        }),
    );
    clock(&nodes[owner], 1140, 3);
    wait_for("replica interruption on expiry tombstone", || {
        fs::read_to_string(nodes[owner].root.join("child.log"))
            .ok()?
            .contains("action=ValueDrop")
            .then_some(())
    });
    assert_eq!(nodes[owner].read().unwrap(), UNAVAILABLE);
    assert_eq!(nodes[owner].refreshed_status().version, 1);
    assert_eq!(nodes[backup].refreshed_status().applies, 1);
    println!("W10d deadline=1135 overlap=unavailable expiry replica cut prefix=1");

    nodes[owner].stop();
    clock(&nodes[owner], 1140, 11);
    fault_generation(&nodes, None, 2);
    nodes[owner].start();
    let rejoined = wait_for("restarted timed owner with retained prefix", || {
        all_members(&nodes, initial[owner].epoch).filter(|statuses| {
            statuses[owner].generation > initial[owner].generation && statuses[owner].version == 1
        })
    });
    assert_ne!(initial[owner].pid, rejoined[owner].pid);
    assert_eq!(nodes[owner].read().unwrap(), UNAVAILABLE);
    clock(&nodes[owner], 1140, 3);
    wait_for("certified expiry after restoration", || {
        (nodes[owner].status()?.version == 2).then_some(())
    });
    // Even a deliberately rolled-back test clock cannot resurrect a certified tombstone.
    clock(&nodes[owner], 1000, 0);
    assert_eq!(nodes[owner].read().unwrap(), b"$-1\r\n");
    assert_eq!(
        resp(nodes[owner].address, &[b"PTTL", KEY]).unwrap(),
        b":-2\r\n"
    );
    println!(
        "W10d restarted pid={} epoch={} certified expiry prefix=2, GET nil after clock rewind",
        rejoined[owner].pid, rejoined[owner].epoch
    );
}
