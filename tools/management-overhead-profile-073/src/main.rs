use std::alloc::{GlobalAlloc, Layout, System};
use std::error::Error;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use hydracache_client_transport_axum::{
    HYDRACACHE_ADMIN_HEADER, HYDRACACHE_CLIENT_ID_HEADER, HYDRACACHE_TENANT_HEADER,
};
use hydracache_observability::MANAGEMENT_API_SCHEMA_VERSION;
use hydracache_server::{
    AdminHttpSurface, ClusterStatus, ClusterStatusProvider, ClusterStatusRuntime,
    LocalConsensusStatus, ManagementAggregationIssue, ManagementConsensusObservation,
    ManagementMemberSnapshot, ManagementPeerTarget, ManagementPeerTransport,
    ManagementSnapshotAggregator, ManagementSnapshotRequest, MemberRole, MemberStatus,
    Reachability, ReshardPhase, ServerConfig, ServerRole, ServerRuntime, StatusSource,
    MANAGEMENT_CAPABILITIES_PATH, MANAGEMENT_DASHBOARD_PATH, MANAGEMENT_FORMATION_PATH,
    MANAGEMENT_HISTORY_PATH, MANAGEMENT_MAX_RETAINED_CURSORS,
};
use serde::Serialize;
use tower::ServiceExt;

struct ProfilingAllocator;

static COUNTING: AtomicBool = AtomicBool::new(false);
static GROSS_ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);
static LIVE_ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);

#[global_allocator]
static GLOBAL_ALLOCATOR: ProfilingAllocator = ProfilingAllocator;

unsafe impl GlobalAlloc for ProfilingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the request is delegated unchanged to the system allocator.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            record_alloc(layout.size());
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the request is delegated unchanged to the system allocator.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            record_alloc(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        record_dealloc(layout.size());
        // SAFETY: pointer and layout came from this allocator and are unchanged.
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the request is delegated unchanged to the system allocator.
        let new_pointer = unsafe { System.realloc(pointer, layout, new_size) };
        if !new_pointer.is_null() {
            if new_size >= layout.size() {
                LIVE_ALLOCATED_BYTES
                    .fetch_add((new_size - layout.size()) as u64, Ordering::Relaxed);
            } else {
                subtract_live((layout.size() - new_size) as u64);
            }
            if COUNTING.load(Ordering::Acquire) {
                GROSS_ALLOCATED_BYTES.fetch_add(new_size as u64, Ordering::Relaxed);
            }
        }
        new_pointer
    }
}

fn record_alloc(bytes: usize) {
    LIVE_ALLOCATED_BYTES.fetch_add(bytes as u64, Ordering::Relaxed);
    if COUNTING.load(Ordering::Acquire) {
        GROSS_ALLOCATED_BYTES.fetch_add(bytes as u64, Ordering::Relaxed);
    }
}

fn record_dealloc(bytes: usize) {
    subtract_live(bytes as u64);
}

fn subtract_live(bytes: u64) {
    let _ = LIVE_ALLOCATED_BYTES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
        Some(current.saturating_sub(bytes))
    });
}

struct AllocationScope {
    before_live: u64,
}

impl AllocationScope {
    fn start() -> Self {
        GROSS_ALLOCATED_BYTES.store(0, Ordering::Relaxed);
        let before_live = LIVE_ALLOCATED_BYTES.load(Ordering::Relaxed);
        assert!(!COUNTING.swap(true, Ordering::AcqRel));
        Self { before_live }
    }

    fn finish(self) -> AllocationSnapshot {
        COUNTING.store(false, Ordering::Release);
        let live_allocated_bytes = LIVE_ALLOCATED_BYTES.load(Ordering::Relaxed);
        AllocationSnapshot {
            gross_allocated_bytes: GROSS_ALLOCATED_BYTES.load(Ordering::Relaxed),
            live_allocated_bytes,
            live_allocated_delta_bytes: live_allocated_bytes as i128 - self.before_live as i128,
        }
    }
}

impl Drop for AllocationScope {
    fn drop(&mut self) {
        COUNTING.store(false, Ordering::Release);
    }
}

