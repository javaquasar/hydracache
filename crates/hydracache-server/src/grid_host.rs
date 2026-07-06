use std::collections::BTreeMap;
use std::env;
use std::fmt;
use std::fs;
use std::future::Future;
use std::net::{SocketAddr, TcpListener as StdTcpListener};
use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use std::time::Duration;

use axum_server::tls_rustls::RustlsConfig;
use hydracache::{
    CacheError, CacheResult, ClusterAdmissionBridge, ClusterDiscoveryEvent, ClusterGeneration,
    ClusterMember, ClusterNodeId, HydraCache, RaftMetadataSnapshot, RaftStyleMetadataControlPlane,
};
use hydracache_cluster_chitchat::{ChitchatDiscovery, ChitchatDiscoveryConfig};
use hydracache_cluster_raft::{
    DurableRaftLogDirectory, DurableRaftLogStore, InMemoryRaftMessageSink, RaftMessageSink,
    RaftMetadataRuntime, RaftMetadataRuntimeConfig, RaftWireMessage,
};
use hydracache_cluster_transport_axum::{
    tls::TlsStartupPolicy, AllowAllAuthorizer, AxumClusterMessageService, ClusterMessageAck,
    ClusterMessageHandler, ClusterOpaqueMessage, ClusterRoute, ClusterRouteAuth,
    StaticNodeIdentityProvider, DEFAULT_RAFT_APPEND_PATH,
};
use serde::{Deserialize, Serialize};
use tokio::sync::watch;

use crate::cluster_status::{GridControlPlaneHandle, Reachability, ReshardPhase};
use crate::config::{ServerConfig, ServerConfigError};

const DEFAULT_CLUSTER_NAME: &str = "hydracache";
const GRID_INPROC_ENV: &str = "HYDRACACHE_GRID_INPROC";
const GRID_DRIVE_INTERVAL: Duration = Duration::from_millis(50);
const GRID_LEADER_WAIT_TIMEOUT: Duration = Duration::from_secs(5);
const NODE_IDENTITY_FILE: &str = "node-identity.json";
const NODE_IDENTITY_FORMAT_VERSION: u32 = 1;

type NetworkedRaftRuntime = RaftMetadataRuntime<DurableRaftLogStore>;
type SharedRaftPeers = Arc<RwLock<BTreeMap<u64, RaftPeer>>>;

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
    let generation = ClusterGeneration::new(1);
    let raft_node_id = identity.raft_node_id;
    let topology = raft_topology(config, node_id.clone(), raft_node_id)?;
    let raft_log_dir = storage_dir.join("raft-log");
    fs::create_dir_all(&raft_log_dir).map_err(|error| {
        CacheError::Backend(format!(
            "failed to create raft log directory {}: {error}",
            raft_log_dir.display()
        ))
    })?;

    let raft_config = RaftMetadataRuntimeConfig::multi_voter(
        DEFAULT_CLUSTER_NAME,
        raft_node_id,
        topology.voters.clone(),
    )
    .auto_campaign(!topology.multi_voter)
    .ticks(topology.election_tick_for(raft_node_id), 1);
    let raft = Arc::new(RaftMetadataRuntime::durable_with_config(
        raft_config,
        DurableRaftLogDirectory::new(),
    )?);
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
    let bridge = ClusterAdmissionBridge::new(discovery.clone(), raft.clone());
    let route_auth = cluster_route_auth(config, &node_id)?;
    let raft_peers = Arc::new(RwLock::new((*topology.peers).clone()));
    let message_sink: Arc<dyn RaftMessageSink> = if topology.multi_voter {
        Arc::new(HttpRaftMessageSink::new(
            node_id.clone(),
            raft_node_id,
            raft_peers.clone(),
            route_auth.clone(),
            config,
        )?)
    } else {
        Arc::new(InMemoryRaftMessageSink::default())
    };
    let (shutdown, _) = watch::channel(false);
    spawn_grid_drive(
        raft.clone(),
        bridge.clone(),
        message_sink.clone(),
        raft_peers.clone(),
        node_id.clone(),
        config.cluster_addr,
        shutdown.subscribe(),
    );
    spawn_cluster_transport(
        config,
        node_id.clone(),
        raft.clone(),
        message_sink.clone(),
        route_auth,
        shutdown.subscribe(),
    )
    .await?;
    if topology.multi_voter {
        if raft_node_id == topology.bootstrap_raft_node_id {
            let _ = send_raft_messages(&message_sink, raft.campaign()?).await;
        }
        wait_for_raft_leader(&raft).await?;
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
        draining: Arc::new(AtomicBool::new(false)),
        drain_remove_proposed: Arc::new(AtomicBool::new(false)),
        shutdown,
    })
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
        .node_id(node_id)
        .generation(generation)
        .bind(config.cluster_addr.to_string())
        .diagnostics_endpoint(format!("http://{}", config.admin_api.listen_addr));
    for seed in &config.seeds {
        builder = builder.bootstrap(seed.clone());
    }
    builder.start().await
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
        }),
        auth,
    )
    .routes();
    if config.tls.enabled {
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

fn spawn_grid_drive(
    raft: Arc<NetworkedRaftRuntime>,
    bridge: ClusterAdmissionBridge,
    message_sink: Arc<dyn RaftMessageSink>,
    raft_peers: SharedRaftPeers,
    local_node_id: ClusterNodeId,
    local_addr: SocketAddr,
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
                    if let Err(_error) = drive_grid_once(
                        &raft,
                        &bridge,
                        &message_sink,
                        &raft_peers,
                        &local_node_id,
                        local_addr,
                    ).await {
                        continue;
                    }
                }
            }
        }
    });
}

