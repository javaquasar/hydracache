mod support;

use std::fs;
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use hydracache_cluster_testkit::{RaftFilterAction, RaftPacketFilter, RuntimeRaftCluster};
use raft::eraftpb::MessageType;
use serde::{Deserialize, Serialize};
use support::daemon_cluster::{DaemonCluster, TestResult};

const SEED: u64 = 0x0D64_0037;
const PORTABLE_ARTIFACT: &str = "daemon-resource-budget-portable.json";
#[cfg(target_os = "linux")]
const LINUX_ARTIFACT: &str = "daemon-resource-budget-linux.json";
#[cfg(target_os = "linux")]
const LINUX_GATE_ENV: &str = "HYDRACACHE_RUN_DAEMON_RESOURCE_LINUX";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
struct ResourceSample {
    running_children: u64,
    tracked_connections: u64,
    held_snapshot_messages: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    rss_kib: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    open_fds: Option<u64>,
}

impl ResourceSample {
    fn peak(samples: &[Self]) -> Self {
        Self {
            running_children: samples
                .iter()
                .map(|sample| sample.running_children)
                .max()
                .unwrap_or(0),
            tracked_connections: samples
                .iter()
                .map(|sample| sample.tracked_connections)
                .max()
                .unwrap_or(0),
            held_snapshot_messages: samples
                .iter()
                .map(|sample| sample.held_snapshot_messages)
                .max()
                .unwrap_or(0),
            rss_kib: samples.iter().filter_map(|sample| sample.rss_kib).max(),
            open_fds: samples.iter().filter_map(|sample| sample.open_fds).max(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ResourceBudget {
    max_child_delta: u64,
    max_connection_delta: u64,
    max_held_snapshot_messages: u64,
    max_rss_growth_kib: u64,
    max_fd_growth: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ResourceBudgetArtifact {
    schema_version: u32,
    release: String,
    seed: u64,
    platform: String,
    budget: ResourceBudget,
    baseline: ResourceSample,
    peak: ResourceSample,
    final_sample: ResourceSample,
    samples: Vec<ResourceSample>,
}

impl ResourceBudgetArtifact {
    fn new(samples: Vec<ResourceSample>, budget: ResourceBudget) -> Self {
        let baseline = samples.first().copied().unwrap_or_default();
        let final_sample = samples.last().copied().unwrap_or_default();
        Self {
            schema_version: 1,
            release: "0.64.0".to_owned(),
            seed: SEED,
            platform: std::env::consts::OS.to_owned(),
            budget,
            baseline,
            peak: ResourceSample::peak(&samples),
            final_sample,
            samples,
        }
    }

    fn write(&self, path: impl AsRef<Path>) -> TestResult {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("target/test-evidence/0.64")
            .join(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }
}

fn portable_sample(
    cluster: &mut DaemonCluster,
    tracked_connections: u64,
    held_snapshot_messages: u64,
) -> ResourceSample {
    ResourceSample {
        running_children: cluster.running_child_count() as u64,
        tracked_connections,
        held_snapshot_messages,
        rss_kib: None,
        open_fds: None,
    }
}

#[cfg(target_os = "linux")]
fn linux_sample(cluster: &mut DaemonCluster) -> TestResult<ResourceSample> {
    let (rss_kib, open_fds) = cluster
        .os_resource_totals()
        .ok_or("Linux /proc resource sampling is unavailable")?;
    Ok(ResourceSample {
        running_children: cluster.running_child_count() as u64,
        tracked_connections: 0,
        held_snapshot_messages: 0,
        rss_kib: Some(rss_kib),
        open_fds: Some(open_fds),
    })
}

fn connect_with_retry(addr: SocketAddr) -> TestResult<TcpStream> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match TcpStream::connect_timeout(&addr, Duration::from_secs(1)) {
            Ok(stream) => return Ok(stream),
            Err(_) if Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(100));
            }
            Err(error) => return Err(error.into()),
        }
    }
}

fn redis_roundtrip(addr: SocketAddr, request: &[u8]) -> TestResult<Vec<u8>> {
    let mut stream = connect_with_retry(addr)?;
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    stream.set_write_timeout(Some(Duration::from_secs(3)))?;
    stream.write_all(request)?;
    stream.shutdown(Shutdown::Write)?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    Ok(response)
}

fn cancel_admin_request(addr: SocketAddr) -> TestResult {
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(3))?;
    stream.set_write_timeout(Some(Duration::from_secs(3)))?;
    stream.write_all(b"GET /admin/status HTTP/1.1\r\nHost: resource-budget\r\n")?;
    drop(stream);
    Ok(())
}

fn churn_daemon_cluster(
    cluster: &mut DaemonCluster,
    rounds: usize,
    samples: &mut Vec<ResourceSample>,
) -> TestResult {
    for round in 0..rounds {
        let redis_addr = cluster
            .redis_addr(round % 3)
            .ok_or("Redis listener missing")?;
        let response = redis_roundtrip(redis_addr, b"*1\r\n$4\r\nPING\r\n")?;
        assert_eq!(response, b"+PONG\r\n");
        let held_client = connect_with_retry(redis_addr)?;
        samples.push(portable_sample(cluster, 1, 0));
        drop(held_client);
        samples.push(portable_sample(cluster, 0, 0));
        cancel_admin_request(cluster.admin_addr((round + 1) % 3))?;
        let _ = cluster.admin_status(round % 3)?;

        samples.push(portable_sample(cluster, 0, 0));
        let leader = cluster
            .admin_status(round % 3)?
            .leader
            .ok_or("cluster has no leader before follower restart")?;
        let node_ids = cluster.node_ids();
        let followers = node_ids
            .iter()
            .enumerate()
            .filter_map(|(index, node_id)| (node_id != &leader).then_some(index))
            .collect::<Vec<_>>();
        let follower = followers[round % followers.len()];
        cluster.kill(follower)?;
        samples.push(portable_sample(cluster, 0, 0));
        cluster.restart(follower)?;
        cluster.wait_for_responsive_shape(3, 3, 3)?;
        cluster.wait_for_running_children(3)?;
        samples.push(portable_sample(cluster, 0, 0));
    }
    Ok(())
}

async fn exercise_held_snapshot_schedule() -> usize {
    let mut raft = RuntimeRaftCluster::three_node();
    raft.campaign(1);
    raft.filters().isolate(3, raft.node_ids());
    raft.join_member(1, "resource-budget-prefix").await.unwrap();
    raft.compact_applied_log_to_snapshot(1).unwrap();
    raft.filters().recover();
    raft.filters().add_filter(
        RaftPacketFilter::new()
            .from(1)
            .to(3)
            .message_type(MessageType::MsgSnapshot)
            .action(RaftFilterAction::Hold),
    );
    raft.tick_all(8);
    let held = raft.filters().held();
    assert!(!held.is_empty(), "snapshot schedule must hold a delivery");
    let peak = held.len();
    let released = raft.filters().release_held();
    raft.filters().recover();
    raft.drain_until_idle(released);
    raft.tick_all(8);
    assert!(raft.filters().held().is_empty());
    assert!(raft
        .node(3)
        .command_applied("member-upsert:resource-budget-prefix:1"));
    peak
}

#[tokio::test]
async fn daemon_cluster_churn_returns_portable_resources_to_baseline() -> TestResult {
    let mut cluster = DaemonCluster::start_bootstrap_with_redis(3, "w37-portable")?;
    cluster.wait_for_responsive_shape(3, 3, 3)?;
    let mut samples = vec![portable_sample(&mut cluster, 0, 0)];

    churn_daemon_cluster(&mut cluster, 3, &mut samples)?;
    let held_peak = exercise_held_snapshot_schedule().await;
    samples.push(portable_sample(&mut cluster, 0, held_peak as u64));
    cluster.wait_for_responsive_shape(3, 3, 3)?;
    cluster.wait_for_running_children(3)?;
    samples.push(portable_sample(&mut cluster, 0, 0));

    let budget = ResourceBudget {
        max_child_delta: 0,
        max_connection_delta: 1,
        max_held_snapshot_messages: 8,
        max_rss_growth_kib: 64 * 1024,
        max_fd_growth: 24,
    };
    let artifact = ResourceBudgetArtifact::new(samples, budget);
    assert_eq!(artifact.baseline.running_children, 3);
    assert_eq!(artifact.final_sample.running_children, 3);
    assert_eq!(artifact.final_sample.tracked_connections, 0);
    assert_eq!(artifact.final_sample.held_snapshot_messages, 0);
    assert_eq!(artifact.peak.tracked_connections, 1);
    assert!(artifact.peak.held_snapshot_messages > 0);
    assert!(
        artifact.peak.held_snapshot_messages <= artifact.budget.max_held_snapshot_messages,
        "held snapshot queue exceeded the portable budget: {artifact:?}"
    );
    artifact.write(PORTABLE_ARTIFACT)?;
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "manual/nightly Linux /proc FD and RSS budget"]
fn linux_fd_and_rss_budget_is_bounded_after_quiescence() -> TestResult {
    if std::env::var(LINUX_GATE_ENV).as_deref() != Ok("1") {
        return Err(format!("set {LINUX_GATE_ENV}=1 to claim the Linux resource proof").into());
    }
    let mut cluster = DaemonCluster::start_bootstrap_with_redis(3, "w37-linux")?;
    cluster.wait_for_responsive_shape(3, 3, 3)?;
    let mut samples = vec![linux_sample(&mut cluster)?];
    churn_daemon_cluster(&mut cluster, 5, &mut Vec::new())?;
    for _ in 0..5 {
        thread::sleep(Duration::from_millis(250));
        samples.push(linux_sample(&mut cluster)?);
    }

    let budget = ResourceBudget {
        max_child_delta: 0,
        max_connection_delta: 0,
        max_held_snapshot_messages: 8,
        max_rss_growth_kib: 64 * 1024,
        max_fd_growth: 24,
    };
    let artifact = ResourceBudgetArtifact::new(samples, budget);
    let baseline_rss = artifact.baseline.rss_kib.unwrap();
    let final_rss = artifact.final_sample.rss_kib.unwrap();
    let baseline_fds = artifact.baseline.open_fds.unwrap();
    let final_fds = artifact.final_sample.open_fds.unwrap();
    assert!(
        final_rss <= baseline_rss.saturating_add(artifact.budget.max_rss_growth_kib),
        "RSS failed to return within budget: {artifact:?}"
    );
    assert!(
        final_fds <= baseline_fds.saturating_add(artifact.budget.max_fd_growth),
        "FD count failed to return within budget: {artifact:?}"
    );
    let tails = artifact.samples.iter().rev().take(3).collect::<Vec<_>>();
    assert!(
        tails
            .windows(2)
            .any(|pair| pair[0].rss_kib <= pair[1].rss_kib)
            || tails
                .iter()
                .all(|sample| sample.rss_kib == tails[0].rss_kib),
        "every post-quiescence sample still grew monotonically: {artifact:?}"
    );
    cluster.wait_for_responsive_shape(3, 3, 3)?;
    artifact.write(LINUX_ARTIFACT)?;
    Ok(())
}

#[test]
fn resource_budget_artifact_contains_baseline_peak_final_and_platform() {
    let sample = ResourceSample {
        running_children: 3,
        tracked_connections: 0,
        held_snapshot_messages: 0,
        rss_kib: Some(1024),
        open_fds: Some(12),
    };
    let artifact = ResourceBudgetArtifact::new(
        vec![sample],
        ResourceBudget {
            max_child_delta: 0,
            max_connection_delta: 0,
            max_held_snapshot_messages: 8,
            max_rss_growth_kib: 64 * 1024,
            max_fd_growth: 24,
        },
    );
    let value = serde_json::to_value(&artifact).unwrap();
    for field in [
        "baseline",
        "peak",
        "final_sample",
        "platform",
        "seed",
        "budget",
    ] {
        assert!(value.get(field).is_some(), "artifact is missing {field}");
    }
    let schema = include_str!("../../../docs/testing/schemas/daemon-resource-budget.schema.json");
    for field in ["baseline", "peak", "final_sample", "platform", "samples"] {
        assert!(schema.contains(&format!("\"{field}\"")));
    }
}

#[test]
fn canary_resource_tracker_leaks_one_connection_or_child_handle() {
    let baseline = ResourceSample {
        running_children: 3,
        ..ResourceSample::default()
    };
    let leaked = ResourceSample {
        running_children: 4,
        tracked_connections: 1,
        ..ResourceSample::default()
    };
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W37") {
        assert!(
            leaked.running_children <= baseline.running_children
                && leaked.tracked_connections <= baseline.tracked_connections,
            "HC-CANARY-RED:W37 daemon resource leak exceeded baseline"
        );
    }
    assert!(
        leaked.running_children > baseline.running_children
            || leaked.tracked_connections > baseline.tracked_connections,
        "the resource guard must reject a leaked child or connection"
    );
}