#[derive(Clone, Copy, Serialize)]
struct AllocationSnapshot {
    gross_allocated_bytes: u64,
    live_allocated_bytes: u64,
    live_allocated_delta_bytes: i128,
}

#[derive(Clone, Copy, Serialize)]
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
    scenario: String,
    operations: usize,
    interval_milliseconds: u64,
    idle_observation_milliseconds: u64,
    gross_allocated_bytes: u64,
    live_allocated_bytes: u64,
    live_allocated_delta_bytes: i128,
    serialized_response_bytes: u64,
    request_elapsed_nanoseconds: u128,
    aggregate_transport_calls: u64,
    retained_cursor_records: usize,
    route_mounted: Option<bool>,
    expected_status_passed: bool,
    cache_invariant_passed: bool,
    cursor_bound_passed: bool,
    history_disabled_passed: bool,
    baseline: MemorySnapshot,
    plateau: MemorySnapshot,
}

struct Options {
    scenario: String,
    requests: usize,
    interval_ms: u64,
    idle_ms: u64,
}

impl Options {
    fn parse() -> Result<Self, Box<dyn Error>> {
        let mut args = std::env::args().skip(1);
        let mut scenario = None;
        let mut requests = 60;
        let mut interval_ms = 1_000;
        let mut idle_ms = 2_000;
        while let Some(argument) = args.next() {
            let value = args
                .next()
                .ok_or_else(|| format!("missing value after {argument}"))?;
            match argument.as_str() {
                "--scenario" => scenario = Some(value),
                "--requests" => requests = value.parse()?,
                "--interval-ms" => interval_ms = value.parse()?,
                "--idle-ms" => idle_ms = value.parse()?,
                _ => return Err(format!("unknown argument {argument}").into()),
            }
        }
        let scenario = scenario.ok_or("--scenario is required")?;
        if requests == 0 {
            return Err("--requests must be greater than zero".into());
        }
        Ok(Self {
            scenario,
            requests,
            interval_ms,
            idle_ms,
        })
    }
}

#[derive(Debug)]
struct StaticStatusProvider(Mutex<ClusterStatus>);

impl ClusterStatusProvider for StaticStatusProvider {
    fn cluster_status(&self, _runtime: ClusterStatusRuntime) -> ClusterStatus {
        self.0.lock().expect("status provider mutex").clone()
    }
}

#[derive(Debug, Default)]
struct CountingTransport {
    calls: AtomicU64,
}

#[async_trait::async_trait]
impl ManagementPeerTransport for CountingTransport {
    async fn fetch(
        &self,
        target: &ManagementPeerTarget,
        request: ManagementSnapshotRequest,
    ) -> Result<Vec<u8>, ManagementAggregationIssue> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        serde_json::to_vec(&ManagementMemberSnapshot {
            schema_version: MANAGEMENT_API_SCHEMA_VERSION,
            node_id: target.node_id.clone(),
            generation: target.generation,
            authority_epoch: request.authority_epoch,
            observation_seq: 19,
            consensus: Some(ManagementConsensusObservation {
                commit_index: 19,
                applied_index: 19,
                last_snapshot_index: Some(12),
                catch_up_target: 19,
                voter: true,
            }),
        })
        .map_err(|_| ManagementAggregationIssue::Malformed)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let options = Options::parse()?;
    let result = match options.scenario.as_str() {
        "management-off-idle" => profile_idle(false, &options).await?,
        "management-on-idle" => profile_idle(true, &options).await?,
        "dashboard-poll" => profile_http(MANAGEMENT_DASHBOARD_PATH, &options).await?,
        "aggregate-cold" => profile_aggregate(false, &options).await?,
        "aggregate-cache-hit" => profile_aggregate(true, &options).await?,
        "cursor-saturation" => profile_cursor_saturation(&options).await?,
        "history-disabled" => profile_http(history_path(), &options).await?,
        other => return Err(format!("unknown scenario {other}").into()),
    };
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}