async fn drive_grid_once(
    raft: &Arc<NetworkedRaftRuntime>,
    bridge: &ClusterAdmissionBridge,
    message_sink: &Arc<dyn RaftMessageSink>,
    raft_peers: &SharedRaftPeers,
    local_node_id: &ClusterNodeId,
    local_addr: SocketAddr,
) -> CacheResult<()> {
    let _ = send_raft_messages(message_sink, raft.tick()?).await;
    let _ = bridge.run_once().await;
    refresh_raft_peers(raft_peers, local_node_id, local_addr, &raft.members());
    sync_raft_voters(raft, message_sink, raft_peers).await?;
    let _ = send_raft_messages(message_sink, raft.take_outbound_messages()).await;
    let _ = send_raft_messages(message_sink, raft.drain_ready()?).await;
    Ok(())
}

fn refresh_raft_peers(
    raft_peers: &SharedRaftPeers,
    local_node_id: &ClusterNodeId,
    local_addr: SocketAddr,
    members: &[ClusterMember],
) {
    let mut peers = raft_peers.write().expect("raft peer map poisoned");
    peers.insert(
        raft_node_id(local_node_id),
        RaftPeer {
            node_id: local_node_id.clone(),
            address: local_addr,
        },
    );
    for member in members {
        if !member.is_member() {
            continue;
        }
        let Some(address) = member
            .endpoints
            .control
            .as_deref()
            .and_then(|endpoint| endpoint.parse::<SocketAddr>().ok())
        else {
            continue;
        };
        peers.insert(
            raft_node_id(&member.node_id),
            RaftPeer {
                node_id: member.node_id.clone(),
                address,
            },
        );
    }
}

