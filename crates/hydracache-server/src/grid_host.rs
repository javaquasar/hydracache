use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt;
use std::fs;
use std::future::Future;
use std::net::{SocketAddr, TcpListener as StdTcpListener};
use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Once, RwLock};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum_server::tls_rustls::RustlsConfig;
use hydracache::{
    CacheError, CacheResult, ClusterAdmissionBridge, ClusterCandidate, ClusterDiscovery,
    ClusterDiscoveryLiveness, ClusterEndpoints, ClusterGeneration, ClusterMember, ClusterNodeId,
    ClusterRole, HydraCache, RaftMetadataCommand, RaftMetadataSnapshot,
    RaftStyleMetadataControlPlane,
};
use hydracache_cluster_chitchat::{ChitchatDiscovery, ChitchatDiscoveryConfig};
use hydracache_cluster_raft::{
    RaftMessageSink, RaftMetadataRuntime, RaftMetadataRuntimeConfig, RaftWireMessage,
    SledRaftLogStore,
};
use hydracache_cluster_transport_axum::{
    tls::TlsStartupPolicy, AllowAllAuthorizer, AxumClusterMessageService, ClusterMessageAck,
    ClusterMessageHandler, ClusterOpaqueMessage, ClusterRoute, ClusterRouteAuth,
    StaticNodeIdentityProvider, DEFAULT_RAFT_APPEND_PATH,
};
use serde::{Deserialize, Serialize};
use tokio::sync::watch;

use crate::cluster_status::{
    GridControlPlaneHandle, RaftCompactionError, RaftCompactionStatus, Reachability, ReshardPhase,
};
use crate::config::{ClusterStartMode, ServerConfig, ServerConfigError};

const DEFAULT_CLUSTER_NAME: &str = "hydracache";
const GRID_INPROC_ENV: &str = "HYDRACACHE_GRID_INPROC";
const GRID_DRIVE_INTERVAL: Duration = Duration::from_millis(50);
const GRID_LEADER_WAIT_TIMEOUT: Duration = Duration::from_secs(5);
const GRID_RUNTIME_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(1);
const RAFT_HTTP_CONNECT_TIMEOUT: Duration = Duration::from_millis(500);
const RAFT_HTTP_REQUEST_TIMEOUT: Duration = Duration::from_secs(2);
const NODE_IDENTITY_FILE: &str = "node-identity.json";
const NODE_IDENTITY_FORMAT_VERSION: u32 = 1;
static NODE_IDENTITY_TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

type NetworkedRaftRuntime = RaftMetadataRuntime<SledRaftLogStore>;
type SharedRaftPeers = Arc<RwLock<BTreeMap<u64, RaftPeer>>>;
type SharedRaftVoterSet = Arc<RwLock<BTreeSet<u64>>>;

/// Build the grid-mode cache used by a member-role daemon.
pub(crate) fn build_member(
    config: &ServerConfig,
) -> Result<(HydraCache, Arc<dyn GridControlPlaneHandle>), ServerConfigError> {
    if use_inprocess_grid() {
        return build_inprocess_member(config);
    }
    build_networked_member(config)
}

fn build_inprocess_member(
    config: &ServerConfig,
) -> Result<(HydraCache, Arc<dyn GridControlPlaneHandle>), ServerConfigError> {
    let control_plane =
        Arc::new(RaftStyleMetadataControlPlane::new(DEFAULT_CLUSTER_NAME).with_term(1));
    let (cache, runtime) = start_inprocess_member_cache(config, control_plane.clone())?;
    Ok((
        cache,
        Arc::new(InProcessGridHandle::new(control_plane, runtime)),
    ))
}

fn build_networked_member(
    config: &ServerConfig,
) -> Result<(HydraCache, Arc<dyn GridControlPlaneHandle>), ServerConfigError> {
    let (stack, runtime) = start_networked_member_stack(config)?;
    let cache = stack.cache.clone();
    Ok((
        cache,
        Arc::new(NetworkedGridHandle::new(stack, Some(runtime))),
    ))
}

fn start_networked_member_stack(
    config: &ServerConfig,
) -> Result<(NetworkedMemberStack, DedicatedGridRuntime), ServerConfigError> {
    if tokio::runtime::Handle::try_current().is_ok() {
        let config = config.clone();
        return std::thread::spawn(move || start_networked_member_stack_without_current(&config))
            .join()
            .map_err(|_| {
                ServerConfigError::GridHostStart(
                    "networked member startup helper thread panicked".to_owned(),
                )
            })?;
    }

    start_networked_member_stack_without_current(config)
}

fn start_networked_member_stack_without_current(
    config: &ServerConfig,
) -> Result<(NetworkedMemberStack, DedicatedGridRuntime), ServerConfigError> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .thread_name("hydracache-grid-host")
        .enable_all()
        .build()
        .map_err(|error| ServerConfigError::GridHostStart(error.to_string()))?;
    let stack = runtime
        .block_on(networked_member_stack(config))
        .map_err(|error| ServerConfigError::GridHostStart(error.to_string()))?;
    Ok((stack, DedicatedGridRuntime::new(runtime)))
}

async fn networked_member_stack(config: &ServerConfig) -> CacheResult<NetworkedMemberStack> {
    let storage_dir = config.storage_dir.as_ref().ok_or_else(|| {
        CacheError::Backend("member role requires storage_dir before grid host startup".to_owned())
    })?;
    let identity = resolve_member_identity(config, storage_dir)?;
    let node_id = identity.node_id;
    let raft_node_id = identity.raft_node_id;
    let topology = raft_topology(config, node_id.clone(), raft_node_id)?;
    let raft_log_dir = storage_dir.join("raft-log");
    fs::create_dir_all(&raft_log_dir).map_err(|error| {
        CacheError::Backend(format!(
            "failed to create raft log directory {}: {error}",
            raft_log_dir.display()
        ))
    })?;
    let start_mode = resolved_start_mode(config, &raft_log_dir)?;

    let raft_config = match start_mode {
        ResolvedClusterStartMode::Bootstrap => RaftMetadataRuntimeConfig::multi_voter(
            DEFAULT_CLUSTER_NAME,
            raft_node_id,
            topology.voters.clone(),
        )
        .auto_campaign(!topology.multi_voter)
        .ticks(topology.election_tick_for(raft_node_id), 1),
        ResolvedClusterStartMode::Join => RaftMetadataRuntimeConfig::try_joining(
            DEFAULT_CLUSTER_NAME,
            raft_node_id,
            topology.remote_voters(raft_node_id),
        )?
        .ticks(topology.joiner_election_tick(), 1),
    };
    let raft = Arc::new(RaftMetadataRuntime::sled_with_config(
        raft_config,
        &raft_log_dir,
    )?);
    let generation = next_member_generation(&raft, &node_id);
    let discovery = Arc::new(
        ChitchatDiscovery::spawn_udp(
            ChitchatDiscoveryConfig::new(
                DEFAULT_CLUSTER_NAME,
                node_id.clone(),
                generation,
                config.cluster_addr,
            )
            .seed_nodes(config.seeds.clone()),
        )
        .await?,
    );
    if matches!(start_mode, ResolvedClusterStartMode::Join) {
        announce_join_candidate(&discovery, node_id.clone(), generation, config).await?;
    }
    let bridge = ClusterAdmissionBridge::new(discovery.clone(), raft.clone());
    let route_auth = cluster_route_auth(config, &node_id)?;
    let raft_peers = Arc::new(RwLock::new((*topology.peers).clone()));
    let last_voters = Arc::new(RwLock::new(
        raft.voter_ids()?.into_iter().collect::<BTreeSet<_>>(),
    ));
    let suppressed_raft_promotions = Arc::new(RwLock::new(BTreeSet::new()));
    let use_network_sink =
        topology.multi_voter || matches!(start_mode, ResolvedClusterStartMode::Join);
    let drive_diagnostics = Arc::new(GridDriveDiagnostics::default());
    let message_sink: Arc<dyn RaftMessageSink> = if use_network_sink {
        Arc::new(
            HttpRaftMessageSink::new(
                node_id.clone(),
                raft_node_id,
                raft_peers.clone(),
                route_auth.clone(),
                config,
            )?
            .with_snapshot_feedback(raft.clone(), drive_diagnostics.clone()),
        )
    } else {
        Arc::new(NoopRaftMessageSink)
    };
    let (shutdown, _) = watch::channel(false);
    spawn_grid_drive(
        GridDriveHandles {
            raft: raft.clone(),
            message_sink: message_sink.clone(),
            raft_peers: raft_peers.clone(),
            diagnostics: drive_diagnostics.clone(),
            last_voters,
            suppressed_raft_promotions,
            local_node_id: node_id.clone(),
            local_endpoint: config.cluster_advertise_endpoint(),
            local_generation: generation,
        },
        discovery.clone(),
        shutdown.subscribe(),
    );
    spawn_admission_drive(bridge.clone(), shutdown.subscribe());
    spawn_cluster_transport(
        config,
        node_id.clone(),
        raft.clone(),
        message_sink.clone(),
        raft_peers.clone(),
        route_auth,
        shutdown.subscribe(),
    )
    .await?;
    match start_mode {
        ResolvedClusterStartMode::Bootstrap if use_network_sink => {
            if raft_node_id == topology.bootstrap_raft_node_id {
                let _ = send_raft_messages(&message_sink, raft.campaign()?).await;
            }
            wait_for_raft_leader(&raft).await?;
        }
        ResolvedClusterStartMode::Join => {
            wait_for_join_complete(&raft, raft_node_id, config.join_timeout()).await?;
        }
        ResolvedClusterStartMode::Bootstrap => {}
    }

    let cache = networked_member_cache(
        config,
        raft.clone(),
        discovery.clone(),
        node_id.clone(),
        generation,
    )
    .await?;

    Ok(NetworkedMemberStack {
        cache,
        node_id,
        raft_node_id,
        raft_peers,
        raft,
        discovery,
        bridge,
        message_sink,
        drive_diagnostics,
        draining: Arc::new(AtomicBool::new(false)),
        drain_remove_proposed: Arc::new(AtomicBool::new(false)),
        raft_compaction_enabled: config.raft_compaction_enabled,
        shutdown,
    })
}

fn next_member_generation(
    raft: &NetworkedRaftRuntime,
    node_id: &ClusterNodeId,
) -> ClusterGeneration {
    let materialized = raft
        .members()
        .into_iter()
        .filter_map(|member| (member.node_id == *node_id).then_some(member.generation));
    let retained = raft
        .commands()
        .into_iter()
        .filter_map(|command| match command {
            RaftMetadataCommand::MemberUpsert {
                node_id: command_node_id,
                generation,
                ..
            } if command_node_id == *node_id => Some(generation),
            _ => None,
        });
    materialized
        .chain(retained)
        .max()
        .map(ClusterGeneration::next)
        .unwrap_or_else(|| ClusterGeneration::new(1))
}

async fn networked_member_cache(
    config: &ServerConfig,
    raft: Arc<NetworkedRaftRuntime>,
    discovery: Arc<ChitchatDiscovery>,
    node_id: ClusterNodeId,
    generation: ClusterGeneration,
) -> CacheResult<HydraCache> {
    let mut builder = HydraCache::member()
        .cluster(DEFAULT_CLUSTER_NAME)
        .control_plane(raft)
        .discovery(discovery)
        .node_id(node_id.clone())
        .generation(generation)
        .bind(config.cluster_addr.to_string())
        .diagnostics_endpoint(format!("http://{}", config.admin_api.listen_addr));
    for seed in &config.seeds {
        builder = builder.bootstrap(seed.clone());
    }
    builder.start().await.map_err(|error| {
        CacheError::Backend(format!(
            "failed to admit local member {node_id} at generation {}: {error}",
            generation.value()
        ))
    })
}

async fn inprocess_member_cache(
    config: &ServerConfig,
    control_plane: Arc<RaftStyleMetadataControlPlane>,
) -> hydracache::CacheResult<HydraCache> {
    let mut builder = HydraCache::member()
        .cluster(DEFAULT_CLUSTER_NAME)
        .control_plane(control_plane.clone())
        .node_id(member_node_id(config))
        .generation(ClusterGeneration::new(1))
        .bind(config.cluster_addr.to_string())
        .diagnostics_endpoint(format!("http://{}", config.admin_api.listen_addr));
    for seed in &config.seeds {
        builder = builder.bootstrap(seed.clone());
    }
    builder.start().await
}

fn start_inprocess_member_cache(
    config: &ServerConfig,
    control_plane: Arc<RaftStyleMetadataControlPlane>,
) -> Result<(HydraCache, Option<Arc<tokio::runtime::Runtime>>), ServerConfigError> {
    if tokio::runtime::Handle::try_current().is_ok() {
        let cache = poll_immediate(inprocess_member_cache(config, control_plane))?
            .map_err(|error| ServerConfigError::GridHostStart(error.to_string()))?;
        return Ok((cache, None));
    }

    let runtime = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .thread_name("hydracache-grid-host")
            .enable_all()
            .build()
            .map_err(|error| ServerConfigError::GridHostStart(error.to_string()))?,
    );
    let cache = runtime
        .block_on(inprocess_member_cache(config, control_plane))
        .map_err(|error| ServerConfigError::GridHostStart(error.to_string()))?;
    Ok((cache, Some(runtime)))
}

