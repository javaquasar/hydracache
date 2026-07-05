use std::env;
use std::fmt;
use std::fs;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use std::time::Duration;

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
    AxumClusterMessageService, ClusterMessageAck, ClusterMessageHandler, ClusterOpaqueMessage,
    ClusterRoute, ClusterRouteAuth,
};
use tokio::net::TcpListener;
use tokio::sync::watch;

use crate::cluster_status::{GridControlPlaneHandle, Reachability, ReshardPhase};
use crate::config::{ServerConfig, ServerConfigError};

const DEFAULT_CLUSTER_NAME: &str = "hydracache";
const GRID_INPROC_ENV: &str = "HYDRACACHE_GRID_INPROC";
const GRID_DRIVE_INTERVAL: Duration = Duration::from_millis(50);

type NetworkedRaftRuntime = RaftMetadataRuntime<DurableRaftLogStore>;

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
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        let stack = tokio::task::block_in_place(|| handle.block_on(networked_member_stack(config)))
            .map_err(|error| ServerConfigError::GridHostStart(error.to_string()))?;
        let cache = stack.cache.clone();
        return Ok((cache, Arc::new(NetworkedGridHandle::new(stack, None))));
    }

    let runtime = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .thread_name("hydracache-grid-host")
            .enable_all()
            .build()
            .map_err(|error| ServerConfigError::GridHostStart(error.to_string()))?,
    );
    let stack = runtime
        .block_on(networked_member_stack(config))
        .map_err(|error| ServerConfigError::GridHostStart(error.to_string()))?;
    let cache = stack.cache.clone();
    Ok((
        cache,
        Arc::new(NetworkedGridHandle::new(stack, Some(runtime))),
    ))
}

async fn networked_member_stack(config: &ServerConfig) -> CacheResult<NetworkedMemberStack> {
    let node_id = member_node_id(config);
    let generation = ClusterGeneration::new(1);
    let raft_node_id = raft_node_id(&node_id);
    let storage_dir = config.storage_dir.as_ref().ok_or_else(|| {
        CacheError::Backend("member role requires storage_dir before grid host startup".to_owned())
    })?;
    let raft_log_dir = storage_dir.join("raft-log");
    fs::create_dir_all(&raft_log_dir).map_err(|error| {
        CacheError::Backend(format!(
            "failed to create raft log directory {}: {error}",
            raft_log_dir.display()
        ))
    })?;

    let raft_config =
        RaftMetadataRuntimeConfig::multi_voter(DEFAULT_CLUSTER_NAME, raft_node_id, [raft_node_id])
            .auto_campaign(true);
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
    let message_sink: Arc<dyn RaftMessageSink> = Arc::new(InMemoryRaftMessageSink::default());

    let cache = networked_member_cache(
        config,
        raft.clone(),
        discovery.clone(),
        node_id.clone(),
        generation,
    )
    .await?;

    let (shutdown, _) = watch::channel(false);
    spawn_grid_drive(
        raft.clone(),
        bridge.clone(),
        message_sink.clone(),
        shutdown.subscribe(),
    );
    spawn_cluster_transport(
        config,
        node_id.clone(),
        raft.clone(),
        message_sink.clone(),
        shutdown.subscribe(),
    )
    .await?;

    Ok(NetworkedMemberStack {
        cache,
        node_id,
        raft_node_id,
        raft,
        discovery,
        bridge,
        message_sink,
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
    mut shutdown: watch::Receiver<bool>,
) -> CacheResult<()> {
    let listener = TcpListener::bind(config.cluster_addr)
        .await
        .map_err(|error| {
            CacheError::Backend(format!("failed to bind cluster transport: {error}"))
        })?;
    let auth = ClusterRouteAuth::missing_provider().acknowledge_insecure_trust_boundary(true);
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
    tokio::spawn(async move {
        let shutdown_signal = async move {
            loop {
                if *shutdown.borrow() {
                    break;
                }
                if shutdown.changed().await.is_err() {
                    break;
                }
            }
        };
        let _ = axum::serve(listener, routes)
            .with_graceful_shutdown(shutdown_signal)
            .await;
    });
    Ok(())
}

fn spawn_grid_drive(
    raft: Arc<NetworkedRaftRuntime>,
    bridge: ClusterAdmissionBridge,
    message_sink: Arc<dyn RaftMessageSink>,
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
                    if let Err(_error) = drive_grid_once(&raft, &bridge, &message_sink).await {
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
) -> CacheResult<()> {
    send_raft_messages(message_sink, raft.tick()?).await?;
    let _ = bridge.run_once().await;
    send_raft_messages(message_sink, raft.take_outbound_messages()).await?;
    send_raft_messages(message_sink, raft.drain_ready()?).await
}

async fn send_raft_messages(
    message_sink: &Arc<dyn RaftMessageSink>,
    messages: Vec<RaftWireMessage>,
) -> CacheResult<()> {
    for message in messages {
        message_sink.send(message).await?;
    }
    Ok(())
}

fn member_node_id(config: &ServerConfig) -> ClusterNodeId {
    let suffix = config
        .cluster_addr
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
    raft: Arc<NetworkedRaftRuntime>,
    discovery: Arc<ChitchatDiscovery>,
    bridge: ClusterAdmissionBridge,
    message_sink: Arc<dyn RaftMessageSink>,
    shutdown: watch::Sender<bool>,
}

struct NetworkedGridHandle {
    node_id: ClusterNodeId,
    raft_node_id: u64,
    raft: Arc<NetworkedRaftRuntime>,
    discovery: Arc<ChitchatDiscovery>,
    _bridge: ClusterAdmissionBridge,
    _message_sink: Arc<dyn RaftMessageSink>,
    shutdown: watch::Sender<bool>,
    _runtime: Option<Arc<tokio::runtime::Runtime>>,
}

impl NetworkedGridHandle {
    fn new(stack: NetworkedMemberStack, runtime: Option<Arc<tokio::runtime::Runtime>>) -> Self {
        Self {
            node_id: stack.node_id,
            raft_node_id: stack.raft_node_id,
            raft: stack.raft,
            discovery: stack.discovery,
            _bridge: stack.bridge,
            _message_sink: stack.message_sink,
            shutdown: stack.shutdown,
            _runtime: runtime,
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
                format!("raft-{leader}")
            }
        })
    }

    fn has_quorum(&self) -> bool {
        self.raft.leader_id().is_some() && !self.raft.members().is_empty()
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
        false
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
    use hydracache::{ClusterCandidate, InMemoryClusterDiscovery};

    #[tokio::test]
    async fn drive_loop_admits_a_gossip_candidate_into_the_shared_raft_runtime() {
        let raft = Arc::new(
            RaftMetadataRuntime::durable(DEFAULT_CLUSTER_NAME, 1, DurableRaftLogDirectory::new())
                .unwrap(),
        );
        let discovery = Arc::new(InMemoryClusterDiscovery::new());
        let bridge = ClusterAdmissionBridge::new(discovery.clone(), raft.clone());
        let message_sink: Arc<dyn RaftMessageSink> = Arc::new(InMemoryRaftMessageSink::default());

        discovery
            .announce(ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)));
        drive_grid_once(&raft, &bridge, &message_sink)
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
}
