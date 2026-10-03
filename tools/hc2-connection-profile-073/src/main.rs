use std::error::Error;
use std::fs::{self, File};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use hydracache_client_hc2::wire::client_envelope;
use hydracache_client_hc2::wire::client_plane_alpha_client::ClientPlaneAlphaClient;
use hydracache_client_hc2::wire::invocation_request;
use hydracache_client_hc2::wire::server_envelope;
use hydracache_client_hc2::wire::{
    Capability as WireCapability, ClientEnvelope, Handshake, InvocationRequest, PutRequest,
    RequestMeta, ServerEnvelope, Subscribe,
};
use hydracache_client_hc2::HC2_GENERATION;
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
use tokio::sync::{mpsc, watch};
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::{Certificate, ClientTlsConfig, Endpoint, Identity};
use tonic::Streaming;

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

#[derive(Deserialize, Serialize)]
struct PressureProfile {
    gross_allocated_bytes: u64,
    gross_allocated_bytes_per_mutation: f64,
    snapshot: MemorySnapshot,
}

#[derive(Deserialize, Serialize)]
struct PressureServerResult {
    active_connections: u64,
    active_subscriptions_at_pressure: u64,
    accepted_connections: u64,
    final_closed_connections: u64,
    connection_profile: EndpointProfile,
    pressure: PressureProfile,
}

#[derive(Serialize)]
struct SlowConsumerProfileResult {
    schema_version: u32,
    profile_id: &'static str,
    scenario: &'static str,
    connections: usize,
    active_subscriptions_at_pressure: u64,
    total_mutations: u64,
    client_events: u64,
    client_dropped_events: u64,
    final_closed_connections: u64,
    client_connection_profile: EndpointProfile,
    client_pressure: PressureProfile,
    server_connection_profile: EndpointProfile,
    server_pressure: PressureProfile,
}

#[derive(Deserialize, Serialize)]
struct RawTransportServerResult {
    active_connections: u64,
    active_subscriptions_at_pressure: u64,
    accepted_connections: u64,
    final_closed_connections: u64,
    dispatched_at_snapshot: u64,
    dispatched_after_stability: u64,
    state_mutations_at_snapshot: u64,
    connection_profile: EndpointProfile,
    pressure: PressureProfile,
}

#[derive(Serialize)]
struct RawTransportProfileResult {
    schema_version: u32,
    profile_id: &'static str,
    scenario: &'static str,
    connections: usize,
    subscriptions: usize,
    value_bytes: usize,
    mutations_per_connection: usize,
    offered_value_bytes_per_connection: usize,
    total_offered_mutations: u64,
    dispatched_at_snapshot: u64,
    dispatched_after_stability: u64,
    response_invocations: u64,
    response_events: u64,
    response_gaps: u64,
    final_closed_connections: u64,
    client_connection_profile: EndpointProfile,
    client_pressure: PressureProfile,
    server_connection_profile: EndpointProfile,
    server_pressure: PressureProfile,
}

#[derive(Clone, Copy)]
enum ConsumerScenario {
    Drained,
    Unread,
}

#[derive(Clone, Copy)]
enum RawTransportScenario {
    Drained,
    Unpolled,
}

impl RawTransportScenario {
    fn parse(value: &str) -> Result<Self, Box<dyn Error>> {
        match value {
            "drained" => Ok(Self::Drained),
            "unpolled" => Ok(Self::Unpolled),
            _ => Err("raw transport scenario must be drained or unpolled".into()),
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Drained => "drained-transport-reader",
            Self::Unpolled => "unpolled-transport-reader",
        }
    }
}

#[derive(Default)]
struct RawReadCounters {
    invocations: AtomicU64,
    events: AtomicU64,
    gaps: AtomicU64,
    unexpected: AtomicU64,
    errors: AtomicU64,
}

struct RawPeer {
    connection_generation: u64,
    outbound: mpsc::Sender<ClientEnvelope>,
    response: Option<Streaming<ServerEnvelope>>,
    reader: Option<tokio::task::JoinHandle<()>>,
}