async fn spawn_cluster_transport(
    config: &ServerConfig,
    node_id: ClusterNodeId,
    raft: Arc<NetworkedRaftRuntime>,
    message_sink: Arc<dyn RaftMessageSink>,
    raft_peers: SharedRaftPeers,
    auth: ClusterRouteAuth,
    mut shutdown: watch::Receiver<bool>,
) -> CacheResult<()> {
    TlsStartupPolicy::new(config.cluster_addr, config.tls.enabled)
        .acknowledge_insecure(config.tls.acknowledge_insecure)
        .validate()
        .map_err(|error| CacheError::Backend(error.to_string()))?;
    let listener = StdTcpListener::bind(config.cluster_addr).map_err(|error| {
        CacheError::Backend(format!("failed to bind cluster transport: {error}"))
    })?;
    listener.set_nonblocking(true).map_err(|error| {
        CacheError::Backend(format!(
            "failed to configure cluster transport listener: {error}"
        ))
    })?;
    let routes = AxumClusterMessageService::new(
        node_id.clone(),
        Arc::new(RaftClusterMessageHandler {
            node_id: node_id.clone(),
            raft_node_id: raft_node_id(&node_id),
            raft,
            message_sink,
            raft_peers,
        }),
        auth,
    )
    .routes();
    if config.tls.enabled {
        install_default_rustls_provider();
        let cert_path = config
            .tls
            .cert_path
            .as_deref()
            .ok_or_else(|| CacheError::Backend("tls.enabled requires cert_path".to_owned()))?;
        let key_path = config
            .tls
            .key_path
            .as_deref()
            .ok_or_else(|| CacheError::Backend("tls.enabled requires key_path".to_owned()))?;
        let rustls_config = RustlsConfig::from_pem_file(cert_path, key_path)
            .await
            .map_err(|error| {
                CacheError::Backend(format!(
                    "failed to load cluster TLS cert/key {} / {}: {error}",
                    cert_path.display(),
                    key_path.display()
                ))
            })?;
        let server = axum_server::from_tcp_rustls(listener, rustls_config).map_err(|error| {
            CacheError::Backend(format!("failed to start TLS cluster transport: {error}"))
        })?;
        tokio::spawn(async move {
            let handle = axum_server::Handle::new();
            let shutdown_handle = handle.clone();
            tokio::spawn(async move {
                wait_for_shutdown(&mut shutdown).await;
                shutdown_handle.graceful_shutdown(None);
            });
            let _ = server
                .handle(handle)
                .serve(routes.into_make_service())
                .await;
        });
    } else {
        let server = axum_server::from_tcp(listener).map_err(|error| {
            CacheError::Backend(format!("failed to start cluster transport: {error}"))
        })?;
        tokio::spawn(async move {
            let handle = axum_server::Handle::new();
            let shutdown_handle = handle.clone();
            tokio::spawn(async move {
                wait_for_shutdown(&mut shutdown).await;
                shutdown_handle.graceful_shutdown(None);
            });
            let _ = server
                .handle(handle)
                .serve(routes.into_make_service())
                .await;
        });
    }
    Ok(())
}

async fn wait_for_shutdown(shutdown: &mut watch::Receiver<bool>) {
    loop {
        if *shutdown.borrow() {
            break;
        }
        if shutdown.changed().await.is_err() {
            break;
        }
    }
}

fn cluster_route_auth(
    config: &ServerConfig,
    node_id: &ClusterNodeId,
) -> CacheResult<ClusterRouteAuth> {
    if let Some(identity) = cluster_auth_provider(config, node_id)? {
        return Ok(ClusterRouteAuth::secure(
            Arc::new(identity),
            Arc::new(AllowAllAuthorizer),
        ));
    }
    if config.tls.enabled {
        return Err(CacheError::Backend(
            "tls.enabled member requires [cluster_auth]: a TLS listener without peer auth rejects every inbound raft message and the cluster cannot form"
                .to_owned(),
        ));
    }
    Ok(
        ClusterRouteAuth::missing_provider().acknowledge_insecure_trust_boundary(
            config.cluster_addr.ip().is_loopback() || config.tls.acknowledge_insecure,
        ),
    )
}

fn cluster_auth_provider(
    config: &ServerConfig,
    node_id: &ClusterNodeId,
) -> CacheResult<Option<StaticNodeIdentityProvider>> {
    let Some(key_id) = config.cluster_auth.key_id.as_deref() else {
        return Ok(None);
    };
    let token_file = config.cluster_auth.token_file.as_deref().ok_or_else(|| {
        CacheError::Backend("cluster_auth requires key_id and readable token_file".to_owned())
    })?;
    let token = read_cluster_auth_token(token_file, "cluster_auth")?;
    let mut provider = StaticNodeIdentityProvider::new(node_id.clone(), key_id, token);
    if let Some(previous_key_id) = config.cluster_auth.previous_key_id.as_deref() {
        let previous_token_file = config
            .cluster_auth
            .previous_token_file
            .as_deref()
            .ok_or_else(|| {
                CacheError::Backend(
                    "cluster_auth.previous requires key_id and readable token_file".to_owned(),
                )
            })?;
        let previous_token = read_cluster_auth_token(previous_token_file, "cluster_auth.previous")?;
        provider = provider.with_previous(previous_key_id, previous_token);
    }
    Ok(Some(provider))
}

fn read_cluster_auth_token(path: &Path, section: &str) -> CacheResult<String> {
    let token = fs::read_to_string(path).map_err(|error| {
        CacheError::Backend(format!(
            "failed to read {section}.token_file {}: {error}",
            path.display()
        ))
    })?;
    let token = token.trim_end_matches(['\r', '\n']).to_owned();
    if token.trim().is_empty() {
        return Err(CacheError::Backend(format!(
            "{section}.token_file {} is empty",
            path.display()
        )));
    }
    Ok(token)
}

async fn announce_join_candidate(
    discovery: &ChitchatDiscovery,
    node_id: ClusterNodeId,
    generation: ClusterGeneration,
    config: &ServerConfig,
) -> CacheResult<()> {
    discovery
        .announce(
            ClusterCandidate::member(node_id)
                .generation(generation)
                .endpoints(ClusterEndpoints::new().control(config.cluster_advertise_endpoint())),
        )
        .await
}

#[derive(Clone)]
struct GridDriveHandles {
    raft: Arc<NetworkedRaftRuntime>,
    message_sink: Arc<dyn RaftMessageSink>,
    raft_peers: SharedRaftPeers,
    diagnostics: Arc<GridDriveDiagnostics>,
    last_voters: SharedRaftVoterSet,
    suppressed_raft_promotions: SharedRaftVoterSet,
    local_node_id: ClusterNodeId,
    local_endpoint: String,
    local_generation: ClusterGeneration,
}

fn spawn_grid_drive(
    handles: GridDriveHandles,
    discovery: Arc<ChitchatDiscovery>,
    mut shutdown: watch::Receiver<bool>,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(GRID_DRIVE_INTERVAL);
        loop {
            tokio::select! {
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        break;
                    }
                }
                _ = interval.tick() => {
                    let candidates = discovery.candidates();
                    if let Err(_error) = drive_grid_once(
                        &handles,
                        &candidates,
                    ).await {
                        handles.diagnostics.record_drive_error(_error.to_string());
                        continue;
                    }
                }
            }
        }
    });
}

fn spawn_admission_drive(bridge: ClusterAdmissionBridge, mut shutdown: watch::Receiver<bool>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(GRID_DRIVE_INTERVAL);
        loop {
            tokio::select! {
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        break;
                    }
                }
                _ = interval.tick() => {
                    let _ = bridge.run_once().await;
                }
            }
        }
    });
}

async fn drive_grid_once(
    handles: &GridDriveHandles,
    candidates: &[ClusterCandidate],
) -> CacheResult<()> {
    handles.diagnostics.record_tick();
    let _ = send_raft_messages_with_diagnostics(
        &handles.message_sink,
        handles.raft.tick()?,
        Some(handles.diagnostics.as_ref()),
    )
    .await;
    refresh_raft_peers(
        &handles.raft_peers,
        &handles.local_node_id,
        handles.local_endpoint.clone(),
        &handles.raft.members(),
        handles.local_generation,
        candidates,
    );
    refresh_suppressed_raft_promotions(
        &handles.last_voters,
        &handles.suppressed_raft_promotions,
        &handles.raft.voter_ids()?,
        &handles.raft.members(),
    );
    sync_raft_voters(
        &handles.raft,
        &handles.message_sink,
        &handles.raft_peers,
        handles.diagnostics.as_ref(),
        &handles.suppressed_raft_promotions,
    )
    .await?;
    let _ = send_raft_messages_with_diagnostics(
        &handles.message_sink,
        handles.raft.take_outbound_messages(),
        Some(handles.diagnostics.as_ref()),
    )
    .await;
    let _ = send_raft_messages_with_diagnostics(
        &handles.message_sink,
        handles.raft.drain_ready()?,
        Some(handles.diagnostics.as_ref()),
    )
    .await;
    Ok(())
}

fn refresh_raft_peers(
    raft_peers: &SharedRaftPeers,
    local_node_id: &ClusterNodeId,
    local_endpoint: String,
    members: &[ClusterMember],
    local_generation: ClusterGeneration,
    candidates: &[ClusterCandidate],
) {
    let mut peers = raft_peers.write().expect("raft peer map poisoned");
    peers.insert(
        raft_node_id(local_node_id),
        RaftPeer {
            node_id: local_node_id.clone(),
            endpoint: local_endpoint,
        },
    );
    for member in members {
        if !member.is_member() {
            continue;
        }
        let Some(endpoint) = member
            .endpoints
            .control
            .as_deref()
            .and_then(valid_raft_endpoint)
        else {
            continue;
        };
        peers.insert(
            raft_node_id(&member.node_id),
            RaftPeer {
                node_id: member.node_id.clone(),
                endpoint,
            },
        );
    }
    for candidate in candidates {
        if candidate.role != ClusterRole::Member || candidate.generation != local_generation {
            continue;
        }
        let Some(endpoint) = candidate
            .endpoints
            .control
            .as_deref()
            .and_then(valid_raft_endpoint)
        else {
            continue;
        };
        peers
            .entry(raft_node_id(&candidate.node_id))
            .or_insert_with(|| RaftPeer {
                node_id: candidate.node_id.clone(),
                endpoint,
            });
    }
}

fn refresh_suppressed_raft_promotions(
    last_voters: &SharedRaftVoterSet,
    suppressed_raft_promotions: &SharedRaftVoterSet,
    current_voters: &[u64],
    members: &[ClusterMember],
) {
    let current_voters = current_voters.iter().copied().collect::<BTreeSet<_>>();
    let materialized_member_voters = members
        .iter()
        .filter(|member| member.is_member())
        .map(|member| raft_node_id(&member.node_id))
        .collect::<BTreeSet<_>>();

    let mut last_voters = last_voters.write().expect("last raft voters poisoned");
    let mut suppressed = suppressed_raft_promotions
        .write()
        .expect("suppressed raft promotions poisoned");
    suppressed.retain(|raft_id| materialized_member_voters.contains(raft_id));
    for removed in last_voters.difference(&current_voters) {
        if materialized_member_voters.contains(removed) {
            suppressed.insert(*removed);
        }
    }
    *last_voters = current_voters;
}

async fn sync_raft_voters(
    raft: &Arc<NetworkedRaftRuntime>,
    message_sink: &Arc<dyn RaftMessageSink>,
    raft_peers: &SharedRaftPeers,
    diagnostics: &GridDriveDiagnostics,
    suppressed_raft_promotions: &SharedRaftVoterSet,
) -> CacheResult<()> {
    let snapshot = raft.snapshot();
    if raft.leader_id() != Some(snapshot.raft_node_id) {
        return Ok(());
    }
    let current_voters = raft.voter_ids()?;
    let suppressed = suppressed_raft_promotions
        .read()
        .expect("suppressed raft promotions poisoned")
        .clone();
    for member in raft.members() {
        if !member.is_member() {
            continue;
        }
        let raft_id = raft_node_id(&member.node_id);
        if current_voters.contains(&raft_id) {
            continue;
        }
        if suppressed.contains(&raft_id) {
            continue;
        }
        if !raft_peers
            .read()
            .expect("raft peer map poisoned")
            .contains_key(&raft_id)
        {
            continue;
        }
        let outbound = raft.propose_add_voter(raft_id)?;
        send_raft_messages_with_diagnostics(message_sink, outbound, Some(diagnostics)).await?;
        break;
    }
    Ok(())
}

async fn send_raft_messages(
    message_sink: &Arc<dyn RaftMessageSink>,
    messages: Vec<RaftWireMessage>,
) -> CacheResult<()> {
    send_raft_messages_with_diagnostics(message_sink, messages, None).await
}