async fn profile_idle(enabled: bool, options: &Options) -> Result<ProfileResult, Box<dyn Error>> {
    let baseline = process_memory()?;
    let scope = AllocationScope::start();
    let router = surface(enabled, live_status(3)).routes();
    tokio::time::sleep(Duration::from_millis(options.idle_ms)).await;
    let plateau = process_memory()?;
    let allocation = scope.finish();

    let response = router
        .oneshot(management_request(MANAGEMENT_CAPABILITIES_PATH))
        .await?;
    let route_mounted = response.status() != StatusCode::NOT_FOUND;
    let expected_status_passed = route_mounted == enabled;
    if !expected_status_passed {
        return Err("management route mount invariant failed".into());
    }
    Ok(result(
        options,
        1,
        allocation,
        baseline,
        plateau,
        0,
        0,
        0,
        0,
        Some(route_mounted),
        true,
        true,
        true,
        expected_status_passed,
    ))
}

async fn profile_http(path: &str, options: &Options) -> Result<ProfileResult, Box<dyn Error>> {
    let router = surface(true, live_status(3)).routes();
    let baseline = process_memory()?;
    let scope = AllocationScope::start();
    let started = Instant::now();
    let mut serialized_response_bytes = 0_u64;
    let mut expected_status_passed = true;
    let mut history_disabled_passed = true;
    for index in 0..options.requests {
        let response = router.clone().oneshot(management_request(path)).await?;
        expected_status_passed &= response.status() == StatusCode::OK;
        let body = to_bytes(response.into_body(), 1024 * 1024).await?;
        serialized_response_bytes = serialized_response_bytes.saturating_add(body.len() as u64);
        if path.starts_with(MANAGEMENT_HISTORY_PATH) {
            let json: serde_json::Value = serde_json::from_slice(&body)?;
            history_disabled_passed &= json["data"]["state"] == "no_adapter";
        }
        if index + 1 < options.requests && options.interval_ms != 0 {
            tokio::time::sleep(Duration::from_millis(options.interval_ms)).await;
        }
    }
    let request_elapsed_nanoseconds = started.elapsed().as_nanos();
    let plateau = process_memory()?;
    let allocation = scope.finish();
    if !expected_status_passed || !history_disabled_passed {
        return Err("HTTP profile invariant failed".into());
    }
    Ok(result(
        options,
        options.requests,
        allocation,
        baseline,
        plateau,
        serialized_response_bytes,
        request_elapsed_nanoseconds,
        0,
        0,
        Some(true),
        true,
        true,
        history_disabled_passed,
        expected_status_passed,
    ))
}

async fn profile_aggregate(
    cache_hit: bool,
    options: &Options,
) -> Result<ProfileResult, Box<dyn Error>> {
    let transport = Arc::new(CountingTransport::default());
    let local = local_snapshot();
    let targets = vec![remote_target()];
    let shared =
        cache_hit.then(|| {
            ManagementSnapshotAggregator::new(
                Arc::clone(&transport) as Arc<dyn ManagementPeerTransport>
            )
        });
    let baseline = process_memory()?;
    let scope = AllocationScope::start();
    let started = Instant::now();
    let mut serialized_response_bytes = 0_u64;
    for _ in 0..options.requests {
        let aggregate = if let Some(aggregator) = &shared {
            aggregator.refresh(local.clone(), targets.clone()).await
        } else {
            ManagementSnapshotAggregator::new(
                Arc::clone(&transport) as Arc<dyn ManagementPeerTransport>
            )
            .refresh(local.clone(), targets.clone())
            .await
        };
        let encoded = serde_json::to_vec(&serde_json::json!({
            "members": &aggregate.members,
            "failure_count": aggregate.failures.len(),
            "truncated": aggregate.truncated,
        }))?;
        serialized_response_bytes = serialized_response_bytes.saturating_add(encoded.len() as u64);
    }
    let request_elapsed_nanoseconds = started.elapsed().as_nanos();
    let plateau = process_memory()?;
    let allocation = scope.finish();
    let aggregate_transport_calls = transport.calls.load(Ordering::SeqCst);
    let expected_calls = if cache_hit {
        1
    } else {
        options.requests as u64
    };
    let cache_invariant_passed = aggregate_transport_calls == expected_calls;
    if !cache_invariant_passed {
        return Err(format!(
            "aggregate cache invariant failed: expected {expected_calls} calls, got {aggregate_transport_calls}"
        )
        .into());
    }
    Ok(result(
        options,
        options.requests,
        allocation,
        baseline,
        plateau,
        serialized_response_bytes,
        request_elapsed_nanoseconds,
        aggregate_transport_calls,
        0,
        None,
        cache_invariant_passed,
        true,
        true,
        true,
    ))
}