impl ConsumerScenario {
    fn parse(value: &str) -> Result<Self, Box<dyn Error>> {
        match value {
            "drained" => Ok(Self::Drained),
            "unread" => Ok(Self::Unread),
            _ => Err("slow-consumer scenario must be drained or unread".into()),
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Drained => "drained-application-consumers",
            Self::Unread => "unread-application-consumers",
        }
    }
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
        [mode, scenario] if mode == "slow" => {
            run_slow_consumer(ConsumerScenario::parse(scenario)?).await
        }
        [mode, scenario, value_bytes] if mode == "raw-stall" => {
            run_raw_transport_stall(
                RawTransportScenario::parse(scenario)?,
                parse_raw_value_bytes(value_bytes)?,
            )
            .await
        }
        [mode, cardinality, control_dir] if mode == "split-server" => {
            run_split_server(parse_cardinality(cardinality)?, Path::new(control_dir)).await
        }
        [mode, cardinality, control_dir] if mode == "pressure-server" => {
            run_pressure_server(parse_cardinality(cardinality)?, Path::new(control_dir)).await
        }
        [mode, cardinality, total_mutations, control_dir] if mode == "raw-pressure-server" => {
            run_raw_pressure_server(
                cardinality.parse()?,
                total_mutations.parse()?,
                Path::new(control_dir),
            )
            .await
        }
        [cardinality] => run_combined(parse_cardinality(cardinality)?).await,
        _ => Err(
            "usage: hc2-connection-profile-073 [split] <1|10|100|1000> | slow <drained|unread> | raw-stall <drained|unpolled> <4096|262144>"
                .into(),
        ),
    }
}