async fn send_raft_messages_with_diagnostics(
    message_sink: &Arc<dyn RaftMessageSink>,
    messages: Vec<RaftWireMessage>,
    diagnostics: Option<&GridDriveDiagnostics>,
) -> CacheResult<()> {
    let mut messages_by_peer = BTreeMap::<u64, Vec<RaftWireMessage>>::new();
    for message in messages {
        messages_by_peer
            .entry(message.to)
            .or_default()
            .push(message);
    }

    let mut sends = tokio::task::JoinSet::new();
    for (_peer, peer_messages) in messages_by_peer {
        let message_sink = Arc::clone(message_sink);
        sends.spawn(async move {
            let mut errors = Vec::new();
            for message in peer_messages {
                if let Err(error) = message_sink.send(message).await {
                    errors.push(error.to_string());
                }
            }
            errors
        });
    }

    let mut last_error = None;
    while let Some(result) = sends.join_next().await {
        match result {
            Ok(errors) => {
                for error in errors {
                    if let Some(diagnostics) = diagnostics {
                        diagnostics.record_send_failure(error.clone());
                    }
                    last_error = Some(error);
                }
            }
            Err(error) => {
                let error = format!("raft send task failed: {error}");
                if let Some(diagnostics) = diagnostics {
                    diagnostics.record_send_failure(error.clone());
                }
                last_error = Some(error);
            }
        }
    }

    if let Some(error) = last_error {
        return Err(CacheError::Backend(format!(
            "one or more raft messages failed: {error}"
        )));
    }
    Ok(())
}

#[derive(Debug, Default)]
struct NoopRaftMessageSink;

#[async_trait::async_trait]
impl RaftMessageSink for NoopRaftMessageSink {
    async fn send(&self, _message: RaftWireMessage) -> CacheResult<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GridDriveDiagnosticsSnapshot {
    ticks: u64,
    drive_errors: u64,
    send_failures: u64,
    snapshot_send_attempts: u64,
    snapshot_send_successes: u64,
    snapshot_send_failures: u64,
    snapshot_sends_in_flight: u64,
    last_error: Option<String>,
}

#[derive(Debug, Default)]
struct GridDriveDiagnostics {
    ticks: AtomicU64,
    drive_errors: AtomicU64,
    send_failures: AtomicU64,
    snapshot_send_attempts: AtomicU64,
    snapshot_send_successes: AtomicU64,
    snapshot_send_failures: AtomicU64,
    snapshot_sends_in_flight: AtomicU64,
    last_error: Mutex<Option<String>>,
}

impl GridDriveDiagnostics {
    fn record_tick(&self) {
        self.ticks.fetch_add(1, Ordering::Relaxed);
    }

    fn record_drive_error(&self, error: String) {
        self.drive_errors.fetch_add(1, Ordering::Relaxed);
        self.set_last_error(error);
    }

    fn record_send_failure(&self, error: String) {
        self.send_failures.fetch_add(1, Ordering::Relaxed);
        self.set_last_error(error);
    }

    fn record_snapshot_send_started(&self) {
        self.snapshot_send_attempts.fetch_add(1, Ordering::Relaxed);
        self.snapshot_sends_in_flight
            .fetch_add(1, Ordering::Relaxed);
    }

    fn record_snapshot_send_finished(&self, delivered: bool) {
        if delivered {
            self.snapshot_send_successes.fetch_add(1, Ordering::Relaxed);
        } else {
            self.snapshot_send_failures.fetch_add(1, Ordering::Relaxed);
        }
        let previous = self
            .snapshot_sends_in_flight
            .fetch_sub(1, Ordering::Relaxed);
        debug_assert!(previous > 0, "snapshot send completion without a start");
    }

    fn snapshot(&self) -> GridDriveDiagnosticsSnapshot {
        GridDriveDiagnosticsSnapshot {
            ticks: self.ticks.load(Ordering::Relaxed),
            drive_errors: self.drive_errors.load(Ordering::Relaxed),
            send_failures: self.send_failures.load(Ordering::Relaxed),
            snapshot_send_attempts: self.snapshot_send_attempts.load(Ordering::Relaxed),
            snapshot_send_successes: self.snapshot_send_successes.load(Ordering::Relaxed),
            snapshot_send_failures: self.snapshot_send_failures.load(Ordering::Relaxed),
            snapshot_sends_in_flight: self.snapshot_sends_in_flight.load(Ordering::Relaxed),
            last_error: self
                .last_error
                .lock()
                .expect("grid drive diagnostics poisoned")
                .clone(),
        }
    }

    fn set_last_error(&self, error: String) {
        *self
            .last_error
            .lock()
            .expect("grid drive diagnostics poisoned") = Some(error);
    }
}

#[derive(Debug, Clone)]
struct RaftPeer {
    node_id: ClusterNodeId,
    endpoint: String,
}

#[derive(Debug, Clone)]
struct RaftTopology {
    voters: Vec<u64>,
    peers: Arc<BTreeMap<u64, RaftPeer>>,
    multi_voter: bool,
    bootstrap_raft_node_id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResolvedClusterStartMode {
    Bootstrap,
    Join,
}

fn resolved_start_mode(
    config: &ServerConfig,
    raft_log_dir: &Path,
) -> CacheResult<ResolvedClusterStartMode> {
    if !matches!(config.cluster_start, ClusterStartMode::Join) {
        return Ok(ResolvedClusterStartMode::Bootstrap);
    }
    if raft_log_dir_has_state(raft_log_dir)? {
        return Ok(ResolvedClusterStartMode::Bootstrap);
    }
    Ok(ResolvedClusterStartMode::Join)
}

fn raft_log_dir_has_state(path: &Path) -> CacheResult<bool> {
    if !path.exists() {
        return Ok(false);
    }
    let mut entries = fs::read_dir(path).map_err(|error| {
        CacheError::Backend(format!(
            "failed to inspect raft log directory {}: {error}",
            path.display()
        ))
    })?;
    Ok(entries
        .next()
        .transpose()
        .map_err(|error| {
            CacheError::Backend(format!(
                "failed to inspect raft log directory {}: {error}",
                path.display()
            ))
        })?
        .is_some())
}

fn raft_topology(
    config: &ServerConfig,
    local_node_id: ClusterNodeId,
    local_raft_node_id: u64,
) -> CacheResult<RaftTopology> {
    let mut peers = BTreeMap::new();
    if let Some(local_endpoint) = valid_raft_endpoint(&config.cluster_advertise_endpoint()) {
        insert_raft_peer(
            &mut peers,
            local_raft_node_id,
            local_node_id,
            local_endpoint.clone(),
        )?;
        for seed in &config.seeds {
            let Some(endpoint) = valid_raft_endpoint(seed) else {
                continue;
            };
            if endpoint == local_endpoint {
                continue;
            }
            let Some(node_id) = node_id_for_seed_endpoint(seed) else {
                continue;
            };
            let raft_id = raft_node_id(&node_id);
            if raft_id == local_raft_node_id {
                continue;
            }
            insert_raft_peer(&mut peers, raft_id, node_id, endpoint)?;
        }
    }

    let multi_voter = peers.len() > 1;
    let mut voters = if multi_voter {
        peers.keys().copied().collect::<Vec<_>>()
    } else {
        vec![local_raft_node_id]
    };
    voters.sort_unstable();
    voters.dedup();
    let bootstrap_raft_node_id = voters.iter().copied().min().unwrap_or(local_raft_node_id);
    Ok(RaftTopology {
        voters,
        peers: Arc::new(peers),
        multi_voter,
        bootstrap_raft_node_id,
    })
}

fn insert_raft_peer(
    peers: &mut BTreeMap<u64, RaftPeer>,
    raft_id: u64,
    node_id: ClusterNodeId,
    endpoint: String,
) -> CacheResult<()> {
    if let Some(existing) = peers.get(&raft_id) {
        if existing.node_id != node_id {
            return Err(CacheError::Backend(format!(
                "raft node id collision for {raft_id}: {} and {}",
                existing.node_id, node_id
            )));
        }
        return Ok(());
    }
    peers.insert(raft_id, RaftPeer { node_id, endpoint });
    Ok(())
}

fn valid_raft_endpoint(endpoint: &str) -> Option<String> {
    let endpoint = endpoint.trim();
    if endpoint.is_empty() {
        return None;
    }
    if endpoint
        .parse::<SocketAddr>()
        .is_ok_and(|address| address.port() == 0 || address.ip().is_unspecified())
    {
        return None;
    }
    Some(endpoint.to_owned())
}

fn node_id_for_seed_endpoint(endpoint: &str) -> Option<ClusterNodeId> {
    let endpoint = endpoint.trim();
    if let Ok(address) = endpoint.parse::<SocketAddr>() {
        return Some(member_node_id_for_addr(address));
    }
    let host = endpoint
        .rsplit_once(':')
        .map(|(host, _port)| host)
        .unwrap_or(endpoint)
        .trim_matches(['[', ']']);
    host.split('.')
        .next()
        .map(str::trim)
        .filter(|node_id| !node_id.is_empty())
        .map(ClusterNodeId::from)
}

impl RaftTopology {
    fn remote_voters(&self, local_raft_node_id: u64) -> Vec<u64> {
        self.voters
            .iter()
            .copied()
            .filter(|voter| *voter != local_raft_node_id)
            .collect()
    }

    fn election_tick_for(&self, raft_node_id: u64) -> usize {
        let rank = self
            .voters
            .iter()
            .position(|voter| *voter == raft_node_id)
            .unwrap_or(0);
        5 + (rank * 2)
    }

    fn joiner_election_tick(&self) -> usize {
        5 + (2 * (self.voters.len() + 1))
    }
}

#[derive(Debug, Clone)]
struct HttpRaftMessageSink {
    local_node_id: ClusterNodeId,
    local_raft_node_id: u64,
    peers: SharedRaftPeers,
    auth: ClusterRouteAuth,
    scheme: &'static str,
    client: reqwest::Client,
    snapshot_feedback: Option<SnapshotDeliveryFeedback>,
}

#[derive(Debug, Clone)]
struct SnapshotDeliveryFeedback {
    raft: Arc<NetworkedRaftRuntime>,
    diagnostics: Arc<GridDriveDiagnostics>,
}

impl HttpRaftMessageSink {
    fn new(
        local_node_id: ClusterNodeId,
        local_raft_node_id: u64,
        peers: SharedRaftPeers,
        auth: ClusterRouteAuth,
        config: &ServerConfig,
    ) -> CacheResult<Self> {
        let (scheme, client) = raft_http_client(config)?;
        Ok(Self {
            local_node_id,
            local_raft_node_id,
            peers,
            auth,
            scheme,
            client,
            snapshot_feedback: None,
        })
    }

    fn with_snapshot_feedback(
        mut self,
        raft: Arc<NetworkedRaftRuntime>,
        diagnostics: Arc<GridDriveDiagnostics>,
    ) -> Self {
        self.snapshot_feedback = Some(SnapshotDeliveryFeedback { raft, diagnostics });
        self
    }

    fn node_id_for(&self, raft_node_id: u64) -> String {
        if raft_node_id == self.local_raft_node_id {
            self.local_node_id.to_string()
        } else {
            self.peers
                .read()
                .expect("raft peer map poisoned")
                .get(&raft_node_id)
                .map(|peer| peer.node_id.to_string())
                .unwrap_or_else(|| raft_node_id.to_string())
        }
    }

    fn authenticated_headers(&self) -> CacheResult<reqwest::header::HeaderMap> {
        let mut headers = reqwest::header::HeaderMap::new();
        self.auth
            .apply_outbound_headers(&mut headers)
            .map_err(|error| {
                CacheError::Backend(format!("failed to apply cluster auth headers: {error}"))
            })?;
        Ok(headers)
    }

