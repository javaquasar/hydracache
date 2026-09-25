use std::error::Error;
use std::fs::{self, File};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use hydracache_client_hc2::{ClientConfig, GrpcMtlsAdapter, GrpcMtlsConfig, Hc2Client};
use hydracache_client_transport_axum::{ClientSurfaceLimits, ClientSurfaceState};
use hydracache_loadgen::allocation::measure_allocations;
use hydracache_server::{serve_hc2_listener, Hc2ClientPlaneService, Hc2ListenerTls, TlsConfig};
use rcgen::{
    BasicConstraints, CertificateParams, CertifiedIssuer, ExtendedKeyUsagePurpose, IsCa, KeyPair,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tempfile::TempDir;
use tokio::net::TcpListener;
use tokio::sync::watch;

#[derive(Clone, Deserialize, Serialize)]
struct MemorySnapshot {
    working_set_bytes: u64,
    peak_working_set_bytes: u64,
    pagefile_bytes: u64,
    peak_pagefile_bytes: u64,
}

#[derive(Serialize)]
struct ProfileResult {
    schema_version: u32,
    profile_id: &'static str,
    ownership_scope: &'static str,
    cardinality: usize,
    active_connections: u64,
    accepted_connections: u64,
    closed_connections_at_plateau: u64,
    final_closed_connections: u64,
    client_zero_resource_snapshots: usize,
    gross_allocated_bytes: u64,
    gross_allocated_bytes_per_connection: f64,
    baseline: MemorySnapshot,
    plateau: MemorySnapshot,
    post_close: MemorySnapshot,
}

#[derive(Deserialize, Serialize)]
struct EndpointProfile {
    gross_allocated_bytes: u64,
    gross_allocated_bytes_per_connection: f64,
    baseline: MemorySnapshot,
    plateau: MemorySnapshot,
    post_close: MemorySnapshot,
}

#[derive(Deserialize, Serialize)]
struct ServerReady {
    address: String,
}

#[derive(Deserialize, Serialize)]
struct SplitServerResult {
    active_connections: u64,
    accepted_connections: u64,
    closed_connections_at_plateau: u64,
    final_closed_connections: u64,
    profile: EndpointProfile,
}

#[derive(Serialize)]
struct SplitProfileResult {
    schema_version: u32,
    profile_id: &'static str,
    cardinality: usize,
    server_scope: &'static str,
    client_scope: &'static str,
    active_connections: u64,
    accepted_connections: u64,
    closed_connections_at_plateau: u64,
    final_closed_connections: u64,
    client_zero_resource_snapshots: usize,
    client: EndpointProfile,
    server: EndpointProfile,
}

struct TestPki {
    ca: String,
    server_cert: String,
    server_key: String,
    client_cert: String,
    client_key: String,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [mode, cardinality] if mode == "split" => run_split(parse_cardinality(cardinality)?).await,
        [mode, cardinality, control_dir] if mode == "split-server" => {
            run_split_server(parse_cardinality(cardinality)?, Path::new(control_dir)).await
        }
        [cardinality] => run_combined(parse_cardinality(cardinality)?).await,
        _ => Err("usage: hc2-connection-profile-073 [split] <1|10|100|1000>".into()),
    }
}

fn parse_cardinality(value: &str) -> Result<usize, Box<dyn Error>> {
    let cardinality = value.parse::<usize>()?;
    if !matches!(cardinality, 1 | 10 | 100 | 1_000) {
        return Err("unsupported frozen W5 cardinality".into());
    }
    Ok(cardinality)
}