async fn profile_cursor_saturation(options: &Options) -> Result<ProfileResult, Box<dyn Error>> {
    let router = surface(true, live_status(250)).routes();
    let issue_count = MANAGEMENT_MAX_RETAINED_CURSORS + 1;
    let baseline = process_memory()?;
    let scope = AllocationScope::start();
    let started = Instant::now();
    let mut serialized_response_bytes = 0_u64;
    let mut first_cursor = None;
    let mut expected_status_passed = true;
    for _ in 0..issue_count {
        let response = router
            .clone()
            .oneshot(management_request(&format!(
                "{MANAGEMENT_FORMATION_PATH}?limit=1"
            )))
            .await?;
        expected_status_passed &= response.status() == StatusCode::OK;
        let body = to_bytes(response.into_body(), 1024 * 1024).await?;
        serialized_response_bytes = serialized_response_bytes.saturating_add(body.len() as u64);
        if first_cursor.is_none() {
            let json: serde_json::Value = serde_json::from_slice(&body)?;
            first_cursor = json["data"]["next_cursor"].as_str().map(str::to_owned);
        }
    }
    let first_cursor = first_cursor.ok_or("cursor response omitted next_cursor")?;
    let evicted = router
        .oneshot(management_request(&format!(
            "{MANAGEMENT_FORMATION_PATH}?limit=1&cursor={first_cursor}"
        )))
        .await?;
    let cursor_bound_passed = evicted.status() == StatusCode::BAD_REQUEST;
    let evicted_body = to_bytes(evicted.into_body(), 1024 * 1024).await?;
    serialized_response_bytes = serialized_response_bytes.saturating_add(evicted_body.len() as u64);
    let request_elapsed_nanoseconds = started.elapsed().as_nanos();
    let plateau = process_memory()?;
    let allocation = scope.finish();
    if !expected_status_passed || !cursor_bound_passed {
        return Err("cursor saturation invariant failed".into());
    }
    Ok(result(
        options,
        issue_count + 1,
        allocation,
        baseline,
        plateau,
        serialized_response_bytes,
        request_elapsed_nanoseconds,
        0,
        MANAGEMENT_MAX_RETAINED_CURSORS,
        Some(true),
        true,
        cursor_bound_passed,
        true,
        expected_status_passed,
    ))
}

#[allow(clippy::too_many_arguments)]
fn result(
    options: &Options,
    operations: usize,
    allocation: AllocationSnapshot,
    baseline: MemorySnapshot,
    plateau: MemorySnapshot,
    serialized_response_bytes: u64,
    request_elapsed_nanoseconds: u128,
    aggregate_transport_calls: u64,
    retained_cursor_records: usize,
    route_mounted: Option<bool>,
    cache_invariant_passed: bool,
    cursor_bound_passed: bool,
    history_disabled_passed: bool,
    expected_status_passed: bool,
) -> ProfileResult {
    ProfileResult {
        schema_version: 1,
        profile_id: "w6-management-overhead-profile-073-v1",
        scenario: options.scenario.clone(),
        operations,
        interval_milliseconds: options.interval_ms,
        idle_observation_milliseconds: options.idle_ms,
        gross_allocated_bytes: allocation.gross_allocated_bytes,
        live_allocated_bytes: allocation.live_allocated_bytes,
        live_allocated_delta_bytes: allocation.live_allocated_delta_bytes,
        serialized_response_bytes,
        request_elapsed_nanoseconds,
        aggregate_transport_calls,
        retained_cursor_records,
        route_mounted,
        expected_status_passed,
        cache_invariant_passed,
        cursor_bound_passed,
        history_disabled_passed,
        baseline,
        plateau,
    }
}