    async fn send_http(&self, message: RaftWireMessage) -> CacheResult<()> {
        let peer = self
            .peers
            .read()
            .expect("raft peer map poisoned")
            .get(&message.to)
            .cloned()
            .ok_or_else(|| {
                CacheError::Backend(format!(
                    "no HTTP raft peer endpoint for raft node {}",
                    message.to
                ))
            })?;
        let request = ClusterOpaqueMessage::new(
            self.node_id_for(message.from),
            peer.node_id.to_string(),
            message.term,
            message.payload,
        );
        let headers = self.authenticated_headers()?;
        let response = self
            .client
            .post(format!(
                "{}://{}{}",
                self.scheme, peer.endpoint, DEFAULT_RAFT_APPEND_PATH
            ))
            .headers(headers)
            .json(&request)
            .send()
            .await
            .map_err(|error| {
                CacheError::Backend(format!(
                    "failed to send raft message to {} at {}: {error}",
                    peer.node_id, peer.endpoint
                ))
            })?;
        if !response.status().is_success() {
            return Err(CacheError::Backend(format!(
                "raft peer {} rejected message with {}",
                peer.node_id,
                response.status()
            )));
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl RaftMessageSink for HttpRaftMessageSink {
    async fn send(&self, message: RaftWireMessage) -> CacheResult<()> {
        if message.to == self.local_raft_node_id {
            return Ok(());
        }
        let snapshot_peer = if self.snapshot_feedback.is_some() && message.is_snapshot()? {
            Some(message.to)
        } else {
            None
        };
        if let (Some(peer_id), Some(feedback)) = (snapshot_peer, &self.snapshot_feedback) {
            feedback.diagnostics.record_snapshot_send_started();
            let result = self.send_http(message).await;
            let delivered = result.is_ok();
            feedback
                .diagnostics
                .record_snapshot_send_finished(delivered);
            feedback
                .raft
                .report_snapshot_delivery_deferred(peer_id, delivered);
            return result;
        }
        self.send_http(message).await
    }
}

fn raft_http_client(config: &ServerConfig) -> CacheResult<(&'static str, reqwest::Client)> {
    if !config.tls.enabled {
        return Ok((
            "http",
            raft_http_client_builder().build().map_err(|error| {
                CacheError::Backend(format!("failed to build raft HTTP client: {error}"))
            })?,
        ));
    }
    install_default_rustls_provider();
    let ca_path = config
        .tls
        .ca_path
        .as_deref()
        .ok_or_else(|| CacheError::Backend("tls.enabled requires ca_path".to_owned()))?;
    let ca_pem = fs::read(ca_path).map_err(|error| {
        CacheError::Backend(format!(
            "failed to read cluster TLS CA {}: {error}",
            ca_path.display()
        ))
    })?;
    let certificate = reqwest::Certificate::from_pem(&ca_pem).map_err(|error| {
        CacheError::Backend(format!(
            "failed to parse cluster TLS CA {}: {error}",
            ca_path.display()
        ))
    })?;
    let client = raft_http_client_builder()
        .add_root_certificate(certificate)
        .build()
        .map_err(|error| {
            CacheError::Backend(format!("failed to build TLS raft client: {error}"))
        })?;
    Ok(("https", client))
}

fn raft_http_client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .connect_timeout(RAFT_HTTP_CONNECT_TIMEOUT)
        .timeout(RAFT_HTTP_REQUEST_TIMEOUT)
}

fn install_default_rustls_provider() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    });
}

async fn wait_for_raft_leader(raft: &Arc<NetworkedRaftRuntime>) -> CacheResult<()> {
    let deadline = tokio::time::Instant::now() + GRID_LEADER_WAIT_TIMEOUT;
    loop {
        if raft.leader_id().is_some() {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(CacheError::Backend(
                "timed out waiting for networked raft leader".to_owned(),
            ));
        }
        tokio::time::sleep(GRID_DRIVE_INTERVAL).await;
    }
}

async fn wait_for_join_complete(
    raft: &Arc<NetworkedRaftRuntime>,
    raft_node_id: u64,
    deadline: Duration,
) -> CacheResult<()> {
    let timeout_ms = deadline.as_millis();
    let deadline = tokio::time::Instant::now() + deadline;
    loop {
        if raft.leader_id().is_some() && raft.voter_ids()?.contains(&raft_node_id) {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(CacheError::Backend(format!(
                "timed out after {timeout_ms}ms waiting for joining raft member {raft_node_id} to become a voter; check seed reachability, cluster auth/TLS compatibility, and live leader availability"
            )));
        }
        tokio::time::sleep(GRID_DRIVE_INTERVAL).await;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MemberIdentity {
    node_id: ClusterNodeId,
    raft_node_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PersistedNodeIdentity {
    format_version: u32,
    cluster: String,
    node_id: String,
    raft_node_id: u64,
}

impl PersistedNodeIdentity {
    fn new(node_id: ClusterNodeId, raft_node_id: u64) -> Self {
        Self {
            format_version: NODE_IDENTITY_FORMAT_VERSION,
            cluster: DEFAULT_CLUSTER_NAME.to_owned(),
            node_id: node_id.to_string(),
            raft_node_id,
        }
    }

    fn into_member_identity(self) -> CacheResult<MemberIdentity> {
        if self.format_version > NODE_IDENTITY_FORMAT_VERSION {
            return Err(CacheError::Backend(format!(
                "unknown future node identity format version {}",
                self.format_version
            )));
        }
        if self.cluster != DEFAULT_CLUSTER_NAME {
            return Err(CacheError::Backend(format!(
                "node identity belongs to cluster {}, expected {DEFAULT_CLUSTER_NAME}",
                self.cluster
            )));
        }
        let node_id = ClusterNodeId::from(self.node_id);
        let expected_raft_node_id = raft_node_id(&node_id);
        if self.raft_node_id != expected_raft_node_id {
            return Err(CacheError::Backend(format!(
                "node identity raft_node_id {} does not match node_id {} (expected {})",
                self.raft_node_id, node_id, expected_raft_node_id
            )));
        }
        Ok(MemberIdentity {
            node_id,
            raft_node_id: self.raft_node_id,
        })
    }
}

fn resolve_member_identity(
    config: &ServerConfig,
    storage_dir: &Path,
) -> CacheResult<MemberIdentity> {
    let path = storage_dir.join(NODE_IDENTITY_FILE);
    if path.exists() {
        return read_member_identity(config, &path);
    }

    fs::create_dir_all(storage_dir).map_err(|error| {
        CacheError::Backend(format!(
            "failed to create member storage directory {}: {error}",
            storage_dir.display()
        ))
    })?;
    let node_id = member_node_id(config);
    let raft_node_id = raft_node_id(&node_id);
    let persisted = PersistedNodeIdentity::new(node_id.clone(), raft_node_id);
    let text = serde_json::to_string_pretty(&persisted)
        .map_err(|error| CacheError::Backend(format!("failed to encode node identity: {error}")))?;
    if !write_node_identity_create_once(&path, &text)? {
        return read_member_identity(config, &path);
    }
    Ok(MemberIdentity {
        node_id,
        raft_node_id,
    })
}

fn read_member_identity(config: &ServerConfig, path: &Path) -> CacheResult<MemberIdentity> {
    let persisted = read_persisted_node_identity(path)?;
    let identity = persisted.into_member_identity()?;
    if let Some(configured_node_id) = configured_member_node_id(config) {
        if configured_node_id != identity.node_id {
            return Err(CacheError::Backend(format!(
                "configured node_id {} conflicts with persisted node identity {} in {}",
                configured_node_id,
                identity.node_id,
                path.display()
            )));
        }
    }
    Ok(identity)
}

fn write_node_identity_create_once(path: &Path, text: &str) -> CacheResult<bool> {
    let storage_dir = path.parent().ok_or_else(|| {
        CacheError::Backend(format!(
            "node identity path {} has no parent directory",
            path.display()
        ))
    })?;
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let sequence = NODE_IDENTITY_TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let temp_path = storage_dir.join(format!(
        ".{NODE_IDENTITY_FILE}.{}.{}.{}.tmp",
        std::process::id(),
        unique,
        sequence
    ));
    fs::write(&temp_path, text).map_err(|error| {
        CacheError::Backend(format!(
            "failed to write temporary node identity {}: {error}",
            temp_path.display()
        ))
    })?;
    let link_result = fs::hard_link(&temp_path, path);
    let cleanup_result = fs::remove_file(&temp_path);
    if let Err(error) = cleanup_result {
        return Err(CacheError::Backend(format!(
            "failed to remove temporary node identity {}: {error}",
            temp_path.display()
        )));
    }
    match link_result {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        Err(error) => Err(CacheError::Backend(format!(
            "failed to persist node identity {}: {error}",
            path.display()
        ))),
    }
}

fn read_persisted_node_identity(path: &Path) -> CacheResult<PersistedNodeIdentity> {
    let text = fs::read_to_string(path).map_err(|error| {
        CacheError::Backend(format!(
            "failed to read node identity {}: {error}",
            path.display()
        ))
    })?;
    serde_json::from_str(&text).map_err(|error| {
        CacheError::Backend(format!(
            "failed to parse node identity {}: {error}",
            path.display()
        ))
    })
}

fn member_node_id(config: &ServerConfig) -> ClusterNodeId {
    configured_member_node_id(config)
        .unwrap_or_else(|| member_node_id_for_addr(config.cluster_addr))
}

fn configured_member_node_id(config: &ServerConfig) -> Option<ClusterNodeId> {
    config
        .node_id
        .as_deref()
        .map(str::trim)
        .filter(|node_id| !node_id.is_empty())
        .map(ClusterNodeId::from)
}

fn member_node_id_for_addr(addr: SocketAddr) -> ClusterNodeId {
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
    ClusterNodeId::from(format!("member-{suffix}"))
}

fn raft_node_id(node_id: &ClusterNodeId) -> u64 {
    stable_nonzero_hash(node_id.as_str())
}

fn stable_nonzero_hash(value: &str) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

    let mut hash = FNV_OFFSET;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash.max(1)
}

fn use_inprocess_grid() -> bool {
    match env::var(GRID_INPROC_ENV) {
        Ok(value) => matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes"
        ),
        Err(_) => false,
    }
}

fn poll_immediate<F>(future: F) -> Result<F::Output, ServerConfigError>
where
    F: Future,
{
    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    match Future::poll(Pin::as_mut(&mut future), &mut context) {
        Poll::Ready(output) => Ok(output),
        Poll::Pending => Err(ServerConfigError::GridHostStart(
            "in-process member host unexpectedly yielded during startup".to_owned(),
        )),
    }
}

fn noop_waker() -> Waker {
    fn raw_waker() -> RawWaker {
        RawWaker::new(std::ptr::null(), &VTABLE)
    }

    unsafe fn clone(_: *const ()) -> RawWaker {
        raw_waker()
    }

    unsafe fn wake(_: *const ()) {}

    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake, wake);

    unsafe { Waker::from_raw(raw_waker()) }
}

struct NetworkedMemberStack {
    cache: HydraCache,
    node_id: ClusterNodeId,
    raft_node_id: u64,
    raft_peers: SharedRaftPeers,
    raft: Arc<NetworkedRaftRuntime>,
    discovery: Arc<ChitchatDiscovery>,
    bridge: ClusterAdmissionBridge,
    message_sink: Arc<dyn RaftMessageSink>,
    drive_diagnostics: Arc<GridDriveDiagnostics>,
    draining: Arc<AtomicBool>,
    drain_remove_proposed: Arc<AtomicBool>,
    raft_compaction_enabled: bool,
    shutdown: watch::Sender<bool>,
}

struct NetworkedGridHandle {
    node_id: ClusterNodeId,
    raft_node_id: u64,
    raft_peers: SharedRaftPeers,
    raft: Arc<NetworkedRaftRuntime>,
    discovery: Arc<ChitchatDiscovery>,
    _bridge: ClusterAdmissionBridge,
    _message_sink: Arc<dyn RaftMessageSink>,
    drive_diagnostics: Arc<GridDriveDiagnostics>,
    draining: Arc<AtomicBool>,
    drain_remove_proposed: Arc<AtomicBool>,
    raft_compaction_enabled: bool,
    shutdown: watch::Sender<bool>,
    _runtime: Option<DedicatedGridRuntime>,
}

impl NetworkedGridHandle {
    fn new(stack: NetworkedMemberStack, runtime: Option<DedicatedGridRuntime>) -> Self {
        Self {
            node_id: stack.node_id,
            raft_node_id: stack.raft_node_id,
            raft_peers: stack.raft_peers,
            raft: stack.raft,
            discovery: stack.discovery,
            _bridge: stack.bridge,
            _message_sink: stack.message_sink,
            drive_diagnostics: stack.drive_diagnostics,
            draining: stack.draining,
            drain_remove_proposed: stack.drain_remove_proposed,
            raft_compaction_enabled: stack.raft_compaction_enabled,
            shutdown: stack.shutdown,
            _runtime: runtime,
        }
    }
}

struct DedicatedGridRuntime {
    runtime: Mutex<Option<tokio::runtime::Runtime>>,
}

impl DedicatedGridRuntime {
    fn new(runtime: tokio::runtime::Runtime) -> Self {
        Self {
            runtime: Mutex::new(Some(runtime)),
        }
    }

    fn block_on<F>(&self, future: F) -> F::Output
    where
        F: Future,
    {
        let guard = self.runtime.lock().expect("grid runtime holder poisoned");
        guard
            .as_ref()
            .expect("grid runtime must exist while handle is live")
            .block_on(future)
    }

    fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let guard = self.runtime.lock().expect("grid runtime holder poisoned");
        let handle = guard
            .as_ref()
            .expect("grid runtime must exist while handle is live")
            .spawn(future);
        std::mem::drop(handle);
    }
}

impl Drop for DedicatedGridRuntime {
    fn drop(&mut self) {
        if let Some(runtime) = self
            .runtime
            .lock()
            .expect("grid runtime holder poisoned")
            .take()
        {
            if tokio::runtime::Handle::try_current().is_ok() {
                let _ = std::thread::spawn(move || {
                    runtime.shutdown_timeout(GRID_RUNTIME_SHUTDOWN_TIMEOUT);
                })
                .join();
            } else {
                runtime.shutdown_timeout(GRID_RUNTIME_SHUTDOWN_TIMEOUT);
            }
        }
    }
}

impl Drop for NetworkedGridHandle {
    fn drop(&mut self) {
        let _ = self.shutdown.send(true);
    }
}

impl fmt::Debug for NetworkedGridHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NetworkedGridHandle")
            .field("node_id", &self.node_id)
            .field("raft_node_id", &self.raft_node_id)
            .field("snapshot", &self.raft.metadata_snapshot())
            .field("drive_diagnostics", &self.drive_diagnostics.snapshot())
            .field("has_dedicated_runtime", &self._runtime.is_some())
            .finish()
    }
}

impl GridControlPlaneHandle for NetworkedGridHandle {
    fn begin_drain(&self) {
        self.draining.store(true, Ordering::SeqCst);
        self.try_remove_local_voter_for_drain();
    }

    fn snapshot(&self) -> RaftMetadataSnapshot {
        self.raft.metadata_snapshot()
    }