async fn run_combined(cardinality: usize) -> Result<(), Box<dyn Error>> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let temp = TempDir::new()?;
    let pki = test_pki()?;
    write_tls(temp.path(), &pki)?;
    let tls = Hc2ListenerTls::from_server_config(&tls_config(temp.path()))?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let state = std::sync::Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default())?);
    let service = Hc2ClientPlaneService::new(state, "profile-cluster");
    let observed = service.clone();
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let serving =
        tokio::spawn(async move { serve_hc2_listener(listener, service, tls, shutdown_rx).await });
    tokio::time::sleep(Duration::from_millis(50)).await;
    let baseline = process_memory()?;

    let (clients_result, allocation) = measure_allocations(cardinality as u64, async {
        let mut clients = Vec::with_capacity(cardinality);
        for index in 0..cardinality {
            clients.push(
                Hc2Client::connect(
                    &adapter(addr, &pki)?,
                    ClientConfig::new(format!("profile-{cardinality}-{index}"), "tenant-a"),
                )
                .await?,
            );
        }
        wait_for_active(&observed, cardinality).await?;
        let zero_snapshots = clients
            .iter()
            .filter(|client| {
                let retained = client.retained_state();
                retained.pending_invocations == 0
                    && retained.pending_subscriptions == 0
                    && retained.active_subscriptions == 0
                    && retained.pending_sessions == 0
                    && retained.active_sessions == 0
            })
            .count();
        Ok::<_, Box<dyn Error>>((clients, zero_snapshots))
    })
    .await;
    let (clients, zero_snapshots) = clients_result?;
    let plateau = process_memory()?;
    let active = observed.accounting();
    if active.active_connections != cardinality as u64
        || active
            .accepted_connections
            .saturating_sub(active.closed_connections)
            != active.active_connections
        || zero_snapshots != cardinality
    {
        return Err("HC/2 plateau accounting mismatch".into());
    }

    for client in &clients {
        client.close();
    }
    drop(clients);
    wait_for_zero(&observed).await?;
    tokio::time::sleep(Duration::from_millis(100)).await;
    let post_close = process_memory()?;
    let closed = observed.accounting();
    if closed.accepted_connections != closed.closed_connections {
        return Err("HC/2 close accounting mismatch".into());
    }
    shutdown_tx.send(true)?;
    serving.await??;

    println!(
        "{}",
        serde_json::to_string(&ProfileResult {
            schema_version: 1,
            profile_id: "w5-hc2-connection-profile-073-v1",
            ownership_scope: "combined-local-client-server-process",
            cardinality,
            active_connections: active.active_connections,
            accepted_connections: active.accepted_connections,
            closed_connections_at_plateau: active.closed_connections,
            final_closed_connections: closed.closed_connections,
            client_zero_resource_snapshots: zero_snapshots,
            gross_allocated_bytes: allocation.gross_allocated_bytes,
            gross_allocated_bytes_per_connection: allocation.gross_allocated_bytes_per_operation,
            baseline,
            plateau,
            post_close,
        })?
    );
    Ok(())
}

async fn run_split(cardinality: usize) -> Result<(), Box<dyn Error>> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let temp = TempDir::new()?;
    let pki = test_pki()?;
    write_tls(temp.path(), &pki)?;

    let stdout_path = temp.path().join("server.stdout.txt");
    let stderr_path = temp.path().join("server.stderr.txt");
    let child = Command::new(std::env::current_exe()?)
        .arg("split-server")
        .arg(cardinality.to_string())
        .arg(temp.path())
        .stdin(Stdio::null())
        .stdout(Stdio::from(File::create(&stdout_path)?))
        .stderr(Stdio::from(File::create(&stderr_path)?))
        .spawn()?;
    let mut child = ChildGuard::new(child);

    let ready: ServerReady =
        wait_for_json(&temp.path().join("server-ready.json"), &mut child).await?;
    let address = ready.address.parse()?;
    let baseline = process_memory()?;
    fs::write(temp.path().join("start-measurement"), [])?;
    wait_for_path(&temp.path().join("counting-ready"), &mut child).await?;

    let (clients_result, allocation) = measure_allocations(cardinality as u64, async {
        let mut clients = Vec::with_capacity(cardinality);
        for index in 0..cardinality {
            clients.push(
                Hc2Client::connect(
                    &adapter(address, &pki)?,
                    ClientConfig::new(format!("split-profile-{cardinality}-{index}"), "tenant-a"),
                )
                .await?,
            );
        }
        let zero_snapshots = clients.iter().filter(client_resources_zero).count();
        Ok::<_, Box<dyn Error>>((clients, zero_snapshots))
    })
    .await;
    let (clients, zero_snapshots) = clients_result?;
    wait_for_path(&temp.path().join("server-plateau"), &mut child).await?;
    let plateau = process_memory()?;
    if zero_snapshots != cardinality {
        return Err("split client retained resource mismatch".into());
    }

    for client in &clients {
        client.close();
    }
    drop(clients);
    tokio::time::sleep(Duration::from_millis(100)).await;
    let post_close = process_memory()?;
    let status = child.wait_for_exit().await?;
    if !status.success() {
        return Err(format!("split server exited with {status}").into());
    }
    if fs::metadata(&stdout_path)?.len() != 0 || fs::metadata(&stderr_path)?.len() != 0 {
        return Err("split server emitted unexpected stdout or stderr".into());
    }
    let server: SplitServerResult = read_json(&temp.path().join("server-result.json"))?;
    if server.active_connections != cardinality as u64
        || server
            .accepted_connections
            .saturating_sub(server.closed_connections_at_plateau)
            != server.active_connections
        || server.accepted_connections != server.final_closed_connections
    {
        return Err("split server accounting mismatch".into());
    }

    println!(
        "{}",
        serde_json::to_string(&SplitProfileResult {
            schema_version: 1,
            profile_id: "w5-hc2-split-connection-profile-073-v1",
            cardinality,
            server_scope: "dedicated-local-server-process",
            client_scope: "dedicated-local-client-controller-process",
            active_connections: server.active_connections,
            accepted_connections: server.accepted_connections,
            closed_connections_at_plateau: server.closed_connections_at_plateau,
            final_closed_connections: server.final_closed_connections,
            client_zero_resource_snapshots: zero_snapshots,
            client: EndpointProfile {
                gross_allocated_bytes: allocation.gross_allocated_bytes,
                gross_allocated_bytes_per_connection: allocation
                    .gross_allocated_bytes_per_operation,
                baseline,
                plateau,
                post_close,
            },
            server: server.profile,
        })?
    );
    Ok(())
}