fn parse_raw_value_bytes(value: &str) -> Result<usize, Box<dyn Error>> {
    let value_bytes = value.parse::<usize>()?;
    if !matches!(value_bytes, 4_096 | 262_144) {
        return Err("unsupported frozen raw transport payload size".into());
    }
    Ok(value_bytes)
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

async fn run_slow_consumer(scenario: ConsumerScenario) -> Result<(), Box<dyn Error>> {
    const CONNECTIONS: usize = 100;
    const MUTATIONS_PER_CONNECTION: usize = 1_100;
    const TOTAL_MUTATIONS: u64 = (CONNECTIONS * MUTATIONS_PER_CONNECTION) as u64;

    let _ = rustls::crypto::ring::default_provider().install_default();
    let temp = TempDir::new()?;
    let pki = test_pki()?;
    write_tls(temp.path(), &pki)?;
    let stdout_path = temp.path().join("server.stdout.txt");
    let stderr_path = temp.path().join("server.stderr.txt");
    let child = Command::new(std::env::current_exe()?)
        .arg("pressure-server")
        .arg(CONNECTIONS.to_string())
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
    let (clients_result, connection_allocation) = measure_allocations(CONNECTIONS as u64, async {
        let mut clients = Vec::with_capacity(CONNECTIONS);
        for index in 0..CONNECTIONS {
            clients.push(
                Hc2Client::connect(
                    &adapter(address, &pki)?,
                    ClientConfig::new(format!("slow-profile-{index}"), "tenant-a"),
                )
                .await?,
            );
        }
        Ok::<_, Box<dyn Error>>(clients)
    })
    .await;
    let clients = clients_result?;
    wait_for_path(&temp.path().join("server-plateau"), &mut child).await?;
    let idle = process_memory()?;

    fs::write(temp.path().join("start-pressure"), [])?;
    wait_for_path(&temp.path().join("pressure-counting-ready"), &mut child).await?;
    let (pressure_result, pressure_allocation) = measure_allocations(TOTAL_MUTATIONS, async {
        let mut unread_subscriptions = Vec::with_capacity(CONNECTIONS);
        let mut drainers = tokio::task::JoinSet::new();
        for (index, client) in clients.iter().enumerate() {
            let prefix = Bytes::from(format!("slow:{index}:"));
            let mut subscription = client.subscribe(prefix, 0).await?;
            match scenario {
                ConsumerScenario::Drained => {
                    drainers.spawn(async move { while subscription.next().await.is_some() {} });
                }
                ConsumerScenario::Unread => unread_subscriptions.push(subscription),
            }
        }

        let mut producers = tokio::task::JoinSet::new();
        for (index, client) in clients.iter().cloned().enumerate() {
            producers.spawn(async move {
                let key = Bytes::from(format!("slow:{index}:key"));
                for sequence in 0..MUTATIONS_PER_CONNECTION {
                    let mut payload = vec![0_u8; 128];
                    payload[..8].copy_from_slice(&(sequence as u64).to_be_bytes());
                    client
                        .put(key.clone(), Bytes::from(payload), None, None)
                        .await
                        .map_err(|error| error.to_string())?;
                }
                Ok::<_, String>(())
            });
        }
        while let Some(result) = producers.join_next().await {
            result.map_err(|error| error.to_string())??;
        }
        wait_for_client_pressure(&clients, scenario, TOTAL_MUTATIONS).await?;
        let (events, dropped) = client_event_totals(&clients);
        Ok::<_, Box<dyn Error>>((unread_subscriptions, drainers, events, dropped))
    })
    .await;
    let (unread_subscriptions, mut drainers, events, dropped) = pressure_result?;
    fs::write(temp.path().join("pressure-complete"), [])?;
    wait_for_path(&temp.path().join("server-pressure"), &mut child).await?;
    let pressure = process_memory()?;

    if matches!(scenario, ConsumerScenario::Drained) && dropped != 0 {
        return Err("drained slow-consumer control dropped events".into());
    }
    if matches!(scenario, ConsumerScenario::Unread) && dropped == 0 {
        return Err("unread slow-consumer treatment did not reach its bound".into());
    }
    drop(unread_subscriptions);
    drainers.abort_all();
    while drainers.join_next().await.is_some() {}
    for client in &clients {
        client.close();
    }
    drop(clients);
    tokio::time::sleep(Duration::from_millis(100)).await;
    let post_close = process_memory()?;
    let status = child.wait_for_exit().await?;
    if !status.success() {
        return Err(format!("pressure server exited with {status}").into());
    }
    if fs::metadata(&stdout_path)?.len() != 0 || fs::metadata(&stderr_path)?.len() != 0 {
        return Err("pressure server emitted unexpected stdout or stderr".into());
    }
    let server: PressureServerResult = read_json(&temp.path().join("pressure-server-result.json"))?;
    if server.active_connections != CONNECTIONS as u64
        || server.active_subscriptions_at_pressure != CONNECTIONS as u64
        || server.accepted_connections != server.final_closed_connections
    {
        return Err("pressure server accounting mismatch".into());
    }

    println!(
        "{}",
        serde_json::to_string(&SlowConsumerProfileResult {
            schema_version: 1,
            profile_id: "w5-hc2-slow-consumer-profile-073-v1",
            scenario: scenario.name(),
            connections: CONNECTIONS,
            active_subscriptions_at_pressure: server.active_subscriptions_at_pressure,
            total_mutations: TOTAL_MUTATIONS,
            client_events: events,
            client_dropped_events: dropped,
            final_closed_connections: server.final_closed_connections,
            client_connection_profile: EndpointProfile {
                gross_allocated_bytes: connection_allocation.gross_allocated_bytes,
                gross_allocated_bytes_per_connection: connection_allocation
                    .gross_allocated_bytes_per_operation,
                baseline,
                plateau: idle,
                post_close,
            },
            client_pressure: PressureProfile {
                gross_allocated_bytes: pressure_allocation.gross_allocated_bytes,
                gross_allocated_bytes_per_mutation: pressure_allocation
                    .gross_allocated_bytes_per_operation,
                snapshot: pressure,
            },
            server_connection_profile: server.connection_profile,
            server_pressure: server.pressure,
        })?
    );
    Ok(())
}

async fn run_raw_transport_stall(
    scenario: RawTransportScenario,
    value_bytes: usize,
) -> Result<(), Box<dyn Error>> {
    const CONNECTIONS: usize = 8;
    const OFFERED_VALUE_BYTES_PER_CONNECTION: usize = 8 * 1024 * 1024;
    let mutations_per_connection = OFFERED_VALUE_BYTES_PER_CONNECTION / value_bytes;
    let total_mutations = u64::try_from(CONNECTIONS * mutations_per_connection)?;

    let _ = rustls::crypto::ring::default_provider().install_default();
    let temp = TempDir::new()?;
    let pki = test_pki()?;
    write_tls(temp.path(), &pki)?;
    let stdout_path = temp.path().join("server.stdout.txt");
    let stderr_path = temp.path().join("server.stderr.txt");
    let child = Command::new(std::env::current_exe()?)
        .arg("raw-pressure-server")
        .arg(CONNECTIONS.to_string())
        .arg(total_mutations.to_string())
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
    let (peers_result, connection_allocation) = measure_allocations(CONNECTIONS as u64, async {
        let mut peers = Vec::with_capacity(CONNECTIONS);
        for index in 0..CONNECTIONS {
            peers.push(
                RawPeer::connect(
                    address,
                    &pki,
                    index,
                    mutations_per_connection.saturating_add(4),
                )
                .await?,
            );
        }
        Ok::<_, Box<dyn Error>>(peers)
    })
    .await;
    let mut peers = peers_result?;
    wait_for_path(&temp.path().join("server-plateau"), &mut child).await?;
    let idle = process_memory()?;

    let counters = Arc::new(RawReadCounters::default());
    if matches!(scenario, RawTransportScenario::Drained) {
        for peer in &mut peers {
            peer.start_reader(Arc::clone(&counters))?;
        }
    }

    fs::write(temp.path().join("start-pressure"), [])?;
    wait_for_path(&temp.path().join("pressure-counting-ready"), &mut child).await?;
    let (send_result, pressure_allocation) = measure_allocations(total_mutations, async {
        let payload = Bytes::from(vec![0x5a; value_bytes]);
        for (index, peer) in peers.iter().enumerate() {
            let key = Bytes::from(format!("raw-stall:{index}:key"));
            peer.offer_mutations(key, payload.clone(), mutations_per_connection)
                .await?;
        }
        match scenario {
            RawTransportScenario::Drained => {
                wait_for_raw_responses(&counters, total_mutations).await?;
            }
            RawTransportScenario::Unpolled => {
                tokio::time::sleep(Duration::from_millis(2_000)).await;
            }
        }
        Ok::<_, Box<dyn Error>>(())
    })
    .await;
    send_result?;
    fs::write(temp.path().join("pressure-complete"), [])?;
    wait_for_path(&temp.path().join("server-pressure"), &mut child).await?;
    let pressure = process_memory()?;
    let response_invocations_at_snapshot = counters.invocations.load(Ordering::Acquire);
    let response_events_at_snapshot = counters.events.load(Ordering::Acquire);
    let response_gaps_at_snapshot = counters.gaps.load(Ordering::Acquire);

    if matches!(scenario, RawTransportScenario::Unpolled) {
        for peer in &mut peers {
            peer.start_reader(Arc::clone(&counters))?;
        }
        wait_for_raw_responses(&counters, total_mutations).await?;
    }

    for peer in &mut peers {
        peer.stop_reader();
    }
    drop(peers);
    tokio::time::sleep(Duration::from_millis(100)).await;
    let post_close = process_memory()?;
    let status = child.wait_for_exit().await?;
    if !status.success() {
        let child_stdout = fs::read_to_string(&stdout_path).unwrap_or_default();
        let child_stderr = fs::read_to_string(&stderr_path).unwrap_or_default();
        return Err(format!(
            "raw pressure server exited with {status}; stdout={child_stdout:?}; stderr={child_stderr:?}"
        )
        .into());
    }
    if fs::metadata(&stdout_path)?.len() != 0 || fs::metadata(&stderr_path)?.len() != 0 {
        return Err("raw pressure server emitted unexpected stdout or stderr".into());
    }
    let server: RawTransportServerResult =
        read_json(&temp.path().join("raw-pressure-server-result.json"))?;
    if server.active_connections != CONNECTIONS as u64
        || server.active_subscriptions_at_pressure != CONNECTIONS as u64
        || server.accepted_connections != server.final_closed_connections
        || server.dispatched_at_snapshot != server.dispatched_after_stability
        || server.state_mutations_at_snapshot != server.dispatched_at_snapshot
    {
        return Err("raw pressure server accounting or stability mismatch".into());
    }
    match scenario {
        RawTransportScenario::Drained => {
            if server.dispatched_at_snapshot != total_mutations
                || response_invocations_at_snapshot != total_mutations
                || response_events_at_snapshot != total_mutations
            {
                return Err("drained raw transport control did not complete".into());
            }
        }
        RawTransportScenario::Unpolled => {
            if server.dispatched_at_snapshot >= total_mutations {
                return Err("unpolled raw transport treatment did not stall dispatch".into());
            }
        }
    }
    if counters.unexpected.load(Ordering::Acquire) != 0
        || counters.errors.load(Ordering::Acquire) != 0
    {
        return Err("raw response reader observed unexpected frames or errors".into());
    }

    println!(
        "{}",
        serde_json::to_string(&RawTransportProfileResult {
            schema_version: 1,
            profile_id: "w5-hc2-raw-transport-stall-073-v1",
            scenario: scenario.name(),
            connections: CONNECTIONS,
            subscriptions: CONNECTIONS,
            value_bytes,
            mutations_per_connection,
            offered_value_bytes_per_connection: OFFERED_VALUE_BYTES_PER_CONNECTION,
            total_offered_mutations: total_mutations,
            dispatched_at_snapshot: server.dispatched_at_snapshot,
            dispatched_after_stability: server.dispatched_after_stability,
            response_invocations: response_invocations_at_snapshot,
            response_events: response_events_at_snapshot,
            response_gaps: response_gaps_at_snapshot,
            final_closed_connections: server.final_closed_connections,
            client_connection_profile: EndpointProfile {
                gross_allocated_bytes: connection_allocation.gross_allocated_bytes,
                gross_allocated_bytes_per_connection: connection_allocation
                    .gross_allocated_bytes_per_operation,
                baseline,
                plateau: idle,
                post_close,
            },
            client_pressure: PressureProfile {
                gross_allocated_bytes: pressure_allocation.gross_allocated_bytes,
                gross_allocated_bytes_per_mutation: pressure_allocation
                    .gross_allocated_bytes_per_operation,
                snapshot: pressure,
            },
            server_connection_profile: server.connection_profile,
            server_pressure: server.pressure,
        })?
    );
    Ok(())
}

impl RawPeer {
    async fn connect(
        address: std::net::SocketAddr,
        pki: &TestPki,
        index: usize,
        outbound_capacity: usize,
    ) -> Result<Self, Box<dyn Error>> {
        let tls = ClientTlsConfig::new()
            .ca_certificate(Certificate::from_pem(pki.ca.as_bytes()))
            .identity(Identity::from_pem(
                pki.client_cert.as_bytes(),
                pki.client_key.as_bytes(),
            ))
            .domain_name("localhost");
        let channel = Endpoint::from_shared(format!("https://{address}"))?
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(300))
            .tls_config(tls)?
            .connect()
            .await?;
        let mut client = ClientPlaneAlphaClient::new(channel)
            .max_decoding_message_size(16 * 1024 * 1024)
            .max_encoding_message_size(16 * 1024 * 1024);
        let (outbound, receiver) = mpsc::channel(outbound_capacity);
        let mut response = client
            .open(ReceiverStream::new(receiver))
            .await?
            .into_inner();
        let connection_generation = u64::try_from(index)?.saturating_add(1);
        outbound
            .send(ClientEnvelope {
                generation: HC2_GENERATION,
                connection_generation,
                correlation_id: 1,
                message: Some(client_envelope::Message::Handshake(Handshake {
                    generation: HC2_GENERATION,
                    client_id: format!("raw-stall-{index}"),
                    requested: vec![
                        WireCapability::Data as i32,
                        WireCapability::Subscriptions as i32,
                    ],
                    connection_generation,
                })),
            })
            .await?;
        let handshake = raw_message(&mut response).await?;
        if !matches!(
            handshake.message,
            Some(server_envelope::Message::Handshake(_))
        ) {
            return Err("raw peer did not receive handshake acknowledgement".into());
        }
        outbound
            .send(ClientEnvelope {
                generation: HC2_GENERATION,
                connection_generation,
                correlation_id: 2,
                message: Some(client_envelope::Message::Subscribe(Subscribe {
                    subscription_id: 1,
                    key_prefix: Bytes::from(format!("raw-stall:{index}:")),
                    resume_watermark: 0,
                })),
            })
            .await?;
        let subscribed = raw_message(&mut response).await?;
        if !matches!(
            subscribed.message,
            Some(server_envelope::Message::Subscribed(_))
        ) {
            return Err("raw peer did not receive subscription acknowledgement".into());
        }
        Ok(Self {
            connection_generation,
            outbound,
            response: Some(response),
            reader: None,
        })
    }

    fn start_reader(&mut self, counters: Arc<RawReadCounters>) -> Result<(), Box<dyn Error>> {
        let mut response = self
            .response
            .take()
            .ok_or("raw response stream is unavailable")?;
        self.reader = Some(tokio::spawn(async move {
            loop {
                match response.message().await {
                    Ok(Some(envelope)) => match envelope.message {
                        Some(server_envelope::Message::Invocation(_)) => {
                            counters.invocations.fetch_add(1, Ordering::AcqRel);
                        }
                        Some(server_envelope::Message::Event(_)) => {
                            counters.events.fetch_add(1, Ordering::AcqRel);
                        }
                        Some(server_envelope::Message::Gap(_)) => {
                            counters.gaps.fetch_add(1, Ordering::AcqRel);
                        }
                        _ => {
                            counters.unexpected.fetch_add(1, Ordering::AcqRel);
                        }
                    },
                    Ok(None) => break,
                    Err(_) => {
                        counters.errors.fetch_add(1, Ordering::AcqRel);
                        break;
                    }
                }
            }
        }));
        Ok(())
    }

    async fn offer_mutations(
        &self,
        key: Bytes,
        value: Bytes,
        mutations: usize,
    ) -> Result<(), Box<dyn Error>> {
        for sequence in 0..mutations {
            self.outbound
                .send(ClientEnvelope {
                    generation: HC2_GENERATION,
                    connection_generation: self.connection_generation,
                    correlation_id: u64::try_from(sequence)?.saturating_add(3),
                    message: Some(client_envelope::Message::Invocation(InvocationRequest {
                        meta: Some(RequestMeta {
                            deadline_unix_ms: 0,
                            idempotency_key: Bytes::new(),
                            tenant: "tenant-a".to_owned(),
                            topology_epoch: 1,
                        }),
                        operation: Some(invocation_request::Operation::Put(PutRequest {
                            key: key.clone(),
                            value: value.clone(),
                            ttl_ms: 0,
                        })),
                    })),
                })
                .await?;
        }
        Ok(())
    }

    fn stop_reader(&mut self) {
        if let Some(reader) = self.reader.take() {
            reader.abort();
        }
    }
}