    fn members(&self) -> Vec<ClusterMember> {
        self.raft.members()
    }

    fn raft_leader_id(&self) -> Option<String> {
        self.raft.leader_id().map(|leader| {
            if leader == self.raft_node_id {
                self.node_id.to_string()
            } else {
                self.raft_peers
                    .read()
                    .expect("raft peer map poisoned")
                    .get(&leader)
                    .map(|peer| peer.node_id.to_string())
                    .unwrap_or_else(|| format!("raft-{leader}"))
            }
        })
    }

    fn has_quorum(&self) -> bool {
        if self.raft.leader_id().is_none() || self.raft.members().is_empty() {
            return false;
        }
        let Ok(voters) = self.raft.voter_ids() else {
            return false;
        };
        if voters.is_empty() {
            return false;
        }
        let reachable = voters
            .iter()
            .filter(|raft_id| self.raft_voter_reachability(**raft_id) == Reachability::Reachable)
            .count();
        raft_voter_majority_reachable(voters.len(), reachable)
    }

    fn voter_count(&self) -> u32 {
        self.raft
            .voter_ids()
            .map(|voters| voters.len() as u32)
            .unwrap_or(0)
    }

    fn reachability(&self, node: &ClusterNodeId) -> Reachability {
        if node == &self.node_id {
            return Reachability::Reachable;
        }

        if let Some(reachability) =
            reachability_from_discovery_liveness(self.discovery.liveness().get(node).copied())
        {
            return reachability;
        }

        if self
            .raft
            .members()
            .into_iter()
            .any(|member| member.node_id == *node)
        {
            Reachability::Reachable
        } else {
            Reachability::Unreachable
        }
    }

    fn reshard_phase(&self) -> ReshardPhase {
        ReshardPhase::Idle
    }

    fn is_draining(&self) -> bool {
        self.draining.load(Ordering::SeqCst)
    }

    fn raft_compaction_status(&self) -> Result<RaftCompactionStatus, RaftCompactionError> {
        raft_compaction_status(
            &self.raft,
            self.raft_compaction_enabled,
            &self.drive_diagnostics,
        )
    }

    fn compact_raft_log_at_applied(&self) -> Result<RaftCompactionStatus, RaftCompactionError> {
        if !self.raft_compaction_enabled {
            return Err(RaftCompactionError::Disabled);
        }
        self.raft
            .compact_applied_log_to_snapshot()
            .map_err(|error| RaftCompactionError::Runtime(error.to_string()))?;
        raft_compaction_status(&self.raft, true, &self.drive_diagnostics)
    }
}

fn raft_compaction_status(
    raft: &NetworkedRaftRuntime,
    enabled: bool,
    diagnostics: &GridDriveDiagnostics,
) -> Result<RaftCompactionStatus, RaftCompactionError> {
    let observation = raft
        .log_compaction_observation()
        .map_err(|error| RaftCompactionError::Runtime(error.to_string()))?;
    let runtime = raft.snapshot();
    let delivery = diagnostics.snapshot();
    Ok(RaftCompactionStatus {
        available: true,
        enabled,
        applied_index: Some(observation.applied_index),
        snapshot_index: Some(observation.snapshot_index),
        first_log_index: Some(observation.first_log_index),
        last_log_index: Some(observation.last_log_index),
        snapshot_send_attempts: Some(delivery.snapshot_send_attempts),
        snapshot_send_successes: Some(delivery.snapshot_send_successes),
        snapshot_send_failures: Some(delivery.snapshot_send_failures),
        snapshot_sends_in_flight: Some(delivery.snapshot_sends_in_flight),
        snapshot_installs: Some(runtime.snapshot_installs),
    })
}

impl NetworkedGridHandle {
    fn raft_voter_reachability(&self, raft_id: u64) -> Reachability {
        if raft_id == self.raft_node_id {
            return Reachability::Reachable;
        }
        let Some(node_id) = self
            .raft_peers
            .read()
            .expect("raft peer map poisoned")
            .get(&raft_id)
            .map(|peer| peer.node_id.clone())
        else {
            return Reachability::Unreachable;
        };
        self.reachability(&node_id)
    }

    fn try_remove_local_voter_for_drain(&self) {
        if self.drain_remove_proposed.load(Ordering::SeqCst) {
            return;
        }
        let Ok(voters) = self.raft.voter_ids() else {
            return;
        };
        if voters.len() <= 1 || !voters.contains(&self.raft_node_id) {
            return;
        }
        if self.raft.leader_id().is_none() {
            return;
        }
        let Ok(messages) = self.raft.request_remove_voter(self.raft_node_id) else {
            return;
        };
        self.drain_remove_proposed.store(true, Ordering::SeqCst);
        if let Some(runtime) = &self._runtime {
            if tokio::runtime::Handle::try_current().is_ok() {
                let message_sink = Arc::clone(&self._message_sink);
                runtime.spawn(async move {
                    let _ = send_raft_messages(&message_sink, messages).await;
                });
            } else {
                let _ = runtime.block_on(send_raft_messages(&self._message_sink, messages));
            }
        }
    }
}

struct InProcessGridHandle {
    control_plane: Arc<RaftStyleMetadataControlPlane>,
    _runtime: Option<Arc<tokio::runtime::Runtime>>,
}

impl InProcessGridHandle {
    fn new(
        control_plane: Arc<RaftStyleMetadataControlPlane>,
        runtime: Option<Arc<tokio::runtime::Runtime>>,
    ) -> Self {
        Self {
            control_plane,
            _runtime: runtime,
        }
    }
}

impl fmt::Debug for InProcessGridHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InProcessGridHandle")
            .field("snapshot", &self.control_plane.snapshot())
            .field("has_dedicated_runtime", &self._runtime.is_some())
            .finish()
    }
}

impl GridControlPlaneHandle for InProcessGridHandle {
    fn begin_drain(&self) {}

    fn snapshot(&self) -> RaftMetadataSnapshot {
        self.control_plane.snapshot()
    }

    fn members(&self) -> Vec<ClusterMember> {
        self.control_plane.members()
    }

    fn raft_leader_id(&self) -> Option<String> {
        None
    }

    fn has_quorum(&self) -> bool {
        !self.control_plane.members().is_empty()
    }

    fn voter_count(&self) -> u32 {
        self.control_plane.members().len() as u32
    }

    fn reachability(&self, node: &ClusterNodeId) -> Reachability {
        if self
            .control_plane
            .members()
            .iter()
            .any(|member| &member.node_id == node)
        {
            Reachability::Reachable
        } else {
            Reachability::Unreachable
        }
    }

    fn reshard_phase(&self) -> ReshardPhase {
        ReshardPhase::Idle
    }

    fn is_draining(&self) -> bool {
        false
    }
}

fn reachability_from_discovery_liveness(
    liveness: Option<ClusterDiscoveryLiveness>,
) -> Option<Reachability> {
    match liveness {
        Some(ClusterDiscoveryLiveness::Live) => Some(Reachability::Reachable),
        Some(ClusterDiscoveryLiveness::Suspect) => Some(Reachability::Suspect),
        Some(ClusterDiscoveryLiveness::Dead) => Some(Reachability::Unreachable),
        None => None,
    }
}

fn raft_voter_majority_reachable(total_voters: usize, reachable_voters: usize) -> bool {
    total_voters > 0 && reachable_voters >= (total_voters / 2).saturating_add(1)
}

#[derive(Clone)]
struct RaftClusterMessageHandler {
    node_id: ClusterNodeId,
    raft_node_id: u64,
    raft: Arc<NetworkedRaftRuntime>,
    message_sink: Arc<dyn RaftMessageSink>,
    raft_peers: SharedRaftPeers,
}

impl fmt::Debug for RaftClusterMessageHandler {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RaftClusterMessageHandler")
            .field("node_id", &self.node_id)
            .field("raft_node_id", &self.raft_node_id)
            .finish()
    }
}

#[async_trait::async_trait]
impl ClusterMessageHandler for RaftClusterMessageHandler {
    async fn handle(
        &self,
        route: ClusterRoute,
        message: ClusterOpaqueMessage,
    ) -> CacheResult<ClusterMessageAck> {
        let payload = message.decode_payload()?;
        if matches!(route, ClusterRoute::Replicate) {
            return Ok(ClusterMessageAck::new(
                route,
                self.node_id.to_string(),
                payload.len(),
            ));
        }

        let outbound = self.raft.step(RaftWireMessage {
            from: self.resolve_wire_sender(&message.from)?,
            to: self.raft_node_id,
            term: message.term,
            payload: payload.to_vec(),
        })?;
        send_raft_messages(&self.message_sink, outbound).await?;
        Ok(ClusterMessageAck::new(
            route,
            self.node_id.to_string(),
            payload.len(),
        ))
    }
}