async fn run_split_server(cardinality: usize, control_dir: &Path) -> Result<(), Box<dyn Error>> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let tls = Hc2ListenerTls::from_server_config(&tls_config(control_dir))?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let state = std::sync::Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default())?);
    let service = Hc2ClientPlaneService::new(state, "split-profile-cluster");
    let observed = service.clone();
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let serving =
        tokio::spawn(async move { serve_hc2_listener(listener, service, tls, shutdown_rx).await });
    tokio::time::sleep(Duration::from_millis(50)).await;
    let baseline = process_memory()?;
    write_json(
        &control_dir.join("server-ready.json"),
        &ServerReady {
            address: address.to_string(),
        },
    )?;
    wait_for_plain_path(&control_dir.join("start-measurement")).await?;

    let (active_result, allocation) = measure_allocations(cardinality as u64, async {
        fs::write(control_dir.join("counting-ready"), [])?;
        wait_for_active(&observed, cardinality).await
    })
    .await;
    active_result?;
    let plateau = process_memory()?;
    let active = observed.accounting();
    if active.active_connections != cardinality as u64
        || active
            .accepted_connections
            .saturating_sub(active.closed_connections)
            != active.active_connections
    {
        return Err("split server plateau accounting mismatch".into());
    }
    fs::write(control_dir.join("server-plateau"), [])?;

    wait_for_zero(&observed).await?;
    tokio::time::sleep(Duration::from_millis(100)).await;
    let post_close = process_memory()?;
    let closed = observed.accounting();
    if closed.accepted_connections != closed.closed_connections {
        return Err("split server close accounting mismatch".into());
    }
    shutdown_tx.send(true)?;
    serving.await??;
    write_json(
        &control_dir.join("server-result.json"),
        &SplitServerResult {
            active_connections: active.active_connections,
            accepted_connections: active.accepted_connections,
            closed_connections_at_plateau: active.closed_connections,
            final_closed_connections: closed.closed_connections,
            profile: EndpointProfile {
                gross_allocated_bytes: allocation.gross_allocated_bytes,
                gross_allocated_bytes_per_connection: allocation
                    .gross_allocated_bytes_per_operation,
                baseline,
                plateau,
                post_close,
            },
        },
    )?;
    Ok(())
}

fn test_pki() -> Result<TestPki, Box<dyn Error>> {
    let mut ca_params = CertificateParams::new(Vec::<String>::new())?;
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    let ca = CertifiedIssuer::self_signed(ca_params, KeyPair::generate()?)?;
    let server_key = KeyPair::generate()?;
    let mut server_params = CertificateParams::new(vec!["localhost".to_owned()])?;
    server_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    let server_cert = server_params.signed_by(&server_key, &ca)?;
    let client_key = KeyPair::generate()?;
    let mut client_params = CertificateParams::new(vec!["hc2-profile".to_owned()])?;
    client_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];
    let client_cert = client_params.signed_by(&client_key, &ca)?;
    Ok(TestPki {
        ca: ca.pem(),
        server_cert: server_cert.pem(),
        server_key: server_key.serialize_pem(),
        client_cert: client_cert.pem(),
        client_key: client_key.serialize_pem(),
    })
}

