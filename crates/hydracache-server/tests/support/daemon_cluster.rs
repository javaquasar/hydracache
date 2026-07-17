#![allow(dead_code)]

use std::collections::BTreeSet;
use std::error::Error;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use hydracache_sim::ResourceSample;
use serde_json::Value;

pub const DAEMON_PROCESS_E2E_ENV: &str = "HYDRACACHE_RUN_DAEMON_PROCESS_E2E";
pub const REDIS_RESP_MULTINODE_E2E_ENV: &str = "HYDRACACHE_RUN_REDIS_RESP_MULTINODE_E2E";
pub const BUILD_PREVIOUS_DAEMON_ENV: &str = "HYDRACACHE_BUILD_PREVIOUS_DAEMON";
pub const PREVIOUS_DAEMON_BINARY_ENV: &str = "HYDRACACHE_PREVIOUS_DAEMON_BINARY";
pub const PREVIOUS_DAEMON_SOURCE_REF_ENV: &str = "HYDRACACHE_PREVIOUS_DAEMON_SOURCE_REF";
pub const PREVIOUS_DAEMON_SOURCE_COMMIT_ENV: &str = "HYDRACACHE_PREVIOUS_DAEMON_SOURCE_COMMIT";
pub const MIXED_DAEMON_SHIP_MODE_ENV: &str = "HYDRACACHE_MIXED_DAEMON_SHIP_MODE";
pub const PREVIOUS_DAEMON_TAG: &str = "v0.65.0";
pub const PREVIOUS_DAEMON_DEV_COMMIT: &str = "292655168fffda4d217c3dafff6831c602e144ec";
const SERVER_BIN_ENV: &str = "CARGO_BIN_EXE_hydracache-server";
const WAIT_TIMEOUT: Duration = Duration::from_secs(60);
const POLL_INTERVAL: Duration = Duration::from_millis(200);

static PREVIOUS_DAEMON_CACHE: OnceLock<Result<Option<PreviousDaemonBinary>, String>> =
    OnceLock::new();

pub type TestResult<T = ()> = Result<T, Box<dyn Error>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonStatus {
    pub leader: Option<String>,
    pub term: u64,
    pub members: u32,
    pub voters: u32,
    pub quorum_ok: bool,
    pub draining: bool,
}