impl RaftClusterMessageHandler {
    fn resolve_wire_sender(&self, wire_from: &str) -> CacheResult<u64> {
        if wire_from == self.node_id.as_str() {
            return Ok(self.raft_node_id);
        }
        let peers = self.raft_peers.read().expect("raft peer map poisoned");
        if let Some((raft_id, _)) = peers
            .iter()
            .find(|(_, peer)| peer.node_id.as_str() == wire_from)
        {
            return Ok(*raft_id);
        }
        let known = peers
            .values()
            .map(|peer| peer.node_id.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        Err(CacheError::Backend(format!(
            "unknown raft wire sender {wire_from}; known senders: local={}, peers=[{}]",
            self.node_id, known
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{http::StatusCode, routing::post, Router};
    use hydracache::{
        ClusterCandidate, ClusterControlPlane, ClusterEndpoints, ClusterEpoch, ClusterRole,
        InMemoryClusterDiscovery,
    };
    use hydracache_cluster_raft::InMemoryRaftMessageSink;
    use hydracache_cluster_transport_axum::{
        HYDRACACHE_NODE_KEY_ID_HEADER, HYDRACACHE_NODE_TOKEN_HEADER,
    };
    use proptest::prelude::*;
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    fn test_raft_runtime() -> Arc<NetworkedRaftRuntime> {
        let sequence = NODE_IDENTITY_TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
        let path = PathBuf::from(format!(
            "target/test-hydracache-grid-host/unit/raft-runtime-{}-{sequence}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        Arc::new(
            RaftMetadataRuntime::sled_with_config(
                RaftMetadataRuntimeConfig::single_node(DEFAULT_CLUSTER_NAME, 1),
                path,
            )
            .unwrap(),
        )
    }

    #[tokio::test]
    async fn drive_loop_admits_a_gossip_candidate_into_the_shared_raft_runtime() {
        let raft = test_raft_runtime();
        let discovery = Arc::new(InMemoryClusterDiscovery::new());
        let bridge = ClusterAdmissionBridge::new(discovery.clone(), raft.clone());
        let message_sink: Arc<dyn RaftMessageSink> = Arc::new(InMemoryRaftMessageSink::default());
        let raft_peers = Arc::new(RwLock::new(BTreeMap::new()));
        let diagnostics = GridDriveDiagnostics::default();

        discovery
            .announce(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)));
        bridge.run_once().await;
        let handles = GridDriveHandles {
            raft: raft.clone(),
            message_sink: message_sink.clone(),
            raft_peers: raft_peers.clone(),
            diagnostics: Arc::new(diagnostics),
            last_voters: Arc::new(RwLock::new(BTreeSet::new())),
            suppressed_raft_promotions: Arc::new(RwLock::new(BTreeSet::new())),
            local_node_id: ClusterNodeId::from("local"),
            local_endpoint: "127.0.0.1:7000".to_owned(),
            local_generation: ClusterGeneration::new(1),
        };
        drive_grid_once(&handles, &[]).await.unwrap();

        assert!(raft
            .members()
            .iter()
            .any(|member| member.node_id.as_str() == "member-a"));
        assert!(raft.commands().iter().any(|command| matches!(
            command,
            hydracache::RaftMetadataCommand::MemberUpsert { node_id, .. }
                if node_id.as_str() == "member-a"
        )));
    }

    #[test]
    fn resolved_start_mode_joins_only_when_configured_and_log_is_empty() {
        let mut config = test_member_config("127.0.0.1:7000");
        let dir = PathBuf::from(format!(
            "target/test-hydracache-grid-host/start-mode-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        assert_eq!(
            resolved_start_mode(&config, &dir).unwrap(),
            ResolvedClusterStartMode::Bootstrap
        );

        config.cluster_start = ClusterStartMode::Join;
        assert_eq!(
            resolved_start_mode(&config, &dir).unwrap(),
            ResolvedClusterStartMode::Join
        );

        std::fs::write(dir.join("conf-state"), b"present").unwrap();
        assert_eq!(
            resolved_start_mode(&config, &dir).unwrap(),
            ResolvedClusterStartMode::Bootstrap
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn member_identity_creation_is_safe_under_concurrent_startup() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = PathBuf::from(format!(
            "target/test-hydracache-grid-host/unit/identity-concurrent-{}-{unique}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);

        let mut config = test_member_config("127.0.0.1:7000");
        config.node_id = Some("member-concurrent".to_owned());
        let config = Arc::new(config);
        let handles = (0..24)
            .map(|_| {
                let config = Arc::clone(&config);
                let dir = dir.clone();
                std::thread::spawn(move || resolve_member_identity(config.as_ref(), &dir).unwrap())
            })
            .collect::<Vec<_>>();

        let identities = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>();
        let first = identities.first().unwrap();
        assert!(identities.iter().all(|identity| identity == first));

        let identity_path = dir.join(NODE_IDENTITY_FILE);
        let persisted = read_persisted_node_identity(&identity_path).unwrap();
        assert_eq!(persisted.into_member_identity().unwrap(), first.clone());
        let leftovers = fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(leftovers, 0);
        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn announce_join_candidate_uses_advertised_control_endpoint() {
        let mut config = test_member_config("127.0.0.1:0");
        config.cluster_advertise_addr = Some("127.0.0.1:7100".to_owned());
        let node_id = ClusterNodeId::from("joiner-a");
        let generation = ClusterGeneration::new(1);
        let discovery = ChitchatDiscovery::spawn_udp(ChitchatDiscoveryConfig::new(
            DEFAULT_CLUSTER_NAME,
            node_id.clone(),
            generation,
            "127.0.0.1:0".parse().unwrap(),
        ))
        .await
        .unwrap();

        announce_join_candidate(&discovery, node_id.clone(), generation, &config)
            .await
            .unwrap();

        let candidates = discovery.candidates();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].node_id, node_id);
        assert_eq!(candidates[0].role, ClusterRole::Member);
        assert_eq!(
            candidates[0].endpoints.control.as_deref(),
            Some("127.0.0.1:7100")
        );
    }

    #[test]
    fn raft_topology_remote_voters_exclude_local_member() {
        let mut config = test_member_config("127.0.0.1:7000");
        config.seeds = vec!["127.0.0.1:7001".to_owned(), "127.0.0.1:7002".to_owned()];
        let local_node = member_node_id_for_addr(config.cluster_addr);
        let local_raft_id = raft_node_id(&local_node);
        let topology = raft_topology(&config, local_node, local_raft_id).unwrap();
        let remote_voters = topology
            .remote_voters(local_raft_id)
            .into_iter()
            .collect::<BTreeSet<_>>();
        let expected_remote_voters = config
            .seeds
            .iter()
            .map(|seed| seed.parse::<SocketAddr>().unwrap())
            .map(member_node_id_for_addr)
            .map(|node_id| raft_node_id(&node_id))
            .collect::<BTreeSet<_>>();

        assert!(topology.multi_voter);
        assert!(topology.voters.contains(&local_raft_id));
        assert!(!remote_voters.contains(&local_raft_id));
        assert_eq!(remote_voters, expected_remote_voters);
        let slowest_bootstrap_tick = topology
            .voters
            .iter()
            .map(|voter| topology.election_tick_for(*voter))
            .max()
            .unwrap();
        assert!(topology.joiner_election_tick() > slowest_bootstrap_tick);
    }

    #[test]
    fn raft_topology_accepts_dns_seed_endpoints() {
        let mut config = test_member_config("0.0.0.0:7000");
        config.node_id = Some("demo-0".to_owned());
        config.cluster_advertise_addr = Some("demo-0.demo-headless:7000".to_owned());
        config.seeds = vec![
            "demo-0.demo-headless:7000".to_owned(),
            "demo-1.demo-headless:7000".to_owned(),
            "demo-2.demo-headless:7000".to_owned(),
        ];
        let local_node = member_node_id(&config);
        let local_raft_id = raft_node_id(&local_node);

        let topology = raft_topology(&config, local_node, local_raft_id).unwrap();
        let voters = topology.voters.iter().copied().collect::<BTreeSet<_>>();

        assert!(topology.multi_voter);
        assert_eq!(voters.len(), 3);
        assert!(voters.contains(&raft_node_id(&ClusterNodeId::from("demo-0"))));
        assert!(voters.contains(&raft_node_id(&ClusterNodeId::from("demo-1"))));
        assert!(voters.contains(&raft_node_id(&ClusterNodeId::from("demo-2"))));
        assert_eq!(
            topology
                .peers
                .get(&raft_node_id(&ClusterNodeId::from("demo-1")))
                .map(|peer| peer.endpoint.as_str()),
            Some("demo-1.demo-headless:7000")
        );
    }

    #[test]
    fn refresh_raft_peers_tracks_admitted_member_control_endpoints() {
        let local_node = ClusterNodeId::from("local");
        let member_node = ClusterNodeId::from("member-a");
        let member = ClusterMember {
            node_id: member_node.clone(),
            generation: ClusterGeneration::new(1),
            role: ClusterRole::Member,
            epoch: ClusterEpoch::new(1),
            endpoints: ClusterEndpoints::new().control("127.0.0.1:7001"),
            metadata: BTreeMap::new(),
        };
        let raft_peers = Arc::new(RwLock::new(BTreeMap::new()));

        refresh_raft_peers(
            &raft_peers,
            &local_node,
            "127.0.0.1:7000".to_owned(),
            &[member],
            ClusterGeneration::new(1),
            &[],
        );

        let peers = raft_peers.read().expect("raft peer map poisoned");
        assert_eq!(
            peers
                .get(&raft_node_id(&local_node))
                .map(|peer| peer.endpoint.as_str()),
            Some("127.0.0.1:7000")
        );
        assert_eq!(
            peers
                .get(&raft_node_id(&member_node))
                .map(|peer| peer.endpoint.as_str()),
            Some("127.0.0.1:7001")
        );
    }

    #[test]
    fn refresh_raft_peers_uses_candidate_endpoints_only_as_missing_hints() {
        let local_node = ClusterNodeId::from("local");
        let member_node = ClusterNodeId::from("member-a");
        let candidate_node = ClusterNodeId::from("member-b");
        let stale_candidate_node = ClusterNodeId::from("member-stale");
        let member = ClusterMember {
            node_id: member_node.clone(),
            generation: ClusterGeneration::new(1),
            role: ClusterRole::Member,
            epoch: ClusterEpoch::new(1),
            endpoints: ClusterEndpoints::new().control("127.0.0.1:7001"),
            metadata: BTreeMap::new(),
        };
        let candidates = vec![
            ClusterCandidate::member(member_node.clone())
                .generation(ClusterGeneration::new(1))
                .endpoints(ClusterEndpoints::new().control("127.0.0.1:7999")),
            ClusterCandidate::member(candidate_node.clone())
                .generation(ClusterGeneration::new(1))
                .endpoints(ClusterEndpoints::new().control("127.0.0.1:7002")),
            ClusterCandidate::member(stale_candidate_node.clone())
                .generation(ClusterGeneration::new(2))
                .endpoints(ClusterEndpoints::new().control("127.0.0.1:7003")),
            ClusterCandidate::client("client-a")
                .generation(ClusterGeneration::new(1))
                .endpoints(ClusterEndpoints::new().control("127.0.0.1:7004")),
        ];
        let raft_peers = Arc::new(RwLock::new(BTreeMap::new()));

        refresh_raft_peers(
            &raft_peers,
            &local_node,
            "127.0.0.1:7000".to_owned(),
            &[member],
            ClusterGeneration::new(1),
            &candidates,
        );

        let peers = raft_peers.read().expect("raft peer map poisoned");
        assert_eq!(
            peers
                .get(&raft_node_id(&member_node))
                .map(|peer| peer.endpoint.as_str()),
            Some("127.0.0.1:7001")
        );
        assert_eq!(
            peers
                .get(&raft_node_id(&candidate_node))
                .map(|peer| peer.endpoint.as_str()),
            Some("127.0.0.1:7002")
        );
        assert!(!peers.contains_key(&raft_node_id(&stale_candidate_node)));
        assert!(!peers.contains_key(&raft_node_id(&ClusterNodeId::from("client-a"))));
    }

    #[tokio::test]
    async fn sync_raft_voters_adds_admitted_member_with_known_peer() {
        let raft = test_raft_runtime();
        let message_sink: Arc<dyn RaftMessageSink> = Arc::new(InMemoryRaftMessageSink::default());
        let member_node = ClusterNodeId::from("member-a");
        let member_raft_id = raft_node_id(&member_node);
        let raft_peers = Arc::new(RwLock::new(BTreeMap::from([(
            member_raft_id,
            RaftPeer {
                node_id: member_node.clone(),
                endpoint: "127.0.0.1:7001".to_owned(),
            },
        )])));

        raft.join_member(
            ClusterCandidate::member(member_node)
                .generation(ClusterGeneration::new(1))
                .endpoints(ClusterEndpoints::new().control("127.0.0.1:7001")),
        )
        .await
        .unwrap();
        let diagnostics = GridDriveDiagnostics::default();
        let suppressed_raft_promotions = Arc::new(RwLock::new(BTreeSet::new()));
        sync_raft_voters(
            &raft,
            &message_sink,
            &raft_peers,
            &diagnostics,
            &suppressed_raft_promotions,
        )
        .await
        .unwrap();

        assert!(raft.voter_ids().unwrap().contains(&member_raft_id));
    }

    #[tokio::test]
    async fn sync_raft_voters_does_not_resurrect_recently_removed_member() {
        let raft = test_raft_runtime();
        let sink = Arc::new(InMemoryRaftMessageSink::default());
        let message_sink: Arc<dyn RaftMessageSink> = sink.clone();
        let member_node = ClusterNodeId::from("member-draining");
        let member_raft_id = raft_node_id(&member_node);
        let raft_peers = Arc::new(RwLock::new(BTreeMap::from([(
            member_raft_id,
            RaftPeer {
                node_id: member_node.clone(),
                endpoint: "127.0.0.1:7001".to_owned(),
            },
        )])));

        raft.join_member(
            ClusterCandidate::member(member_node)
                .generation(ClusterGeneration::new(1))
                .endpoints(ClusterEndpoints::new().control("127.0.0.1:7001")),
        )
        .await
        .unwrap();
        let suppressed_raft_promotions = Arc::new(RwLock::new(BTreeSet::from([member_raft_id])));

        sync_raft_voters(
            &raft,
            &message_sink,
            &raft_peers,
            &GridDriveDiagnostics::default(),
            &suppressed_raft_promotions,
        )
        .await
        .unwrap();

        assert!(!raft.voter_ids().unwrap().contains(&member_raft_id));
        assert!(
            sink.messages().is_empty(),
            "recently removed voter must not receive a resurrecting AddNode"
        );
    }

    #[tokio::test]
    async fn wait_for_join_complete_returns_when_self_is_voter() {
        let raft = test_raft_runtime();

        wait_for_join_complete(&raft, 1, Duration::from_millis(1))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn drive_loop_counts_and_reports_send_failures() {
        let sink: Arc<dyn RaftMessageSink> = Arc::new(FailingRaftMessageSink);
        let diagnostics = GridDriveDiagnostics::default();
        let message = RaftWireMessage {
            from: 1,
            to: 2,
            term: 1,
            payload: Vec::new(),
        };

        let error = send_raft_messages_with_diagnostics(&sink, vec![message], Some(&diagnostics))
            .await
            .unwrap_err();
        let snapshot = diagnostics.snapshot();

        assert!(error.to_string().contains("forced raft send failure"));
        assert_eq!(snapshot.send_failures, 1);
        assert!(snapshot
            .last_error
            .as_deref()
            .is_some_and(|error| error.contains("forced raft send failure")));
    }

    #[tokio::test]
    async fn raft_drive_continues_after_bounded_peer_send_timeout() {
        let delivered_to_live_peer = Arc::new(tokio::sync::Notify::new());
        let sink: Arc<dyn RaftMessageSink> = Arc::new(SlowPeerRaftMessageSink {
            slow_peer: 2,
            live_peer: 3,
            delivered_to_live_peer: Arc::clone(&delivered_to_live_peer),
        });
        let diagnostics = GridDriveDiagnostics::default();
        let messages = vec![
            RaftWireMessage {
                from: 1,
                to: 2,
                term: 1,
                payload: Vec::new(),
            },
            RaftWireMessage {
                from: 1,
                to: 3,
                term: 1,
                payload: Vec::new(),
            },
        ];

        let first_error = send_raft_messages_with_diagnostics(&sink, messages, Some(&diagnostics))
            .await
            .unwrap_err();
        assert!(
            first_error.to_string().contains("slow peer unavailable"),
            "bounded peer failure should be surfaced: {first_error}"
        );
        assert_eq!(diagnostics.snapshot().send_failures, 1);
        tokio::time::timeout(
            Duration::from_millis(100),
            delivered_to_live_peer.notified(),
        )
        .await
        .expect("first batch should still reach the live peer");

        send_raft_messages_with_diagnostics(
            &sink,
            vec![RaftWireMessage {
                from: 1,
                to: 3,
                term: 1,
                payload: Vec::new(),
            }],
            Some(&diagnostics),
        )
        .await
        .unwrap();

        tokio::time::timeout(
            Duration::from_millis(100),
            delivered_to_live_peer.notified(),
        )
        .await
        .expect("later live-peer message should still be processed after a bounded peer failure");
        let snapshot = diagnostics.snapshot();
        assert_eq!(snapshot.send_failures, 1);
        assert!(snapshot
            .last_error
            .as_deref()
            .is_some_and(|error| error.contains("slow peer unavailable")));
    }

    #[tokio::test]
    async fn raft_send_batch_does_not_head_of_line_block_live_peers() {
        let delivered_to_live_peer = Arc::new(tokio::sync::Notify::new());
        let sink: Arc<dyn RaftMessageSink> = Arc::new(SlowPeerRaftMessageSink {
            slow_peer: 2,
            live_peer: 3,
            delivered_to_live_peer: Arc::clone(&delivered_to_live_peer),
        });
        let messages = vec![
            RaftWireMessage {
                from: 1,
                to: 2,
                term: 1,
                payload: Vec::new(),
            },
            RaftWireMessage {
                from: 1,
                to: 3,
                term: 1,
                payload: Vec::new(),
            },
        ];
        let send = tokio::spawn({
            let sink = Arc::clone(&sink);
            async move { send_raft_messages_with_diagnostics(&sink, messages, None).await }
        });

        tokio::time::timeout(
            Duration::from_millis(100),
            delivered_to_live_peer.notified(),
        )
        .await
        .expect("slow peer must not block delivery to another peer");

        let error = send
            .await
            .expect("send task should not panic")
            .expect_err("slow peer failure should still be reported");
        assert!(
            error.to_string().contains("slow peer unavailable"),
            "slow peer error should be preserved: {error}"
        );
    }

    #[tokio::test]
    async fn single_voter_sink_does_not_accumulate() {
        let sink = NoopRaftMessageSink;
        let message = RaftWireMessage {
            from: 1,
            to: 1,
            term: 1,
            payload: Vec::new(),
        };

        sink.send(message).await.unwrap();
    }

    #[test]
    fn reachability_maps_chitchat_liveness() {
        assert_eq!(
            reachability_from_discovery_liveness(Some(ClusterDiscoveryLiveness::Live)),
            Some(Reachability::Reachable)
        );
        assert_eq!(
            reachability_from_discovery_liveness(Some(ClusterDiscoveryLiveness::Suspect)),
            Some(Reachability::Suspect)
        );
        assert_eq!(
            reachability_from_discovery_liveness(Some(ClusterDiscoveryLiveness::Dead)),
            Some(Reachability::Unreachable)
        );
        assert_eq!(reachability_from_discovery_liveness(None), None);
    }

    #[test]
    fn has_quorum_reflects_voter_majority() {
        assert!(!raft_voter_majority_reachable(0, 0));
        assert!(raft_voter_majority_reachable(1, 1));
        assert!(!raft_voter_majority_reachable(3, 1));
        assert!(raft_voter_majority_reachable(3, 2));
        assert!(!raft_voter_majority_reachable(4, 2));
        assert!(raft_voter_majority_reachable(4, 3));
    }

    #[derive(Debug)]
    struct FailingRaftMessageSink;

    #[async_trait::async_trait]
    impl RaftMessageSink for FailingRaftMessageSink {
        async fn send(&self, _message: RaftWireMessage) -> CacheResult<()> {
            Err(CacheError::Backend("forced raft send failure".to_owned()))
        }
    }

    #[derive(Debug)]
    struct SlowPeerRaftMessageSink {
        slow_peer: u64,
        live_peer: u64,
        delivered_to_live_peer: Arc<tokio::sync::Notify>,
    }

    #[async_trait::async_trait]
    impl RaftMessageSink for SlowPeerRaftMessageSink {
        async fn send(&self, message: RaftWireMessage) -> CacheResult<()> {
            if message.to == self.slow_peer {
                tokio::time::sleep(Duration::from_millis(250)).await;
                return Err(CacheError::Backend("slow peer unavailable".to_owned()));
            }
            if message.to == self.live_peer {
                self.delivered_to_live_peer.notify_one();
            }
            Ok(())
        }
    }

    fn test_raft_handler(peers: BTreeMap<u64, RaftPeer>) -> RaftClusterMessageHandler {
        let node_id = ClusterNodeId::from("local");
        RaftClusterMessageHandler {
            raft_node_id: raft_node_id(&node_id),
            node_id,
            raft: test_raft_runtime(),
            message_sink: Arc::new(InMemoryRaftMessageSink::default()),
            raft_peers: Arc::new(RwLock::new(peers)),
        }
    }

    #[test]
    fn plaintext_route_is_acknowledged_only_on_loopback_or_staged_boundary() {
        let node_id = ClusterNodeId::from("member-a");
        let mut config = test_member_config("127.0.0.1:7000");
        let auth = cluster_route_auth(&config, &node_id).unwrap();
        assert!(auth.route_enabled(ClusterRoute::RaftAppend));

        config.cluster_addr = "10.0.0.1:7000".parse().unwrap();
        config.tls.acknowledge_insecure = false;
        let auth = cluster_route_auth(&config, &node_id).unwrap();
        assert!(!auth.route_enabled(ClusterRoute::RaftAppend));

        config.tls.acknowledge_insecure = true;
        let auth = cluster_route_auth(&config, &node_id).unwrap();
        assert!(auth.route_enabled(ClusterRoute::RaftAppend));
    }

    #[test]
    fn cluster_auth_rotation_and_token_failures_are_explicit() {
        let dir = unique_test_dir("cluster-auth");
        fs::create_dir_all(&dir).unwrap();
        let current = dir.join("current.token");
        let previous = dir.join("previous.token");
        let node_id = ClusterNodeId::from("member-auth");
        let mut config = test_member_config("127.0.0.1:7000");
        config.cluster_auth.key_id = Some("current".to_owned());
        config.cluster_auth.token_file = Some(current.clone());

        let error = cluster_auth_provider(&config, &node_id).unwrap_err();
        assert!(error
            .to_string()
            .contains("failed to read cluster_auth.token_file"));

        fs::write(&current, "\r\n").unwrap();
        let error = cluster_auth_provider(&config, &node_id).unwrap_err();
        assert!(error.to_string().contains("cluster_auth.token_file"));
        assert!(error.to_string().contains("is empty"));

        fs::write(&current, "current-secret\r\n").unwrap();
        config.cluster_auth.previous_key_id = Some("previous".to_owned());
        let error = cluster_auth_provider(&config, &node_id).unwrap_err();
        assert!(error
            .to_string()
            .contains("cluster_auth.previous requires key_id and readable token_file"));

        config.cluster_auth.previous_token_file = Some(previous.clone());
        fs::write(&previous, "previous-secret\n").unwrap();
        assert!(cluster_auth_provider(&config, &node_id).unwrap().is_some());

        let mut tls_without_auth = test_member_config("127.0.0.1:7000");
        tls_without_auth.tls.enabled = true;
        let error = cluster_route_auth(&tls_without_auth, &node_id).unwrap_err();
        assert!(error.to_string().contains("requires [cluster_auth]"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn raft_tls_client_rejects_missing_and_unreadable_ca() {
        let dir = unique_test_dir("raft-client-ca");
        fs::create_dir_all(&dir).unwrap();
        let mut config = test_member_config("127.0.0.1:7000");
        config.tls.enabled = true;

        let error = raft_http_client(&config).unwrap_err();
        assert!(error.to_string().contains("tls.enabled requires ca_path"));

        let missing = dir.join("missing-ca.pem");
        config.tls.ca_path = Some(missing);
        let error = raft_http_client(&config).unwrap_err();
        assert!(error.to_string().contains("failed to read cluster TLS CA"));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn persisted_identity_rejects_wrong_cluster_hash_and_malformed_files() {
        let node_id = ClusterNodeId::from("member-identity");
        let raft_id = raft_node_id(&node_id);
        let error = PersistedNodeIdentity {
            format_version: NODE_IDENTITY_FORMAT_VERSION,
            cluster: "other-cluster".to_owned(),
            node_id: node_id.to_string(),
            raft_node_id: raft_id,
        }
        .into_member_identity()
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("belongs to cluster other-cluster"));

        let error = PersistedNodeIdentity {
            format_version: NODE_IDENTITY_FORMAT_VERSION,
            cluster: DEFAULT_CLUSTER_NAME.to_owned(),
            node_id: node_id.to_string(),
            raft_node_id: raft_id.saturating_add(1),
        }
        .into_member_identity()
        .unwrap_err();
        assert!(error.to_string().contains("does not match node_id"));

        let dir = unique_test_dir("identity-errors");
        fs::create_dir_all(&dir).unwrap();
        let missing = dir.join("missing.json");
        let error = read_persisted_node_identity(&missing).unwrap_err();
        assert!(error.to_string().contains("failed to read node identity"));

        let malformed = dir.join("malformed.json");
        fs::write(&malformed, "{").unwrap();
        let error = read_persisted_node_identity(&malformed).unwrap_err();
        assert!(error.to_string().contains("failed to parse node identity"));

        let storage_file = dir.join("storage-is-a-file");
        fs::write(&storage_file, "occupied").unwrap();
        let error = resolve_member_identity(&test_member_config("127.0.0.1:7000"), &storage_file)
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("failed to create member storage directory"));

        let missing_parent = dir.join("missing-parent").join(NODE_IDENTITY_FILE);
        let error = write_node_identity_create_once(&missing_parent, "{}").unwrap_err();
        assert!(error
            .to_string()
            .contains("failed to write temporary node identity"));
        let _ = fs::remove_dir_all(dir);
    }

    #[tokio::test(start_paused = true)]
    async fn leader_wait_timeout_is_bounded_and_contextual() {
        let dir = unique_test_dir("leader-timeout");
        let raft = Arc::new(
            RaftMetadataRuntime::sled_with_config(
                RaftMetadataRuntimeConfig::try_joining(DEFAULT_CLUSTER_NAME, 2, [1]).unwrap(),
                &dir,
            )
            .unwrap(),
        );

        let error = wait_for_raft_leader(&raft).await.unwrap_err();

        assert!(error
            .to_string()
            .contains("timed out waiting for networked raft leader"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn immediate_poll_and_dedicated_runtime_lifecycle_cover_both_outcomes() {
        assert_eq!(poll_immediate(std::future::ready(7)).unwrap(), 7);
        let error = poll_immediate(std::future::pending::<()>()).unwrap_err();
        assert!(error.to_string().contains("unexpectedly yielded"));

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let runtime = DedicatedGridRuntime::new(runtime);
        assert_eq!(runtime.block_on(async { 11 }), 11);
        let (tx, rx) = std::sync::mpsc::channel();
        runtime.spawn(async move {
            tx.send(13).unwrap();
        });
        runtime.block_on(async { tokio::task::yield_now().await });
        assert_eq!(rx.recv_timeout(Duration::from_secs(1)).unwrap(), 13);
        drop(runtime);
    }

    #[tokio::test]
    async fn dedicated_runtime_can_be_dropped_inside_tokio() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        drop(DedicatedGridRuntime::new(runtime));
    }

    #[test]
    fn lightweight_handles_and_diagnostics_expose_failure_state() {
        let control_plane = Arc::new(RaftStyleMetadataControlPlane::new(DEFAULT_CLUSTER_NAME));
        let handle = InProcessGridHandle::new(control_plane, None);
        handle.begin_drain();
        assert_eq!(
            handle.reachability(&ClusterNodeId::from("unknown")),
            Reachability::Unreachable
        );
        assert!(format!("{handle:?}").contains("InProcessGridHandle"));

        let diagnostics = GridDriveDiagnostics::default();
        diagnostics.record_drive_error("drive failed".to_owned());
        let snapshot = diagnostics.snapshot();
        assert_eq!(snapshot.drive_errors, 1);
        assert_eq!(snapshot.last_error.as_deref(), Some("drive failed"));

        let handler = test_raft_handler(BTreeMap::new());
        assert_eq!(
            handler.resolve_wire_sender("local").unwrap(),
            handler.raft_node_id
        );
        assert!(format!("{handler:?}").contains("RaftClusterMessageHandler"));
    }

    #[tokio::test]
    async fn http_raft_sink_fails_loud_for_missing_peer_and_bad_headers() {
        let local = ClusterNodeId::from("local");
        let local_raft_id = raft_node_id(&local);
        let config = test_member_config("127.0.0.1:7000");
        let sink = HttpRaftMessageSink::new(
            local.clone(),
            local_raft_id,
            Arc::new(RwLock::new(BTreeMap::new())),
            ClusterRouteAuth::missing_provider().acknowledge_insecure_trust_boundary(true),
            &config,
        )
        .unwrap();
        let local_message = RaftWireMessage {
            from: local_raft_id,
            to: local_raft_id,
            term: 1,
            payload: Vec::new(),
        };
        sink.send(local_message).await.unwrap();
        assert_eq!(sink.node_id_for(local_raft_id), "local");
        assert_eq!(sink.node_id_for(99), "99");

        let error = sink
            .send(RaftWireMessage {
                from: local_raft_id,
                to: 99,
                term: 1,
                payload: Vec::new(),
            })
            .await
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("no HTTP raft peer endpoint for raft node 99"));

        let bad_auth = ClusterRouteAuth::secure(
            Arc::new(StaticNodeIdentityProvider::new(local, "key", "bad\nvalue")),
            Arc::new(AllowAllAuthorizer),
        );
        let bad_sink = HttpRaftMessageSink::new(
            ClusterNodeId::from("local"),
            local_raft_id,
            Arc::new(RwLock::new(BTreeMap::new())),
            bad_auth,
            &config,
        )
        .unwrap();
        let error = bad_sink.authenticated_headers().unwrap_err();
        assert!(error
            .to_string()
            .contains("failed to apply cluster auth headers"));
    }

    #[test]
    fn topology_log_and_identity_paths_fail_loud_on_invalid_inputs() {
        let dir = unique_test_dir("topology-errors");
        assert!(!raft_log_dir_has_state(&dir).unwrap());
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("not-a-directory");
        fs::write(&file, "occupied").unwrap();
        let error = raft_log_dir_has_state(&file).unwrap_err();
        assert!(error
            .to_string()
            .contains("failed to inspect raft log directory"));

        assert_eq!(valid_raft_endpoint("  "), None);
        assert_eq!(valid_raft_endpoint("0.0.0.0:7000"), None);
        assert_eq!(valid_raft_endpoint("127.0.0.1:0"), None);

        let mut peers = BTreeMap::new();
        insert_raft_peer(
            &mut peers,
            7,
            ClusterNodeId::from("member-a"),
            "127.0.0.1:7000".to_owned(),
        )
        .unwrap();
        insert_raft_peer(
            &mut peers,
            7,
            ClusterNodeId::from("member-a"),
            "127.0.0.1:7000".to_owned(),
        )
        .unwrap();
        assert_eq!(peers.len(), 1);

        let root = Path::new(std::path::MAIN_SEPARATOR_STR);
        assert!(root.parent().is_none());
        let error = write_node_identity_create_once(root, "{}").unwrap_err();
        assert!(error.to_string().contains("has no parent directory"));
        let _ = fs::remove_dir_all(dir);
    }

    struct PanicRaftMessageSink;

    #[async_trait::async_trait]
    impl RaftMessageSink for PanicRaftMessageSink {
        async fn send(&self, _message: RaftWireMessage) -> CacheResult<()> {
            panic!("intentional send-task panic")
        }
    }

    #[tokio::test]
    async fn send_task_panic_is_reported_in_diagnostics() {
        let sink: Arc<dyn RaftMessageSink> = Arc::new(PanicRaftMessageSink);
        let diagnostics = GridDriveDiagnostics::default();
        let error = send_raft_messages_with_diagnostics(
            &sink,
            vec![RaftWireMessage {
                from: 1,
                to: 2,
                term: 1,
                payload: Vec::new(),
            }],
            Some(&diagnostics),
        )
        .await
        .unwrap_err();

        assert!(error.to_string().contains("raft send task failed"));
        let snapshot = diagnostics.snapshot();
        assert_eq!(snapshot.send_failures, 1);
        assert!(snapshot
            .last_error
            .as_deref()
            .is_some_and(|error| error.contains("intentional send-task panic")));
    }

    #[test]
    fn seed_hash_collision_fails_loud_at_topology_build() {
        let mut peers = BTreeMap::new();
        insert_raft_peer(
            &mut peers,
            42,
            ClusterNodeId::from("member-a"),
            "127.0.0.1:7000".to_owned(),
        )
        .unwrap();

        let error = insert_raft_peer(
            &mut peers,
            42,
            ClusterNodeId::from("member-b"),
            "127.0.0.1:7001".to_owned(),
        )
        .unwrap_err();

        assert!(
            error.to_string().contains("raft node id collision"),
            "collision should fail loud: {error}"
        );
    }

    proptest! {
        #[test]
        fn wire_id_mapping_is_consistent_across_sink_and_handler(
            wire_from in "[0-9A-Za-z:_|-]{1,24}"
        ) {
            prop_assume!(wire_from != "local");
            let peer_node_id = ClusterNodeId::from(wire_from.clone());
            let peer_raft_id = raft_node_id(&peer_node_id);
            let handler = test_raft_handler(BTreeMap::from([(
                peer_raft_id,
                RaftPeer {
                    node_id: peer_node_id,
                    endpoint: "127.0.0.1:7001".to_owned(),
                },
            )]));

            prop_assert_eq!(
                handler.resolve_wire_sender(&wire_from).unwrap(),
                peer_raft_id
            );
        }
    }

    #[test]
    fn wire_id_mapping_fails_loud_for_unknown_sender() {
        let handler = test_raft_handler(BTreeMap::new());

        let error = handler.resolve_wire_sender("42").unwrap_err();

        assert!(
            error.to_string().contains("unknown raft wire sender 42"),
            "unknown sender should fail loud: {error}"
        );
    }

    #[test]
    fn http_raft_sink_attaches_cluster_auth_headers() {
        let node_id = ClusterNodeId::from("member-a");
        let auth = ClusterRouteAuth::secure(
            Arc::new(StaticNodeIdentityProvider::new(
                node_id.clone(),
                "k1",
                "secret",
            )),
            Arc::new(AllowAllAuthorizer),
        );
        let config = test_member_config("127.0.0.1:7000");
        let sink = HttpRaftMessageSink::new(
            node_id,
            1,
            Arc::new(RwLock::new(BTreeMap::new())),
            auth,
            &config,
        )
        .unwrap();

        let headers = sink.authenticated_headers().unwrap();

        assert_eq!(headers[HYDRACACHE_NODE_KEY_ID_HEADER], "k1");
        assert_eq!(headers[HYDRACACHE_NODE_TOKEN_HEADER], "secret");
    }

    #[tokio::test]
    async fn http_raft_sink_times_out_when_peer_accepts_without_reply() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let peer_addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (_stream, _addr) = listener.accept().await.unwrap();
            tokio::time::sleep(RAFT_HTTP_REQUEST_TIMEOUT + Duration::from_secs(2)).await;
        });
        let mut peers = BTreeMap::new();
        peers.insert(
            2,
            RaftPeer {
                node_id: ClusterNodeId::from("peer"),
                endpoint: peer_addr.to_string(),
            },
        );
        let sink = HttpRaftMessageSink::new(
            ClusterNodeId::from("local"),
            1,
            Arc::new(RwLock::new(peers)),
            ClusterRouteAuth::missing_provider().acknowledge_insecure_trust_boundary(true),
            &test_member_config("127.0.0.1:7000"),
        )
        .unwrap();
        let message = RaftWireMessage {
            from: 1,
            to: 2,
            term: 1,
            payload: Vec::new(),
        };

        let result =
            tokio::time::timeout(RAFT_HTTP_REQUEST_TIMEOUT + Duration::from_secs(1), async {
                sink.send(message).await
            })
            .await;

        server.abort();
        let error = result
            .expect("raft send should use the bounded HTTP client timeout")
            .expect_err("silent peer should time out");
        assert!(
            error.to_string().contains("failed to send raft message"),
            "timeout should be reported as a raft send failure: {error}"
        );
    }

    #[tokio::test]
    async fn snapshot_http_timeout_reports_failure_and_releases_inflight_feedback() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let peer_addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (_stream, _addr) = listener.accept().await.unwrap();
            tokio::time::sleep(RAFT_HTTP_REQUEST_TIMEOUT + Duration::from_secs(2)).await;
        });
        let peers = BTreeMap::from([(
            2,
            RaftPeer {
                node_id: ClusterNodeId::from("peer"),
                endpoint: peer_addr.to_string(),
            },
        )]);
        let diagnostics = Arc::new(GridDriveDiagnostics::default());
        let raft = test_raft_runtime();
        let sink = HttpRaftMessageSink::new(
            ClusterNodeId::from("local"),
            1,
            Arc::new(RwLock::new(peers)),
            ClusterRouteAuth::missing_provider().acknowledge_insecure_trust_boundary(true),
            &test_member_config("127.0.0.1:7000"),
        )
        .unwrap()
        .with_snapshot_feedback(raft, Arc::clone(&diagnostics));
        let mut snapshot = raft::eraftpb::Message {
            from: 1,
            to: 2,
            term: 1,
            ..Default::default()
        };
        snapshot.set_msg_type(raft::eraftpb::MessageType::MsgSnapshot);
        let message = RaftWireMessage::encode(&snapshot).unwrap();

        let send = tokio::spawn(async move { sink.send(message).await });
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if diagnostics.snapshot().snapshot_sends_in_flight == 1 {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("snapshot request should become observable while awaiting its HTTP outcome");

        let error = send
            .await
            .expect("snapshot send task should not panic")
            .expect_err("silent snapshot receiver should time out");
        server.abort();
        assert!(error.to_string().contains("failed to send raft message"));
        let snapshot = diagnostics.snapshot();
        assert_eq!(snapshot.snapshot_send_attempts, 1);
        assert_eq!(snapshot.snapshot_send_successes, 0);
        assert_eq!(snapshot.snapshot_send_failures, 1);
        assert_eq!(snapshot.snapshot_sends_in_flight, 0);
    }

    #[tokio::test]
    async fn sink_verifies_peer_against_configured_ca() {
        install_default_rustls_provider();
        let server_tls = write_test_tls_material("sink-ca/server");
        let wrong_tls = write_test_tls_material("sink-ca/wrong");
        let listener = StdTcpListener::bind("127.0.0.1:0").unwrap();
        let peer_addr = listener.local_addr().unwrap();
        listener.set_nonblocking(true).unwrap();
        let rustls_config =
            RustlsConfig::from_pem_file(&server_tls.cert_path, &server_tls.key_path)
                .await
                .unwrap();
        let handle = axum_server::Handle::new();
        let shutdown_handle = handle.clone();
        let server = axum_server::from_tcp_rustls(listener, rustls_config)
            .unwrap()
            .handle(handle)
            .serve(
                Router::new()
                    .route(
                        DEFAULT_RAFT_APPEND_PATH,
                        post(|| async { StatusCode::NO_CONTENT }),
                    )
                    .into_make_service(),
            );
        let server_task = tokio::spawn(async move { server.await.unwrap() });
        let auth = ClusterRouteAuth::missing_provider().acknowledge_insecure_trust_boundary(true);
        let mut peers = BTreeMap::new();
        peers.insert(
            2,
            RaftPeer {
                node_id: ClusterNodeId::from("peer"),
                endpoint: peer_addr.to_string(),
            },
        );
        let mut config = test_member_config("127.0.0.1:7000");
        config.tls = crate::config::TlsConfig {
            enabled: true,
            cert_path: Some(server_tls.cert_path.clone()),
            key_path: Some(server_tls.key_path.clone()),
            ca_path: Some(server_tls.ca_path.clone()),
            acknowledge_insecure: false,
        };
        let sink = HttpRaftMessageSink::new(
            ClusterNodeId::from("local"),
            1,
            Arc::new(RwLock::new(peers)),
            auth,
            &config,
        )
        .unwrap();
        assert_eq!(sink.scheme, "https");

        let message = RaftWireMessage {
            from: 1,
            to: 2,
            term: 1,
            payload: Vec::new(),
        };
        let ok = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if sink.send(message.clone()).await.is_ok() {
                    break true;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .unwrap();
        assert!(ok);

        let mut wrong_config = config;
        wrong_config.tls.ca_path = Some(wrong_tls.ca_path);
        let wrong_sink = HttpRaftMessageSink::new(
            ClusterNodeId::from("local"),
            1,
            sink.peers.clone(),
            ClusterRouteAuth::missing_provider().acknowledge_insecure_trust_boundary(true),
            &wrong_config,
        )
        .unwrap();
        let error = wrong_sink.send(message).await.unwrap_err();
        assert!(
            error.to_string().contains("failed to send raft message"),
            "wrong CA should fail during TLS verification: {error}"
        );

        shutdown_handle.graceful_shutdown(None);
        server_task.await.unwrap();
    }

    struct TestTlsMaterial {
        cert_path: PathBuf,
        key_path: PathBuf,
        ca_path: PathBuf,
    }

    fn write_test_tls_material(name: &str) -> TestTlsMaterial {
        let dir = PathBuf::from(format!("target/test-hydracache-grid-host/unit/{name}"));
        fs::create_dir_all(&dir).unwrap();
        let rcgen::CertifiedKey { cert, signing_key } =
            rcgen::generate_simple_self_signed(["127.0.0.1".to_owned(), "localhost".to_owned()])
                .unwrap();
        let cert_path = dir.join("cert.pem");
        let key_path = dir.join("key.pem");
        let ca_path = dir.join("ca.pem");
        fs::write(&cert_path, cert.pem()).unwrap();
        fs::write(&key_path, signing_key.serialize_pem()).unwrap();
        fs::write(&ca_path, cert.pem()).unwrap();
        TestTlsMaterial {
            cert_path,
            key_path,
            ca_path,
        }
    }

    fn unique_test_dir(name: &str) -> PathBuf {
        let sequence = NODE_IDENTITY_TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
        PathBuf::from(format!(
            "target/test-hydracache-grid-host/unit/{name}-{}-{sequence}",
            std::process::id()
        ))
    }

    fn test_member_config(cluster_addr: &str) -> ServerConfig {
        ServerConfig {
            role: crate::config::ServerRole::Member,
            listen_addr: "127.0.0.1:18080".parse().unwrap(),
            cluster_addr: cluster_addr.parse().unwrap(),
            node_id: None,
            seeds: vec![cluster_addr.to_owned()],
            storage_dir: Some(PathBuf::from("target/test-hydracache-grid-host/unit")),
            drain_timeout_ms: 1_000,
            tls: crate::config::TlsConfig::default(),
            cluster_auth: crate::config::ClusterAuthConfig::default(),
            backup: crate::config::BackupConfig::default(),
            client_api: crate::config::ClientApiConfig::default(),
            admin_api: crate::config::AdminApiConfig::default(),
            ..ServerConfig::default()
        }
    }
}
