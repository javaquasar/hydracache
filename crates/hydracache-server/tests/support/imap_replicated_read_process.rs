use super::*;

use std::fs::{self, File};
use std::net::TcpListener as PortReservation;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Instant, SystemTime};

use hydracache::{
    ClusterGeneration, ClusterNodeId, ClusterTimeAuthority, DurableValueStore, PartitionPromotion,
    PartitionValuePrefix, TimeObservation,
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
use hydracache_server::{MemberReadHost, ServerConfig, ServerRole};
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
}

#[derive(Serialize, Deserialize)]
struct PromotionRequest {
    serial: u32,
    backups: Vec<PartitionMember>,
}

#[derive(Serialize, Deserialize)]
struct PromotionResponse {
    serial: u32,
    plan: Option<PartitionPromotion>,
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
    loop {
        if let Some(request) = read_json::<PromotionRequest>(&root.join("promotion.json")) {
            if request.serial > promotion_serial {
                let plan = host
                    .promote_partition(partition, request.backups)
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
        publish_json(
            &self.root.join("promotion.json"),
            &PromotionRequest { serial, backups },
        );
        wait_for("promotion control reply", || {
            read_json::<PromotionResponse>(&self.root.join("promotion-reply.json"))
                .filter(|reply| reply.serial == serial)
        })
        .plan
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
            .to_message()
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
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::var_os("HYDRACACHE_IMAP_REPLICA_TEST_ARTIFACTS")
        .map_or_else(std::env::temp_dir, PathBuf::from)
        .join(format!("imap-replica-{}-{timestamp}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let reservations: Vec<_> = (0..9)
        .map(|_| PortReservation::bind("127.0.0.1:0").unwrap())
        .collect();
    let addresses: Vec<_> = reservations
        .iter()
        .map(|socket| socket.local_addr().unwrap())
        .collect();
    let seeds: Vec<_> = addresses[..3].iter().map(ToString::to_string).collect();
    let mut nodes: Vec<_> = addresses[..3]
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
                listen_addr: addresses[index + 3],
                seeds: seeds.clone(),
                storage_dir: Some(node_root.clone()),
                ..ServerConfig::default()
            };
            publish_json(&node_root.join("config.json"), &config);
            let hc1_address = addresses[index + 6];
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
    let encoded = KEY
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