async fn sync_raft_voters(
    raft: &Arc<NetworkedRaftRuntime>,
    message_sink: &Arc<dyn RaftMessageSink>,
    raft_peers: &SharedRaftPeers,
) -> CacheResult<()> {
    let snapshot = raft.snapshot();
    if raft.leader_id() != Some(snapshot.raft_node_id) {
        return Ok(());
    }
    let current_voters = raft.voter_ids()?;
    for member in raft.members() {
        if !member.is_member() {
            continue;
        }
        let raft_id = raft_node_id(&member.node_id);
        if current_voters.contains(&raft_id) {
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
        send_raft_messages(message_sink, outbound).await?;
        break;
    }
    Ok(())
}

async fn send_raft_messages(
    message_sink: &Arc<dyn RaftMessageSink>,
    messages: Vec<RaftWireMessage>,
) -> CacheResult<()> {
    let mut last_error = None;
    for message in messages {
        if let Err(error) = message_sink.send(message).await {
            last_error = Some(error.to_string());
        }
    }
    if let Some(error) = last_error {
        return Err(CacheError::Backend(format!(
            "one or more raft messages failed: {error}"
        )));
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct RaftPeer {
    node_id: ClusterNodeId,
    address: SocketAddr,
}

#[derive(Debug, Clone)]
struct RaftTopology {
    voters: Vec<u64>,
    peers: Arc<BTreeMap<u64, RaftPeer>>,
    multi_voter: bool,
    bootstrap_raft_node_id: u64,
}

fn raft_topology(
    config: &ServerConfig,
    local_node_id: ClusterNodeId,
    local_raft_node_id: u64,
) -> CacheResult<RaftTopology> {
    let mut peers = BTreeMap::new();
    if config.cluster_addr.port() != 0 {
        insert_raft_peer(
            &mut peers,
            local_raft_node_id,
            local_node_id,
            config.cluster_addr,
        )?;
        for seed in &config.seeds {
            if let Ok(address) = seed.parse::<SocketAddr>() {
                if address.port() == 0 || address == config.cluster_addr {
                    continue;
                }
                let node_id = member_node_id_for_addr(address);
                let raft_id = raft_node_id(&node_id);
                insert_raft_peer(&mut peers, raft_id, node_id, address)?;
            }
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
    address: SocketAddr,
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
    peers.insert(raft_id, RaftPeer { node_id, address });
    Ok(())
}

impl RaftTopology {
    fn election_tick_for(&self, raft_node_id: u64) -> usize {
        let rank = self
            .voters
            .iter()
            .position(|voter| *voter == raft_node_id)
            .unwrap_or(0);
        5 + (rank * 2)
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
        })
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
}

#[async_trait::async_trait]
impl RaftMessageSink for HttpRaftMessageSink {
    async fn send(&self, message: RaftWireMessage) -> CacheResult<()> {
        if message.to == self.local_raft_node_id {
            return Ok(());
        }
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
                self.scheme, peer.address, DEFAULT_RAFT_APPEND_PATH
            ))
            .headers(headers)
            .json(&request)
            .send()
            .await
            .map_err(|error| {
                CacheError::Backend(format!(
                    "failed to send raft message to {}: {error}",
                    peer.node_id
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

fn raft_http_client(config: &ServerConfig) -> CacheResult<(&'static str, reqwest::Client)> {
    if !config.tls.enabled {
        return Ok(("http", reqwest::Client::new()));
    }
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
    let client = reqwest::Client::builder()
        .add_root_certificate(certificate)
        .build()
        .map_err(|error| {
            CacheError::Backend(format!("failed to build TLS raft client: {error}"))
        })?;
    Ok(("https", client))
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
        let persisted = read_persisted_node_identity(&path)?;
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
        return Ok(identity);
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
    fs::write(&path, text).map_err(|error| {
        CacheError::Backend(format!(
            "failed to write node identity {}: {error}",
            path.display()
        ))
    })?;
    Ok(MemberIdentity {
        node_id,
        raft_node_id,
    })
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

fn raft_wire_node_id(node_id: &str) -> u64 {
    node_id
        .parse::<u64>()
        .unwrap_or_else(|_| stable_nonzero_hash(node_id))
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
    draining: Arc<AtomicBool>,
    drain_remove_proposed: Arc<AtomicBool>,
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
    draining: Arc<AtomicBool>,
    drain_remove_proposed: Arc<AtomicBool>,
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
            draining: stack.draining,
            drain_remove_proposed: stack.drain_remove_proposed,
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
}

impl Drop for DedicatedGridRuntime {
    fn drop(&mut self) {
        if let Some(runtime) = self
            .runtime
            .lock()
            .expect("grid runtime holder poisoned")
            .take()
        {
            runtime.shutdown_background();
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
        reachable >= (voters.len() / 2).saturating_add(1)
    }

    fn reachability(&self, node: &ClusterNodeId) -> Reachability {
        if node == &self.node_id {
            return Reachability::Reachable;
        }

        for event in self.discovery.events().into_iter().rev() {
            match event {
                ClusterDiscoveryEvent::MemberLive(event_node) if event_node == *node => {
                    return Reachability::Reachable;
                }
                ClusterDiscoveryEvent::MemberSuspect(event_node) if event_node == *node => {
                    return Reachability::Suspect;
                }
                ClusterDiscoveryEvent::MemberDead(event_node) if event_node == *node => {
                    return Reachability::Unreachable;
                }
                _ => {}
            }
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
        if self.raft.leader_id() != Some(self.raft_node_id) {
            return;
        }
        let Ok(messages) = self.raft.propose_remove_voter(self.raft_node_id) else {
            return;
        };
        self.drain_remove_proposed.store(true, Ordering::SeqCst);
        if let Some(runtime) = &self._runtime {
            let _ = runtime.block_on(send_raft_messages(&self._message_sink, messages));
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

#[derive(Clone)]
struct RaftClusterMessageHandler {
    node_id: ClusterNodeId,
    raft_node_id: u64,
    raft: Arc<NetworkedRaftRuntime>,
    message_sink: Arc<dyn RaftMessageSink>,
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
            from: raft_wire_node_id(&message.from),
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{http::StatusCode, routing::post, Router};
    use hydracache::{
        ClusterCandidate, ClusterControlPlane, ClusterEndpoints, ClusterEpoch, ClusterRole,
        InMemoryClusterDiscovery,
    };
    use hydracache_cluster_transport_axum::{
        HYDRACACHE_NODE_KEY_ID_HEADER, HYDRACACHE_NODE_TOKEN_HEADER,
    };
    use std::path::PathBuf;

    #[tokio::test]
    async fn drive_loop_admits_a_gossip_candidate_into_the_shared_raft_runtime() {
        let raft = Arc::new(
            RaftMetadataRuntime::durable(DEFAULT_CLUSTER_NAME, 1, DurableRaftLogDirectory::new())
                .unwrap(),
        );
        let discovery = Arc::new(InMemoryClusterDiscovery::new());
        let bridge = ClusterAdmissionBridge::new(discovery.clone(), raft.clone());
        let message_sink: Arc<dyn RaftMessageSink> = Arc::new(InMemoryRaftMessageSink::default());
        let raft_peers = Arc::new(RwLock::new(BTreeMap::new()));

        discovery
            .announce(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)));
        drive_grid_once(
            &raft,
            &bridge,
            &message_sink,
            &raft_peers,
            &ClusterNodeId::from("local"),
            "127.0.0.1:7000".parse().unwrap(),
        )
        .await
        .unwrap();

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
            "127.0.0.1:7000".parse().unwrap(),
            &[member],
        );

        let peers = raft_peers.read().expect("raft peer map poisoned");
        assert_eq!(
            peers
                .get(&raft_node_id(&local_node))
                .map(|peer| peer.address),
            Some("127.0.0.1:7000".parse().unwrap())
        );
        assert_eq!(
            peers
                .get(&raft_node_id(&member_node))
                .map(|peer| peer.address),
            Some("127.0.0.1:7001".parse().unwrap())
        );
    }

    #[tokio::test]
    async fn sync_raft_voters_adds_admitted_member_with_known_peer() {
        let raft = Arc::new(
            RaftMetadataRuntime::durable(DEFAULT_CLUSTER_NAME, 1, DurableRaftLogDirectory::new())
                .unwrap(),
        );
        let message_sink: Arc<dyn RaftMessageSink> = Arc::new(InMemoryRaftMessageSink::default());
        let member_node = ClusterNodeId::from("member-a");
        let member_raft_id = raft_node_id(&member_node);
        let raft_peers = Arc::new(RwLock::new(BTreeMap::from([(
            member_raft_id,
            RaftPeer {
                node_id: member_node.clone(),
                address: "127.0.0.1:7001".parse().unwrap(),
            },
        )])));

        raft.join_member(
            ClusterCandidate::member(member_node)
                .generation(ClusterGeneration::new(1))
                .endpoints(ClusterEndpoints::new().control("127.0.0.1:7001")),
        )
        .await
        .unwrap();
        sync_raft_voters(&raft, &message_sink, &raft_peers)
            .await
            .unwrap();

        assert!(raft.voter_ids().unwrap().contains(&member_raft_id));
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
    fn seed_hash_collision_fails_loud_at_topology_build() {
        let mut peers = BTreeMap::new();
        insert_raft_peer(
            &mut peers,
            42,
            ClusterNodeId::from("member-a"),
            "127.0.0.1:7000".parse().unwrap(),
        )
        .unwrap();

        let error = insert_raft_peer(
            &mut peers,
            42,
            ClusterNodeId::from("member-b"),
            "127.0.0.1:7001".parse().unwrap(),
        )
        .unwrap_err();

        assert!(
            error.to_string().contains("raft node id collision"),
            "collision should fail loud: {error}"
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
    async fn sink_verifies_peer_against_configured_ca() {
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
                address: peer_addr,
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
        }
    }
}
