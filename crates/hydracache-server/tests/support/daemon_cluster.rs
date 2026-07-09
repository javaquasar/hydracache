#![allow(dead_code)]

use std::collections::BTreeSet;
use std::error::Error;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use hydracache_sim::ResourceSample;
use serde_json::Value;

pub const DAEMON_PROCESS_E2E_ENV: &str = "HYDRACACHE_RUN_DAEMON_PROCESS_E2E";
const SERVER_BIN_ENV: &str = "CARGO_BIN_EXE_hydracache-server";
const WAIT_TIMEOUT: Duration = Duration::from_secs(30);
const POLL_INTERVAL: Duration = Duration::from_millis(200);

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
    pub listen_addr: SocketAddr,
    pub cluster_addr: SocketAddr,
    pub admin_addr: SocketAddr,
    pub storage_dir: PathBuf,
    pub cluster_start: &'static str,
}

#[derive(Debug)]
pub struct DaemonNode {
    spec: DaemonNodeSpec,
    child: Option<Child>,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
}

#[derive(Debug)]
pub struct DaemonCluster {
    binary: PathBuf,
    root: PathBuf,
    nodes: Vec<DaemonNode>,
}

impl DaemonCluster {
    pub fn start_bootstrap(count: usize, name: &str) -> TestResult<Self> {
        let binary = server_binary()?;
        let root = unique_root(name)?;
        fs::create_dir_all(&root)?;

        let mut addrs = reserve_node_addrs(count);
        let seed_addrs = addrs
            .iter()
            .map(|(_, cluster_addr, _)| cluster_addr.to_string())
            .collect::<Vec<_>>();
        let mut nodes = Vec::new();
        for index in 0..count {
            let (listen_addr, cluster_addr, admin_addr) = addrs.remove(0);
            let spec = DaemonNodeSpec {
                name: format!("{name}-{index}"),
                node_id: member_node_id_for_addr(cluster_addr),
                listen_addr,
                cluster_addr,
                admin_addr,
                storage_dir: root.join(format!("node-{index}")),
                cluster_start: "bootstrap",
            };
            nodes.push(DaemonNode::new(spec, &root));
        }

        let mut cluster = Self {
            binary,
            root,
            nodes,
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

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn storage_dir(&self, index: usize) -> &Path {
        &self.nodes[index].spec.storage_dir
    }

    pub fn admin_addr(&self, index: usize) -> SocketAddr {
        self.nodes[index].spec.admin_addr
    }

    pub fn running_indices(&mut self) -> Vec<usize> {
        let mut running = Vec::new();
        for (index, node) in self.nodes.iter_mut().enumerate() {
            if node.is_running() {
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
        Err(format!("{label} did not converge before {WAIT_TIMEOUT:?}").into())
    }

    pub fn kill(&mut self, index: usize) -> TestResult {
        self.nodes[index].kill()
    }

    pub fn restart(&mut self, index: usize) -> TestResult {
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
        self.nodes[index].spawn(&self.binary, seed_addrs)
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
            stdout_path,
            stderr_path,
        }
    }

    fn spawn(&mut self, binary: &Path, seed_addrs: &[String]) -> TestResult {
        if self.is_running() {
            return Err(format!("{} is already running", self.spec.name).into());
        }
        fs::create_dir_all(&self.spec.storage_dir)?;
        let stdout = File::create(&self.stdout_path)?;
        let stderr = File::create(&self.stderr_path)?;
        let child = Command::new(binary)
            .env_remove("HYDRACACHE_GRID_INPROC")
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
            .stderr(Stdio::from(stderr))
            .spawn()?;
        self.child = Some(child);
        Ok(())
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
    fn signal(&self, signal: &str) -> TestResult {
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

pub fn leaders(statuses: &[DaemonStatus]) -> BTreeSet<String> {
    statuses
        .iter()
        .filter_map(|status| status.leader.clone())
        .collect()
}

fn server_binary() -> TestResult<PathBuf> {
    std::env::var_os(SERVER_BIN_ENV)
        .map(PathBuf::from)
        .ok_or_else(|| format!("{SERVER_BIN_ENV} is not set; run through cargo test").into())
}

fn unique_root(name: &str) -> TestResult<PathBuf> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    Ok(PathBuf::from(format!(
        "target/test-hydracache-daemon-process/{name}-{}-{now}",
        std::process::id()
    )))
}

fn reserve_node_addrs(count: usize) -> Vec<(SocketAddr, SocketAddr, SocketAddr)> {
    let listeners = (0..count * 3)
        .map(|_| TcpListener::bind("127.0.0.1:0").expect("reserve loopback port"))
        .collect::<Vec<_>>();
    let addrs = listeners
        .iter()
        .map(|listener| listener.local_addr().expect("reserved listener address"))
        .collect::<Vec<_>>();
    drop(listeners);
    addrs
        .chunks_exact(3)
        .map(|chunk| (chunk[0], chunk[1], chunk[2]))
        .collect()
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