impl DaemonStatus {
    fn from_json(value: Value) -> TestResult<Self> {
        Ok(Self {
            leader: value
                .get("leader")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            term: value
                .get("term")
                .and_then(Value::as_u64)
                .ok_or("admin status missing term")?,
            members: u32_field(&value, "members")?,
            voters: u32_field(&value, "voters")?,
            quorum_ok: value
                .get("quorum_ok")
                .and_then(Value::as_bool)
                .ok_or("admin status missing quorum_ok")?,
            draining: value
                .get("draining")
                .and_then(Value::as_bool)
                .ok_or("admin status missing draining")?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct DaemonNodeSpec {
    pub name: String,
    pub node_id: String,
    pub binary: PathBuf,
    pub listen_addr: SocketAddr,
    pub cluster_addr: SocketAddr,
    pub admin_addr: SocketAddr,
    pub redis_addr: Option<SocketAddr>,
    pub storage_dir: PathBuf,
    pub cluster_start: &'static str,
}

#[derive(Debug)]
pub struct DaemonNode {
    spec: DaemonNodeSpec,
    child: Option<Child>,
    suspended: bool,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
}

#[derive(Debug)]
pub struct DaemonCluster {
    current_binary: PathBuf,
    root: PathBuf,
    nodes: Vec<DaemonNode>,
    raft_compaction_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct DaemonReplayEvidence {
    pub root: PathBuf,
    pub node_ids: Vec<String>,
    pub stdout_logs: Vec<PathBuf>,
    pub stderr_logs: Vec<PathBuf>,
    pub last_statuses: Vec<DaemonStatus>,
    pub bounded_send_error: Option<String>,
    pub binary_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviousDaemonBinary {
    pub path: PathBuf,
    pub source_ref: String,
    pub source_commit: String,
    pub shipped_tag: bool,
}

impl PreviousDaemonBinary {
    pub fn write_provenance(&self, directory: &Path) -> TestResult<PathBuf> {
        fs::create_dir_all(directory)?;
        let path = directory.join("previous-daemon-provenance.json");
        fs::write(
            &path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "binary": self.path,
                "source_ref": self.source_ref,
                "source_commit": self.source_commit,
                "shipped_tag": self.shipped_tag,
                "required_ship_tag": PREVIOUS_DAEMON_TAG,
                "dev_fallback_commit": PREVIOUS_DAEMON_DEV_COMMIT,
            }))?,
        )?;
        Ok(path)
    }
}

impl DaemonCluster {
    pub fn start_bootstrap(count: usize, name: &str) -> TestResult<Self> {
        Self::start_bootstrap_inner(count, name, false, false)
    }

    pub fn start_bootstrap_with_redis(count: usize, name: &str) -> TestResult<Self> {
        Self::start_bootstrap_inner(count, name, true, false)
    }

    pub fn start_bootstrap_with_raft_compaction(count: usize, name: &str) -> TestResult<Self> {
        Self::start_bootstrap_inner(count, name, false, true)
    }

    pub fn start_bootstrap_with_binaries(binaries: Vec<PathBuf>, name: &str) -> TestResult<Self> {
        Self::start_bootstrap_with_explicit_binaries(binaries, name, false, false)
    }

    pub fn start_bootstrap_with_binaries_and_raft_compaction(
        binaries: Vec<PathBuf>,
        name: &str,
    ) -> TestResult<Self> {
        Self::start_bootstrap_with_explicit_binaries(binaries, name, false, true)
    }

    fn start_bootstrap_inner(
        count: usize,
        name: &str,
        redis_enabled: bool,
        raft_compaction_enabled: bool,
    ) -> TestResult<Self> {
        let current_binary = server_binary()?;
        let binaries = vec![current_binary.clone(); count];
        Self::start_bootstrap_inner_with_binaries(
            binaries,
            current_binary,
            name,
            redis_enabled,
            raft_compaction_enabled,
        )
    }

    fn start_bootstrap_with_explicit_binaries(
        binaries: Vec<PathBuf>,
        name: &str,
        redis_enabled: bool,
        raft_compaction_enabled: bool,
    ) -> TestResult<Self> {
        let current_binary = server_binary()?;
        let binaries =
            assign_explicit_node_binaries(binaries.len(), binaries, current_binary.as_path())?;
        Self::start_bootstrap_inner_with_binaries(
            binaries,
            current_binary,
            name,
            redis_enabled,
            raft_compaction_enabled,
        )
    }

    fn start_bootstrap_inner_with_binaries(
        binaries: Vec<PathBuf>,
        current_binary: PathBuf,
        name: &str,
        redis_enabled: bool,
        raft_compaction_enabled: bool,
    ) -> TestResult<Self> {
        let count = binaries.len();
        if count == 0 {
            return Err("daemon cluster requires at least one explicit node binary".into());
        }
        let root = unique_root(name)?;
        fs::create_dir_all(&root)?;

        let mut addrs = reserve_node_addrs(count, redis_enabled);
        let seed_addrs = addrs
            .iter()
            .map(|(_, cluster_addr, _, _)| cluster_addr.to_string())
            .collect::<Vec<_>>();
        let mut nodes = Vec::new();
        for (index, binary) in binaries.into_iter().enumerate() {
            let (listen_addr, cluster_addr, admin_addr, redis_addr) = addrs.remove(0);
            let spec = DaemonNodeSpec {
                name: format!("{name}-{index}"),
                node_id: member_node_id_for_addr(cluster_addr),
                binary,
                listen_addr,
                cluster_addr,
                admin_addr,
                redis_addr,
                storage_dir: root.join(format!("node-{index}")),
                cluster_start: "bootstrap",
            };
            nodes.push(DaemonNode::new(spec, &root));
        }

        let mut cluster = Self {
            current_binary,
            root,
            nodes,
            raft_compaction_enabled,
        };
        for index in 0..cluster.nodes.len() {
            cluster.spawn_node(index, &seed_addrs)?;
        }
        Ok(cluster)
    }

    pub fn node_ids(&self) -> Vec<String> {
        self.nodes
            .iter()
            .map(|node| node.spec.node_id.clone())
            .collect()
    }

    pub fn binary_paths(&self) -> Vec<PathBuf> {
        self.nodes
            .iter()
            .map(|node| node.spec.binary.clone())
            .collect()
    }

    pub fn binary_path(&self, index: usize) -> &Path {
        &self.nodes[index].spec.binary
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn storage_dir(&self, index: usize) -> &Path {
        &self.nodes[index].spec.storage_dir
    }

    pub fn admin_addr(&self, index: usize) -> SocketAddr {
        self.nodes[index].spec.admin_addr
    }

    pub fn redis_addr(&self, index: usize) -> Option<SocketAddr> {
        self.nodes[index].spec.redis_addr
    }

    pub fn running_indices(&mut self) -> Vec<usize> {
        let mut running = Vec::new();
        for (index, node) in self.nodes.iter_mut().enumerate() {
            if node.is_serving() {
                running.push(index);
            }
        }
        running
    }

    pub fn statuses(&mut self) -> Vec<DaemonStatus> {
        self.running_indices()
            .into_iter()
            .filter_map(|index| self.admin_status(index).ok())
            .collect()
    }

    pub fn overviews(&mut self) -> Vec<Value> {
        self.running_indices()
            .into_iter()
            .filter_map(|index| self.cluster_overview(index).ok())
            .collect()
    }

    pub fn admin_status(&self, index: usize) -> TestResult<DaemonStatus> {
        let value = http_json(
            self.nodes[index].spec.admin_addr,
            "GET",
            "/admin/status",
            true,
        )?;
        DaemonStatus::from_json(value)
    }

    pub fn cluster_overview(&self, index: usize) -> TestResult<Value> {
        http_json(
            self.nodes[index].spec.admin_addr,
            "GET",
            "/cluster/overview",
            false,
        )
    }

    pub fn raft_compaction_status(&self, index: usize) -> TestResult<Value> {
        http_json(
            self.nodes[index].spec.admin_addr,
            "GET",
            "/admin/raft/compaction",
            true,
        )
    }

    pub fn compact_raft_log(&self, index: usize) -> TestResult<Value> {
        http_json(
            self.nodes[index].spec.admin_addr,
            "POST",
            "/admin/raft/compaction",
            true,
        )
    }

    pub fn wait_for_shape(&mut self, members: u32, voters: u32) -> TestResult<Vec<DaemonStatus>> {
        self.wait_for(format!("members={members} voters={voters}"), |cluster| {
            let statuses = cluster.statuses();
            let leaders = leaders(&statuses);
            (!statuses.is_empty()
                && leaders.len() == 1
                && statuses.iter().all(|status| {
                    status.members == members && status.voters == voters && status.quorum_ok
                }))
            .then_some(statuses)
        })
    }

    pub fn wait_for_responsive_shape(
        &mut self,
        expected_statuses: usize,
        members: u32,
        voters: u32,
    ) -> TestResult<Vec<DaemonStatus>> {
        self.wait_for(
            format!("responsive={expected_statuses} members={members} voters={voters}"),
            |cluster| {
                let statuses = cluster.statuses();
                let leaders = leaders(&statuses);
                (statuses.len() == expected_statuses
                    && leaders.len() == 1
                    && statuses.iter().all(|status| {
                        status.members == members && status.voters == voters && status.quorum_ok
                    }))
                .then_some(statuses)
            },
        )
    }

    pub fn wait_for_non_draining_shape(
        &mut self,
        label: &str,
        members: u32,
        voters: u32,
    ) -> TestResult<Vec<DaemonStatus>> {
        self.wait_for(label.to_owned(), |cluster| {
            let statuses = cluster.statuses();
            let active = statuses
                .iter()
                .filter(|status| !status.draining)
                .cloned()
                .collect::<Vec<_>>();
            let leaders = leaders(&active);
            (!active.is_empty()
                && leaders.len() == 1
                && active.iter().all(|status| {
                    status.members == members && status.voters == voters && status.quorum_ok
                }))
            .then_some(statuses)
        })
    }

    pub fn wait_for_leader_not(
        &mut self,
        old_leader: &str,
        members: u32,
        voters: u32,
    ) -> TestResult<Vec<DaemonStatus>> {
        self.wait_for(format!("leader different from {old_leader}"), |cluster| {
            let statuses = cluster.statuses();
            let leaders = leaders(&statuses);
            (leaders.len() == 1
                && !leaders.contains(old_leader)
                && statuses.iter().all(|status| {
                    status.members == members && status.voters == voters && status.quorum_ok
                }))
            .then_some(statuses)
        })
    }

    pub fn wait_for<F, T>(&mut self, label: String, mut predicate: F) -> TestResult<T>
    where
        F: FnMut(&mut Self) -> Option<T>,
    {
        let deadline = Instant::now() + WAIT_TIMEOUT;
        while Instant::now() < deadline {
            if let Some(value) = predicate(self) {
                return Ok(value);
            }
            std::thread::sleep(POLL_INTERVAL);
        }
        let last_statuses = self.statuses();
        Err(format!(
            "{label} did not converge before {WAIT_TIMEOUT:?}; last_statuses={last_statuses:?}"
        )
        .into())
    }

    pub fn kill(&mut self, index: usize) -> TestResult {
        self.nodes[index].kill()
    }

    pub fn restart(&mut self, index: usize) -> TestResult {
        let seed_addrs = self.seed_addrs();
        self.spawn_node(index, &seed_addrs)
    }

    pub fn restart_with_binary(&mut self, index: usize, binary: PathBuf) -> TestResult {
        let selected =
            assign_explicit_node_binaries(1, vec![binary], self.current_binary.as_path())?
                .remove(0);
        let node = self
            .nodes
            .get_mut(index)
            .ok_or_else(|| format!("daemon node index {index} is out of bounds"))?;
        if node.is_running() {
            return Err(format!(
                "{} must be stopped before replacing its binary",
                node.spec.name
            )
            .into());
        }
        node.spec.binary = selected;
        let seed_addrs = self.seed_addrs();
        self.spawn_node(index, &seed_addrs)
    }

    pub fn drain(&self, index: usize) -> TestResult<Value> {
        http_json(
            self.nodes[index].spec.admin_addr,
            "POST",
            "/admin/drain",
            true,
        )
    }

    pub fn resource_sample(&mut self) -> Option<ResourceSample> {
        let samples = self
            .running_indices()
            .into_iter()
            .filter_map(|index| self.nodes[index].resource_sample())
            .collect::<Vec<_>>();
        (!samples.is_empty()).then(|| ResourceSample {
            storage_bytes: samples.iter().map(|sample| sample.rss_kib * 1024).sum(),
            network_in_flight: samples.iter().map(|sample| sample.open_fds).sum(),
            client_in_flight: 0,
            subscriber_pending: 0,
        })
    }

    pub fn running_child_count(&mut self) -> usize {
        self.running_indices().len()
    }

    pub fn wait_for_running_children(&mut self, expected: usize) -> TestResult {
        self.wait_for(format!("running children={expected}"), |cluster| {
            (cluster.running_child_count() == expected).then_some(())
        })
    }

    pub fn os_resource_totals(&mut self) -> Option<(u64, u64)> {
        let running = self.running_indices();
        let samples = running
            .iter()
            .filter_map(|index| self.nodes[*index].resource_sample())
            .collect::<Vec<_>>();
        (samples.len() == running.len() && !samples.is_empty()).then(|| {
            (
                samples.iter().map(|sample| sample.rss_kib).sum(),
                samples.iter().map(|sample| sample.open_fds).sum(),
            )
        })
    }

    pub fn replay_evidence(&mut self, bounded_send_error: Option<String>) -> DaemonReplayEvidence {
        DaemonReplayEvidence {
            root: self.root.clone(),
            node_ids: self.node_ids(),
            stdout_logs: self
                .nodes
                .iter()
                .map(|node| node.stdout_path.clone())
                .collect(),
            stderr_logs: self
                .nodes
                .iter()
                .map(|node| node.stderr_path.clone())
                .collect(),
            last_statuses: self.statuses(),
            bounded_send_error,
            binary_paths: self.binary_paths(),
        }
    }

    #[cfg(target_os = "linux")]
    pub fn suspend(&mut self, index: usize) -> TestResult {
        self.nodes[index].signal("STOP")
    }

    #[cfg(target_os = "linux")]
    pub fn resume(&mut self, index: usize) -> TestResult {
        self.nodes[index].signal("CONT")
    }

    fn seed_addrs(&self) -> Vec<String> {
        self.nodes
            .iter()
            .map(|node| node.spec.cluster_addr.to_string())
            .collect()
    }

    fn spawn_node(&mut self, index: usize, seed_addrs: &[String]) -> TestResult {
        self.nodes[index].spawn(seed_addrs, self.raft_compaction_enabled)
    }
}

impl Drop for DaemonCluster {
    fn drop(&mut self) {
        for node in &mut self.nodes {
            let _ = node.kill();
        }
    }
}

impl DaemonNode {
    fn new(spec: DaemonNodeSpec, root: &Path) -> Self {
        let stdout_path = root.join(format!("{}.stdout.log", spec.name));
        let stderr_path = root.join(format!("{}.stderr.log", spec.name));
        Self {
            spec,
            child: None,
            suspended: false,
            stdout_path,
            stderr_path,
        }
    }

    fn spawn(&mut self, seed_addrs: &[String], raft_compaction_enabled: bool) -> TestResult {
        if self.is_running() {
            return Err(format!("{} is already running", self.spec.name).into());
        }
        fs::create_dir_all(&self.spec.storage_dir)?;
        let stdout = File::create(&self.stdout_path)?;
        let stderr = File::create(&self.stderr_path)?;
        let mut command = Command::new(&self.spec.binary);
        command
            .env_remove("HYDRACACHE_GRID_INPROC")
            .env_remove("HYDRACACHE_RAFT_COMPACTION")
            .env("HYDRACACHE_ROLE", "member")
            .env("HYDRACACHE_NODE_ID", &self.spec.node_id)
            .env("HYDRACACHE_LISTEN_ADDR", self.spec.listen_addr.to_string())
            .env(
                "HYDRACACHE_CLUSTER_ADDR",
                self.spec.cluster_addr.to_string(),
            )
            .env(
                "HYDRACACHE_CLUSTER_ADVERTISE_ADDR",
                self.spec.cluster_addr.to_string(),
            )
            .env("HYDRACACHE_ADMIN_ADDR", self.spec.admin_addr.to_string())
            .env("HYDRACACHE_CLUSTER_START", self.spec.cluster_start)
            .env("HYDRACACHE_SEEDS", seed_addrs.join(","))
            .env("HYDRACACHE_STORAGE_DIR", &self.spec.storage_dir)
            .env("HYDRACACHE_JOIN_TIMEOUT_MS", "10000")
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr));
        if raft_compaction_enabled {
            command.env("HYDRACACHE_RAFT_COMPACTION", "true");
        }
        if let Some(redis_addr) = self.spec.redis_addr {
            command
                .env("HYDRACACHE_REDIS_API_ENABLED", "true")
                .env("HYDRACACHE_REDIS_ADDR", redis_addr.to_string());
        }
        let child = command.spawn()?;
        self.child = Some(child);
        self.suspended = false;
        Ok(())
    }

    fn is_serving(&mut self) -> bool {
        self.is_running() && !self.suspended
    }

    fn is_running(&mut self) -> bool {
        let Some(child) = self.child.as_mut() else {
            return false;
        };
        match child.try_wait() {
            Ok(Some(_)) => {
                self.child = None;
                false
            }
            Ok(None) => true,
            Err(_) => false,
        }
    }

    fn kill(&mut self) -> TestResult {
        let Some(mut child) = self.child.take() else {
            return Ok(());
        };
        self.suspended = false;
        if child.try_wait()?.is_none() {
            child.kill()?;
        }
        let _ = child.wait()?;
        Ok(())
    }

    fn resource_sample(&self) -> Option<ProcessResourceSample> {
        ProcessResourceSample::for_pid(self.child.as_ref()?.id())
    }

    #[cfg(target_os = "linux")]
    fn signal(&mut self, signal: &str) -> TestResult {
        let pid = self
            .child
            .as_ref()
            .ok_or("cannot signal a stopped daemon")?
            .id()
            .to_string();
        let status = Command::new("kill")
            .arg(format!("-{signal}"))
            .arg(pid)
            .status()?;
        if status.success() {
            match signal {
                "STOP" => self.suspended = true,
                "CONT" => self.suspended = false,
                _ => {}
            }
            Ok(())
        } else {
            Err(format!("kill -{signal} failed with {status}").into())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProcessResourceSample {
    rss_kib: u64,
    open_fds: u64,
}

impl ProcessResourceSample {
    #[cfg(target_os = "linux")]
    fn for_pid(pid: u32) -> Option<Self> {
        let status = fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
        let rss_kib = status.lines().find_map(|line| {
            let value = line.strip_prefix("VmRSS:")?;
            value.split_whitespace().next()?.parse().ok()
        })?;
        let open_fds = fs::read_dir(format!("/proc/{pid}/fd")).ok()?.count() as u64;
        Some(Self { rss_kib, open_fds })
    }

    #[cfg(not(target_os = "linux"))]
    fn for_pid(_pid: u32) -> Option<Self> {
        None
    }
}

pub fn daemon_process_e2e_enabled() -> bool {
    std::env::var(DAEMON_PROCESS_E2E_ENV)
        .map(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

pub fn skip_unless_daemon_process_e2e(test_name: &str) -> bool {
    if daemon_process_e2e_enabled() {
        return true;
    }
    eprintln!(
        "skipping {test_name}: set {DAEMON_PROCESS_E2E_ENV}=1 to run real-process daemon E2E"
    );
    false
}

pub fn redis_resp_multinode_e2e_enabled() -> bool {
    std::env::var(REDIS_RESP_MULTINODE_E2E_ENV)
        .map(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

pub fn skip_unless_redis_resp_multinode_e2e(test_name: &str) -> bool {
    if redis_resp_multinode_e2e_enabled() {
        return true;
    }
    eprintln!(
        "skipping {test_name}: set {REDIS_RESP_MULTINODE_E2E_ENV}=1 to run real-process Redis RESP multinode E2E"
    );
    false
}

pub fn leaders(statuses: &[DaemonStatus]) -> BTreeSet<String> {
    statuses
        .iter()
        .filter_map(|status| status.leader.clone())
        .collect()
}

fn server_binary() -> TestResult<PathBuf> {
    resolve_server_binary(
        std::env::var_os(SERVER_BIN_ENV),
        option_env!("CARGO_BIN_EXE_hydracache-server"),
    )
}

pub fn current_server_binary() -> TestResult<PathBuf> {
    server_binary()
}

pub fn assign_explicit_node_binaries(
    expected_nodes: usize,
    requested: Vec<PathBuf>,
    current_binary: &Path,
) -> TestResult<Vec<PathBuf>> {
    if requested.len() != expected_nodes {
        return Err(format!(
            "explicit daemon binary count {} does not match node count {expected_nodes}",
            requested.len()
        )
        .into());
    }
    if requested.iter().any(|path| path.as_os_str().is_empty()) {
        return Err("explicit daemon binary paths must not be empty".into());
    }
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W6") {
        return Ok(vec![current_binary.to_path_buf(); expected_nodes]);
    }
    Ok(requested)
}

pub fn resolve_previous_daemon_binary() -> TestResult<Option<PreviousDaemonBinary>> {
    match PREVIOUS_DAEMON_CACHE.get_or_init(|| {
        resolve_previous_daemon_binary_uncached().map_err(|error| error.to_string())
    }) {
        Ok(binary) => Ok(binary.clone()),
        Err(error) => Err(error.clone().into()),
    }
}

fn resolve_previous_daemon_binary_uncached() -> TestResult<Option<PreviousDaemonBinary>> {
    let root = workspace_root();
    let ship_mode = environment_flag(MIXED_DAEMON_SHIP_MODE_ENV);
    let tag_commit = git_resolve_commit(&root, PREVIOUS_DAEMON_TAG)?;
    if ship_mode {
        let Some(commit) = tag_commit.as_deref() else {
            return Err(format!(
                "{MIXED_DAEMON_SHIP_MODE_ENV}=1 requires full-history tag {PREVIOUS_DAEMON_TAG}; no dev fallback is valid for ship evidence"
            )
            .into());
        };
        if !git_is_ancestor(&root, commit, "HEAD")? {
            return Err(format!(
                "ship provenance tag {PREVIOUS_DAEMON_TAG} ({commit}) is not an ancestor of HEAD"
            )
            .into());
        }
    }

    if let Some(path) = std::env::var_os(PREVIOUS_DAEMON_BINARY_ENV) {
        let path = fs::canonicalize(PathBuf::from(path)).map_err(|error| {
            format!("{PREVIOUS_DAEMON_BINARY_ENV} does not resolve to a readable binary: {error}")
        })?;
        if !path.is_file() {
            return Err(format!(
                "{PREVIOUS_DAEMON_BINARY_ENV} is not a file: {}",
                path.display()
            )
            .into());
        }
        let source_ref = required_environment(PREVIOUS_DAEMON_SOURCE_REF_ENV)?;
        let source_commit = required_environment(PREVIOUS_DAEMON_SOURCE_COMMIT_ENV)?;
        validate_commit_id(&source_commit)?;
        let shipped_tag = validate_previous_provenance(
            &root,
            &source_ref,
            &source_commit,
            tag_commit.as_deref(),
            ship_mode,
        )?;
        return Ok(Some(PreviousDaemonBinary {
            path,
            source_ref,
            source_commit,
            shipped_tag,
        }));
    }

    if !environment_flag(BUILD_PREVIOUS_DAEMON_ENV) {
        if ship_mode {
            return Err(format!(
                "ship mode requires {PREVIOUS_DAEMON_BINARY_ENV} or {BUILD_PREVIOUS_DAEMON_ENV}=1"
            )
            .into());
        }
        return Ok(None);
    }

    let (source_ref, source_commit, shipped_tag) = match tag_commit {
        Some(commit) => (PREVIOUS_DAEMON_TAG.to_owned(), commit, true),
        None => {
            let Some(commit) = git_resolve_commit(&root, PREVIOUS_DAEMON_DEV_COMMIT)? else {
                return Err(format!(
                    "neither {PREVIOUS_DAEMON_TAG} nor pinned dev fallback {PREVIOUS_DAEMON_DEV_COMMIT} is available in repository history"
                )
                .into());
            };
            if commit != PREVIOUS_DAEMON_DEV_COMMIT {
                return Err(format!(
                    "pinned dev fallback resolved unexpectedly: expected {PREVIOUS_DAEMON_DEV_COMMIT}, got {commit}"
                )
                .into());
            }
            if !git_is_ancestor(&root, &commit, "HEAD")? {
                return Err(
                    format!("pinned dev fallback {commit} is not an ancestor of HEAD").into(),
                );
            }
            (PREVIOUS_DAEMON_DEV_COMMIT.to_owned(), commit, false)
        }
    };
    let path = build_previous_daemon(&root, &source_ref, &source_commit)?;
    Ok(Some(PreviousDaemonBinary {
        path,
        source_ref,
        source_commit,
        shipped_tag,
    }))
}

pub fn ensure_distinct_daemon_binaries(previous: &Path, current: &Path) -> TestResult {
    let previous = fs::canonicalize(previous)
        .map_err(|error| format!("failed to canonicalize previous daemon binary: {error}"))?;
    let current = fs::canonicalize(current)
        .map_err(|error| format!("failed to canonicalize current daemon binary: {error}"))?;
    if previous == current || files_are_identical(&previous, &current)? {
        return Err(format!(
            "mixed-version harness resolved identical previous/current daemon bytes: previous={} current={}",
            previous.display(),
            current.display()
        )
        .into());
    }
    Ok(())
}

fn files_are_identical(left: &Path, right: &Path) -> std::io::Result<bool> {
    if fs::metadata(left)?.len() != fs::metadata(right)?.len() {
        return Ok(false);
    }
    let mut left = File::open(left)?;
    let mut right = File::open(right)?;
    let mut left_buffer = [0_u8; 64 * 1024];
    let mut right_buffer = [0_u8; 64 * 1024];
    loop {
        let left_read = left.read(&mut left_buffer)?;
        let right_read = right.read(&mut right_buffer)?;
        if left_read != right_read || left_buffer[..left_read] != right_buffer[..right_read] {
            return Ok(false);
        }
        if left_read == 0 {
            return Ok(true);
        }
    }
}

fn validate_previous_provenance(
    root: &Path,
    source_ref: &str,
    source_commit: &str,
    tag_commit: Option<&str>,
    ship_mode: bool,
) -> TestResult<bool> {
    if source_ref == PREVIOUS_DAEMON_TAG {
        let actual = tag_commit.ok_or_else(|| {
            format!(
                "explicit previous binary claims {PREVIOUS_DAEMON_TAG}, but the full-history tag is absent"
            )
        })?;
        if actual != source_commit {
            return Err(format!(
                "previous daemon tag provenance mismatch: {PREVIOUS_DAEMON_TAG} resolves to {actual}, supplied commit is {source_commit}"
            )
            .into());
        }
        if !git_is_ancestor(root, actual, "HEAD")? {
            return Err(
                format!("previous daemon tag commit {actual} is not an ancestor of HEAD").into(),
            );
        }
        return Ok(true);
    }

    if ship_mode {
        return Err(format!(
            "ship evidence requires source_ref={PREVIOUS_DAEMON_TAG}; got {source_ref}"
        )
        .into());
    }
    if source_ref != PREVIOUS_DAEMON_DEV_COMMIT || source_commit != PREVIOUS_DAEMON_DEV_COMMIT {
        return Err(format!(
            "development previous daemon must use pinned base commit {PREVIOUS_DAEMON_DEV_COMMIT}; got ref={source_ref} commit={source_commit}"
        )
        .into());
    }
    let resolved = git_resolve_commit(root, PREVIOUS_DAEMON_DEV_COMMIT)?
        .ok_or("pinned previous daemon development commit is absent")?;
    if resolved != PREVIOUS_DAEMON_DEV_COMMIT || !git_is_ancestor(root, &resolved, "HEAD")? {
        return Err("pinned previous daemon development commit has invalid ancestry".into());
    }
    Ok(false)
}

fn build_previous_daemon(
    workspace: &Path,
    source_ref: &str,
    source_commit: &str,
) -> TestResult<PathBuf> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let safe_ref = source_ref
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let build_root = workspace.join(format!(
        "target/test-hydracache-daemon-process/previous-builds/{safe_ref}-{}-{now}",
        std::process::id()
    ));
    let worktree = build_root.join("worktree");
    let target_dir = build_root.join("cargo-target");
    fs::create_dir_all(&build_root)?;

    let worktree_status = Command::new("git")
        .arg("-C")
        .arg(workspace)
        .args(["worktree", "add", "--detach"])
        .arg(&worktree)
        .arg(source_commit)
        .status()?;
    if !worktree_status.success() {
        return Err(format!(
            "failed to create detached previous-daemon worktree at {} from {source_ref} ({source_commit}): {worktree_status}",
            worktree.display()
        )
        .into());
    }

    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    let build_status = Command::new(cargo)
        .current_dir(&worktree)
        .args([
            "build",
            "--locked",
            "-p",
            "hydracache-server",
            "--bin",
            "hydracache-server",
            "--target-dir",
        ])
        .arg(&target_dir)
        .status()?;
    if !build_status.success() {
        return Err(format!(
            "previous daemon build failed in detached worktree {}: {build_status}",
            worktree.display()
        )
        .into());
    }
    let binary = target_dir
        .join("debug")
        .join(format!("hydracache-server{}", std::env::consts::EXE_SUFFIX));
    if !binary.is_file() {
        return Err(format!(
            "previous daemon build succeeded but binary is missing: {}",
            binary.display()
        )
        .into());
    }
    Ok(fs::canonicalize(binary)?)
}

fn workspace_root() -> PathBuf {
    let canonical = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root must be canonicalizable");
    command_compatible_path(canonical)
}

#[cfg(windows)]
fn command_compatible_path(path: PathBuf) -> PathBuf {
    let rendered = path.to_string_lossy();
    if let Some(rest) = rendered.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{rest}"));
    }
    rendered
        .strip_prefix(r"\\?\")
        .map(PathBuf::from)
        .unwrap_or(path)
}

#[cfg(not(windows))]
fn command_compatible_path(path: PathBuf) -> PathBuf {
    path
}

fn git_resolve_commit(root: &Path, reference: &str) -> TestResult<Option<String>> {
    let revision = format!("{reference}^{{commit}}");
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--verify"])
        .arg(revision)
        .output()?;
    if !output.status.success() {
        return Ok(None);
    }
    let commit = String::from_utf8(output.stdout)?.trim().to_owned();
    validate_commit_id(&commit)?;
    Ok(Some(commit))
}

fn git_is_ancestor(root: &Path, ancestor: &str, descendant: &str) -> TestResult<bool> {
    Ok(Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["merge-base", "--is-ancestor", ancestor, descendant])
        .status()?
        .success())
}

fn validate_commit_id(commit: &str) -> TestResult {
    if commit.len() != 40 || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("invalid full Git commit id: {commit}").into());
    }
    Ok(())
}

fn required_environment(name: &str) -> TestResult<String> {
    let value = std::env::var(name)
        .map_err(|_| format!("{name} is required with an explicit previous daemon binary"))?;
    if value.trim().is_empty() {
        return Err(format!("{name} must not be empty").into());
    }
    Ok(value)
}

fn environment_flag(name: &str) -> bool {
    std::env::var(name)
        .map(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

pub fn resolve_server_binary(
    runtime_binary: Option<OsString>,
    compile_time_binary: Option<&str>,
) -> TestResult<PathBuf> {
    runtime_binary
        .map(PathBuf::from)
        .or_else(|| compile_time_binary.map(PathBuf::from))
        .ok_or_else(|| {
            format!(
                "{SERVER_BIN_ENV} is unavailable at runtime and compile time; run through cargo test"
            )
            .into()
        })
}

fn unique_root(name: &str) -> TestResult<PathBuf> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    Ok(PathBuf::from(format!(
        "target/test-hydracache-daemon-process/{name}-{}-{now}",
        std::process::id()
    )))
}

fn reserve_node_addrs(
    count: usize,
    redis_enabled: bool,
) -> Vec<(SocketAddr, SocketAddr, SocketAddr, Option<SocketAddr>)> {
    let surface_count = if redis_enabled { 4 } else { 3 };
    let reservations = (0..count * surface_count)
        .map(|_| reserve_dual_protocol_loopback())
        .collect::<Vec<_>>();
    let addrs = reservations
        .iter()
        .map(|(tcp, _)| tcp.local_addr().expect("reserved listener address"))
        .collect::<Vec<_>>();
    drop(reservations);
    addrs
        .chunks_exact(surface_count)
        .map(|chunk| {
            let redis_addr = redis_enabled.then(|| chunk[3]);
            (chunk[0], chunk[1], chunk[2], redis_addr)
        })
        .collect()
}

fn reserve_dual_protocol_loopback() -> (TcpListener, UdpSocket) {
    loop {
        let tcp = TcpListener::bind("127.0.0.1:0").expect("reserve loopback TCP port");
        match complete_dual_protocol_reservation(tcp) {
            Ok(reservation) => return reservation,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::AddrInUse | std::io::ErrorKind::PermissionDenied
                ) => {}
            Err(error) => panic!("reserve loopback UDP port for TCP candidate: {error}"),
        }
    }
}

fn complete_dual_protocol_reservation(
    tcp: TcpListener,
) -> std::io::Result<(TcpListener, UdpSocket)> {
    let address = tcp.local_addr()?;
    let udp = UdpSocket::bind(address)?;
    Ok((tcp, udp))
}

fn member_node_id_for_addr(addr: SocketAddr) -> String {
    let suffix = addr
        .to_string()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    format!("member-{suffix}")
}

fn http_json(addr: SocketAddr, method: &str, path: &str, admin: bool) -> TestResult<Value> {
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(2))?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;

    let mut request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\nContent-Length: 0\r\n"
    );
    if admin {
        request.push_str(
            "x-hydracache-client-id: daemon-process-test\r\nx-hydracache-tenant: system\r\nx-hydracache-admin: true\r\n",
        );
    }
    request.push_str("\r\n");
    stream.write_all(request.as_bytes())?;

    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    let (head, body) = response
        .split_once("\r\n\r\n")
        .ok_or("malformed HTTP response")?;
    let status = head
        .split_whitespace()
        .nth(1)
        .ok_or("HTTP response missing status")?;
    if status != "200" && status != "202" {
        return Err(format!("HTTP {method} {path} returned {status}: {body}").into());
    }
    Ok(serde_json::from_str(body)?)
}

fn u32_field(value: &Value, field: &'static str) -> TestResult<u32> {
    let raw = value
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("admin status missing {field}"))?;
    Ok(u32::try_from(raw)?)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::net::{TcpListener, UdpSocket};

    use super::{
        complete_dual_protocol_reservation, ensure_distinct_daemon_binaries, reserve_node_addrs,
        unique_root,
    };

    #[test]
    fn reserve_node_addrs_skips_redis_surface_when_disabled() {
        let addrs = reserve_node_addrs(3, false);

        assert_eq!(addrs.len(), 3);
        assert!(addrs
            .iter()
            .all(|(_, _, _, redis_addr)| redis_addr.is_none()));
    }

    #[test]
    fn reserve_node_addrs_reserves_redis_surface_when_enabled() {
        let addrs = reserve_node_addrs(2, true);

        assert_eq!(addrs.len(), 2);
        for (http_addr, gossip_addr, raft_addr, redis_addr) in addrs {
            let redis_addr = redis_addr.expect("redis surface should be reserved");
            assert_ne!(http_addr, redis_addr);
            assert_ne!(gossip_addr, redis_addr);
            assert_ne!(raft_addr, redis_addr);
        }
    }

    #[test]
    fn dual_protocol_reservation_rejects_udp_occupied_tcp_candidate() {
        let udp = UdpSocket::bind("127.0.0.1:0").expect("reserve UDP blocker");
        let address = udp.local_addr().unwrap();
        let tcp = TcpListener::bind(address).expect("TCP namespace may share the UDP port");

        let error = complete_dual_protocol_reservation(tcp).unwrap_err();
        assert!(matches!(
            error.kind(),
            std::io::ErrorKind::AddrInUse | std::io::ErrorKind::PermissionDenied
        ));
    }

    #[test]
    fn distinct_binary_check_rejects_a_copied_current_binary() {
        let root = unique_root("identical-daemon-binaries").unwrap();
        fs::create_dir_all(&root).unwrap();
        let previous = root.join("previous-daemon");
        let current = root.join("current-daemon");
        fs::write(&previous, b"same executable bytes").unwrap();
        fs::write(&current, b"same executable bytes").unwrap();

        let error = ensure_distinct_daemon_binaries(&previous, &current).unwrap_err();
        assert!(error.to_string().contains("identical previous/current"));
        let _ = fs::remove_dir_all(root);
    }
}