async fn raw_message(
    response: &mut Streaming<ServerEnvelope>,
) -> Result<ServerEnvelope, Box<dyn Error>> {
    match tokio::time::timeout(Duration::from_secs(5), response.message()).await {
        Ok(Ok(Some(envelope))) => Ok(envelope),
        Ok(Ok(None)) => Err("raw response stream ended".into()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("timed out waiting for raw response frame".into()),
    }
}

async fn wait_for_raw_responses(
    counters: &RawReadCounters,
    expected: u64,
) -> Result<(), Box<dyn Error>> {
    for _ in 0..6_000 {
        if counters.invocations.load(Ordering::Acquire) == expected
            && counters.events.load(Ordering::Acquire) == expected
        {
            return Ok(());
        }
        if counters.unexpected.load(Ordering::Acquire) != 0
            || counters.errors.load(Ordering::Acquire) != 0
        {
            return Err(format!(
                "raw response reader failed before completion: invocations={}, events={}, gaps={}, unexpected={}, errors={}",
                counters.invocations.load(Ordering::Acquire),
                counters.events.load(Ordering::Acquire),
                counters.gaps.load(Ordering::Acquire),
                counters.unexpected.load(Ordering::Acquire),
                counters.errors.load(Ordering::Acquire),
            )
            .into());
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    Err("timed out waiting for drained raw responses".into())
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

async fn run_pressure_server(cardinality: usize, control_dir: &Path) -> Result<(), Box<dyn Error>> {
    const TOTAL_MUTATIONS: u64 = 110_000;
    let _ = rustls::crypto::ring::default_provider().install_default();
    let tls = Hc2ListenerTls::from_server_config(&tls_config(control_dir))?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let state = std::sync::Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default())?);
    let service = Hc2ClientPlaneService::new(state, "slow-profile-cluster");
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
    let (active_result, connection_allocation) = measure_allocations(cardinality as u64, async {
        fs::write(control_dir.join("counting-ready"), [])?;
        wait_for_active(&observed, cardinality).await
    })
    .await;
    active_result?;
    let idle = process_memory()?;
    let active = observed.accounting();
    fs::write(control_dir.join("server-plateau"), [])?;

    wait_for_plain_path(&control_dir.join("start-pressure")).await?;
    let (pressure_result, pressure_allocation) = measure_allocations(TOTAL_MUTATIONS, async {
        fs::write(control_dir.join("pressure-counting-ready"), [])?;
        wait_for_plain_path(&control_dir.join("pressure-complete")).await
    })
    .await;
    pressure_result?;
    let pressure = process_memory()?;
    let pressured = observed.accounting();
    if pressured.active_connections != cardinality as u64
        || pressured.active_subscriptions != cardinality as u64
        || pressured.pending_invocations != 0
    {
        return Err("pressure server live accounting mismatch".into());
    }
    fs::write(control_dir.join("server-pressure"), [])?;

    wait_for_zero(&observed).await?;
    tokio::time::sleep(Duration::from_millis(100)).await;
    let post_close = process_memory()?;
    let closed = observed.accounting();
    if closed.accepted_connections != closed.closed_connections {
        return Err("pressure server close accounting mismatch".into());
    }
    shutdown_tx.send(true)?;
    serving.await??;
    write_json(
        &control_dir.join("pressure-server-result.json"),
        &PressureServerResult {
            active_connections: active.active_connections,
            active_subscriptions_at_pressure: pressured.active_subscriptions,
            accepted_connections: active.accepted_connections,
            final_closed_connections: closed.closed_connections,
            connection_profile: EndpointProfile {
                gross_allocated_bytes: connection_allocation.gross_allocated_bytes,
                gross_allocated_bytes_per_connection: connection_allocation
                    .gross_allocated_bytes_per_operation,
                baseline,
                plateau: idle,
                post_close,
            },
            pressure: PressureProfile {
                gross_allocated_bytes: pressure_allocation.gross_allocated_bytes,
                gross_allocated_bytes_per_mutation: pressure_allocation
                    .gross_allocated_bytes_per_operation,
                snapshot: pressure,
            },
        },
    )?;
    Ok(())
}

async fn run_raw_pressure_server(
    cardinality: usize,
    total_mutations: u64,
    control_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let tls = Hc2ListenerTls::from_server_config(&tls_config(control_dir))?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let state = Arc::new(ClientSurfaceState::new(ClientSurfaceLimits::default())?);
    let service = Hc2ClientPlaneService::new(Arc::clone(&state), "raw-stall-profile-cluster");
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
    let (active_result, connection_allocation) = measure_allocations(cardinality as u64, async {
        fs::write(control_dir.join("counting-ready"), [])?;
        wait_for_active(&observed, cardinality).await
    })
    .await;
    active_result?;
    let idle = process_memory()?;
    let active = observed.accounting();
    fs::write(control_dir.join("server-plateau"), [])?;

    wait_for_plain_path(&control_dir.join("start-pressure")).await?;
    let (pressure_result, pressure_allocation) = measure_allocations(total_mutations, async {
        fs::write(control_dir.join("pressure-counting-ready"), [])?;
        wait_for_plain_path(&control_dir.join("pressure-complete")).await
    })
    .await;
    pressure_result?;
    let dispatched_at_snapshot = state.dispatch_attempts();
    let state_mutations_at_snapshot = state.state_mutations();
    tokio::time::sleep(Duration::from_millis(500)).await;
    let dispatched_after_stability = state.dispatch_attempts();
    let pressure = process_memory()?;
    let pressured = observed.accounting();
    if pressured.active_connections != cardinality as u64
        || pressured.active_subscriptions != cardinality as u64
        || pressured.pending_invocations != 0
    {
        return Err("raw pressure server live accounting mismatch".into());
    }
    fs::write(control_dir.join("server-pressure"), [])?;

    wait_for_zero(&observed).await?;
    tokio::time::sleep(Duration::from_millis(100)).await;
    let post_close = process_memory()?;
    let closed = observed.accounting();
    if closed.accepted_connections != closed.closed_connections {
        return Err("raw pressure server close accounting mismatch".into());
    }
    shutdown_tx.send(true)?;
    serving.await??;
    write_json(
        &control_dir.join("raw-pressure-server-result.json"),
        &RawTransportServerResult {
            active_connections: active.active_connections,
            active_subscriptions_at_pressure: pressured.active_subscriptions,
            accepted_connections: active.accepted_connections,
            final_closed_connections: closed.closed_connections,
            dispatched_at_snapshot,
            dispatched_after_stability,
            state_mutations_at_snapshot,
            connection_profile: EndpointProfile {
                gross_allocated_bytes: connection_allocation.gross_allocated_bytes,
                gross_allocated_bytes_per_connection: connection_allocation
                    .gross_allocated_bytes_per_operation,
                baseline,
                plateau: idle,
                post_close,
            },
            pressure: PressureProfile {
                gross_allocated_bytes: pressure_allocation.gross_allocated_bytes,
                gross_allocated_bytes_per_mutation: pressure_allocation
                    .gross_allocated_bytes_per_operation,
                snapshot: pressure,
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

fn client_event_totals(clients: &[Hc2Client]) -> (u64, u64) {
    clients.iter().fold((0, 0), |(events, dropped), client| {
        let metrics = client.metrics();
        (
            events.saturating_add(metrics.events),
            dropped.saturating_add(metrics.dropped_events),
        )
    })
}

async fn wait_for_client_pressure(
    clients: &[Hc2Client],
    scenario: ConsumerScenario,
    total_mutations: u64,
) -> Result<(), Box<dyn Error>> {
    for _ in 0..3_000 {
        let mut submitted = 0_u64;
        let mut completed = 0_u64;
        let mut failed = 0_u64;
        let mut cancelled = 0_u64;
        let mut active_subscriptions = 0_usize;
        for client in clients {
            let metrics = client.metrics();
            submitted = submitted.saturating_add(metrics.submitted);
            completed = completed.saturating_add(metrics.completed);
            failed = failed.saturating_add(metrics.failed);
            cancelled = cancelled.saturating_add(metrics.cancelled);
            active_subscriptions =
                active_subscriptions.saturating_add(client.retained_state().active_subscriptions);
        }
        let (events, dropped) = client_event_totals(clients);
        let scenario_ready = match scenario {
            ConsumerScenario::Drained => events == total_mutations && dropped == 0,
            ConsumerScenario::Unread => events > 0 && dropped > 0,
        };
        if submitted == total_mutations
            && completed == total_mutations
            && failed == 0
            && cancelled == 0
            && active_subscriptions == clients.len()
            && scenario_ready
        {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    Err("timed out waiting for client pressure invariants".into())
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
