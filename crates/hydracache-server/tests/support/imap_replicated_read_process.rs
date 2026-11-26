use super::*;

use std::fs::{self, File};
use std::net::TcpListener as PortReservation;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Instant, SystemTime};

use hydracache::DurableValueStore;
use hydracache_server::{MemberReadHost, ServerConfig, ServerRole};
use serde::{Deserialize, Serialize};
use serde_json::json;

const RUN: &str = "HYDRACACHE_RUN_DAEMON_PROCESS_E2E";
const CHILD: &str = "HYDRACACHE_IMAP_REPLICA_TEST_CHILD";
const FAULT: &str = "HYDRACACHE_TEST_RAFT_OUTBOUND_FAULT_FILE";

#[derive(Debug, Serialize, Deserialize)]
struct Status {
    pid: u32,
    sample: u64,
    members: usize,
    epoch: u64,
    generation: u64,
    receipt: u32,
    lookups: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct Receipt {
    serial: u32,
    epoch: u64,
    applied: u64,
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
                },
            );
        }
        assert!(!task.is_finished());
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

struct Node {
    root: PathBuf,
    address: SocketAddr,
    identity: String,
    child: Option<Child>,
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

#[test]
fn isolated_and_restarted_owner_needs_fresh_read_authority_and_replica_prefix() {
    if std::env::var(RUN).as_deref() != Ok("1") {
        return;
    }
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::var_os("HYDRACACHE_IMAP_REPLICA_TEST_ARTIFACTS")
        .map_or_else(std::env::temp_dir, PathBuf::from)
        .join(format!("imap-replica-{}-{timestamp}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let reservations: Vec<_> = (0..6)
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
            Node {
                root: node_root,
                address: config.listen_addr,
                identity,
                child: None,
            }
        })
        .collect();
    fault_generation(&nodes, None, 0);
    drop(reservations);
    for node in &mut nodes {
        node.start();
    }
    println!("D2 process artifacts {}", root.display());
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
    assert!(refused == b"-ERR deadline exceeded\r\n" || refused == b"-ERR backend unavailable\r\n");
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
    assert_eq!(stale, b"-ERR backend unavailable\r\n");
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
        assert_eq!(node.read().unwrap(), b"-ERR backend unavailable\r\n");
        assert_eq!(node.refreshed_status().lookups, before);
    }
    for node in &nodes {
        node.publish_replica(3, new_epoch, 8, epoch);
    }
    let recovered = expect_owner(&nodes, &value);
    println!("D2 restart old_pid={} new_pid={} epoch={new_epoch} stale_lookups=0 lagging_lookups=0 recovered_owner={}",
        initial[owner].pid, rejoined[owner].pid, nodes[recovered].identity);
}