fn surface(enabled: bool, status: ClusterStatus) -> AdminHttpSurface {
    let provider: Arc<dyn ClusterStatusProvider> =
        Arc::new(StaticStatusProvider(Mutex::new(status)));
    let runtime = ServerRuntime::new(ServerConfig {
        role: ServerRole::Local,
        management_api_enabled: enabled,
        ..ServerConfig::default()
    })
    .expect("valid local profile config")
    .with_cluster_status_provider(provider)
    .start();
    AdminHttpSurface::new(runtime)
}

fn live_status(member_count: usize) -> ClusterStatus {
    let members = (0..member_count)
        .map(|index| MemberStatus {
            node_id: if index == 0 {
                "local-profile-node".to_owned()
            } else {
                format!("remote-profile-node-{index}")
            },
            role: if index == 0 {
                MemberRole::Local
            } else {
                MemberRole::Member
            },
            reachable: Reachability::Reachable,
            generation: index as u64 + 1,
        })
        .collect();
    ClusterStatus {
        source: StatusSource::Live,
        leader: Some("local-profile-node".to_owned()),
        term: 7,
        epoch: 42,
        observation_seq: 19,
        metadata_authoritative: true,
        quorum_ok: true,
        members,
        voters: member_count as u32,
        voter_ids: (1..=member_count as u64).collect(),
        reshard_phase: ReshardPhase::Idle,
        draining: false,
        local_consensus: Some(LocalConsensusStatus {
            node_id: "local-profile-node".to_owned(),
            generation: 1,
            voter: true,
            commit_index: 19,
            applied_index: 19,
            last_snapshot_index: Some(12),
            catch_up_target: 19,
        }),
    }
}

fn local_snapshot() -> ManagementMemberSnapshot {
    ManagementMemberSnapshot {
        schema_version: MANAGEMENT_API_SCHEMA_VERSION,
        node_id: "local-profile-node".to_owned(),
        generation: 1,
        authority_epoch: 42,
        observation_seq: 19,
        consensus: Some(ManagementConsensusObservation {
            commit_index: 19,
            applied_index: 19,
            last_snapshot_index: Some(12),
            catch_up_target: 19,
            voter: true,
        }),
    }
}

fn remote_target() -> ManagementPeerTarget {
    ManagementPeerTarget {
        node_id: "remote-profile-node-1".to_owned(),
        generation: 2,
        endpoint: "unused-profile-endpoint".to_owned(),
        management_schema_version: Some(MANAGEMENT_API_SCHEMA_VERSION),
    }
}

fn management_request(path: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(path)
        .header(HYDRACACHE_CLIENT_ID_HEADER, "profile-operator")
        .header(HYDRACACHE_TENANT_HEADER, "system")
        .header(HYDRACACHE_ADMIN_HEADER, "true")
        .body(Body::empty())
        .expect("valid management request")
}

fn history_path() -> &'static str {
    concat!(
        "/management/v1/history",
        "?query_id=cache_entries&start_ms=1000000&end_ms=1060000&step_ms=10000"
    )
}

#[cfg(windows)]
fn process_memory() -> Result<MemorySnapshot, Box<dyn Error>> {
    use std::mem::size_of;
    use windows_sys::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    let mut counters: PROCESS_MEMORY_COUNTERS = unsafe { std::mem::zeroed() };
    counters.cb = size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
    // SAFETY: counters points to writable storage with the declared size.
    if unsafe { GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb) } == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(MemorySnapshot {
        working_set_bytes: counters.WorkingSetSize as u64,
        peak_working_set_bytes: counters.PeakWorkingSetSize as u64,
        pagefile_bytes: counters.PagefileUsage as u64,
        peak_pagefile_bytes: counters.PeakPagefileUsage as u64,
    })
}

#[cfg(not(windows))]
fn process_memory() -> Result<MemorySnapshot, Box<dyn Error>> {
    Err("W6 local profile currently requires Windows process memory counters".into())
}
