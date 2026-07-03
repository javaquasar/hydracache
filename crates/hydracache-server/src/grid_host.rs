use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use hydracache::{
    ClusterGeneration, ClusterMember, ClusterNodeId, HydraCache, RaftMetadataSnapshot,
    RaftStyleMetadataControlPlane,
};

use crate::cluster_status::{GridControlPlaneHandle, Reachability, ReshardPhase};
use crate::config::{ServerConfig, ServerConfigError};

const DEFAULT_CLUSTER_NAME: &str = "hydracache";

/// Build the in-process grid-mode cache used by a member-role daemon.
pub(crate) fn build_member(
    config: &ServerConfig,
) -> Result<(HydraCache, Arc<dyn GridControlPlaneHandle>), ServerConfigError> {
    let control_plane =
        Arc::new(RaftStyleMetadataControlPlane::new(DEFAULT_CLUSTER_NAME).with_term(1));
    let (cache, runtime) = start_member_cache(config, control_plane.clone())?;
    Ok((
        cache,
        Arc::new(InProcessGridHandle::new(control_plane, runtime)),
    ))
}

async fn member_cache(
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

fn start_member_cache(
    config: &ServerConfig,
    control_plane: Arc<RaftStyleMetadataControlPlane>,
) -> Result<(HydraCache, Option<Arc<tokio::runtime::Runtime>>), ServerConfigError> {
    if tokio::runtime::Handle::try_current().is_ok() {
        let cache = poll_immediate(member_cache(config, control_plane))?
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
        .block_on(member_cache(config, control_plane))
        .map_err(|error| ServerConfigError::GridHostStart(error.to_string()))?;
    Ok((cache, Some(runtime)))
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