fn write_tls(directory: &Path, pki: &TestPki) -> Result<(), Box<dyn Error>> {
    let cert = directory.join("server.pem");
    let key = directory.join("server.key");
    let ca = directory.join("clients.pem");
    fs::write(&cert, &pki.server_cert)?;
    fs::write(&key, &pki.server_key)?;
    fs::write(&ca, &pki.ca)?;
    Ok(())
}

fn tls_config(directory: &Path) -> TlsConfig {
    TlsConfig {
        enabled: true,
        cert_path: Some(directory.join("server.pem")),
        key_path: Some(directory.join("server.key")),
        ca_path: Some(directory.join("clients.pem")),
        acknowledge_insecure: false,
    }
}

fn adapter(addr: std::net::SocketAddr, pki: &TestPki) -> Result<GrpcMtlsAdapter, Box<dyn Error>> {
    Ok(GrpcMtlsAdapter::new(GrpcMtlsConfig::new(
        format!("https://{addr}"),
        "localhost",
        pki.ca.as_bytes(),
        pki.client_cert.as_bytes(),
        pki.client_key.as_bytes(),
    )?))
}

async fn wait_for_active(
    service: &Hc2ClientPlaneService,
    cardinality: usize,
) -> Result<(), Box<dyn Error>> {
    for _ in 0..500 {
        if service.accounting().active_connections == cardinality as u64 {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    Err("timed out waiting for active HC/2 connections".into())
}

async fn wait_for_zero(service: &Hc2ClientPlaneService) -> Result<(), Box<dyn Error>> {
    for _ in 0..500 {
        if service.accounting().live_resources_zero() {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    Err("timed out waiting for HC/2 resource reconciliation".into())
}

fn client_resources_zero(client: &&Hc2Client) -> bool {
    let retained = client.retained_state();
    retained.pending_invocations == 0
        && retained.pending_subscriptions == 0
        && retained.active_subscriptions == 0
        && retained.pending_sessions == 0
        && retained.active_sessions == 0
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), Box<dyn Error>> {
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, serde_json::to_vec(value)?)?;
    fs::rename(temporary, path)?;
    Ok(())
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, Box<dyn Error>> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

async fn wait_for_plain_path(path: &Path) -> Result<(), Box<dyn Error>> {
    for _ in 0..3_000 {
        if path.is_file() {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    Err(format!("timed out waiting for {}", path.display()).into())
}

async fn wait_for_path(path: &Path, child: &mut ChildGuard) -> Result<(), Box<dyn Error>> {
    for _ in 0..3_000 {
        if path.is_file() {
            return Ok(());
        }
        if let Some(status) = child.try_wait()? {
            return Err(format!("split server exited early with {status}").into());
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    Err(format!("timed out waiting for {}", path.display()).into())
}

async fn wait_for_json<T: DeserializeOwned>(
    path: &Path,
    child: &mut ChildGuard,
) -> Result<T, Box<dyn Error>> {
    wait_for_path(path, child).await?;
    read_json(path)
}

struct ChildGuard {
    child: Child,
}

impl ChildGuard {
    fn new(child: Child) -> Self {
        Self { child }
    }

    fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>, Box<dyn Error>> {
        Ok(self.child.try_wait()?)
    }

    async fn wait_for_exit(&mut self) -> Result<std::process::ExitStatus, Box<dyn Error>> {
        for _ in 0..3_000 {
            if let Some(status) = self.child.try_wait()? {
                return Ok(status);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Err("timed out waiting for split server exit".into())
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(windows)]
fn process_memory() -> Result<MemorySnapshot, Box<dyn Error>> {
    use std::mem::{size_of, zeroed};
    use windows_sys::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    // SAFETY: the structure is initialized with its required byte size and the
    // pseudo-handle is valid for the current process for the duration of the call.
    unsafe {
        let mut counters: PROCESS_MEMORY_COUNTERS = zeroed();
        counters.cb = size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        if GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb) == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(MemorySnapshot {
            working_set_bytes: counters.WorkingSetSize as u64,
            peak_working_set_bytes: counters.PeakWorkingSetSize as u64,
            pagefile_bytes: counters.PagefileUsage as u64,
            peak_pagefile_bytes: counters.PeakPagefileUsage as u64,
        })
    }
}

#[cfg(not(windows))]
fn process_memory() -> Result<MemorySnapshot, Box<dyn Error>> {
    Err("W5 process memory probe currently requires Windows ProcessStatus".into())
}
