use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::options;
use axum::{Json, Router};
use hydracache_sim::{
    replay_world, run_scenario_with, ControlActionV1, ControlApplyError, ReplayScriptV1,
    ScenarioError, SimConfig, SimMode, SimSnapshot, SimWorld, SIM_SNAPSHOT_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};

use crate::SandboxState;

#[derive(Debug, Clone, Deserialize)]
struct SimNewRequest {
    seed: Option<u64>,
    steps: Option<u64>,
    scenario: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct SimStepRequest {
    steps: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum SimInjectRequest {
    Workload {
        enabled: bool,
    },
    Crash {
        node: String,
    },
    Restart {
        node: String,
    },
    Partition {
        from: String,
        to: String,
    },
    Heal {
        from: String,
        to: String,
    },
    Isolate {
        node: String,
    },
    Rejoin {
        node: String,
    },
    Disable {
        node: String,
    },
    Enable {
        node: String,
    },
    AddNode,
    ModeChange {
        mode: SimMode,
    },
    Drop {
        from: String,
        to: String,
    },
    Delay {
        from: String,
        to: String,
        millis: u64,
    },
    PushEvent {
        client: String,
        ns: String,
        key: String,
        value: String,
    },
    Subscribe {
        client: String,
        ns: String,
    },
    AddClient {
        client: String,
        ns: String,
    },
    Unsubscribe {
        client: String,
        ns: String,
    },
    RemoveClient {
        client: String,
    },
}

#[derive(Debug, Clone, Serialize)]
struct SimRouteError {
    code: &'static str,
    message: String,
}

type SimRouteResult =
    Result<([(&'static str, &'static str); 3], Json<SimSnapshot>), SimRouteRejection>;

/// Build the sandbox simulator routes.
pub(super) fn router() -> Router<SandboxState> {
    Router::new()
        .route("/sim/new", options(sim_options).post(sim_new))
        .route("/sim/step", options(sim_options).post(sim_step))
        .route("/sim/inject", options(sim_options).post(sim_inject))
        .route("/sim/control", options(sim_options).post(sim_control))
        .route("/sim/snapshot", options(sim_options).get(sim_snapshot))
        .route("/sim/replay", options(sim_options).get(sim_replay))
}

async fn sim_new(
    State(state): State<SandboxState>,
    Json(request): Json<SimNewRequest>,
) -> SimRouteResult {
    let mut world = if let Some(scenario) = request.scenario.as_deref() {
        if scenario == "default" {
            SimWorld::with_raft_election(request.seed.unwrap_or(50), SimConfig::default())
        } else {
            // Same election backend as a default server run, so a curated
            // scenario does not silently fall back to the simulator model.
            run_scenario_with(scenario, |seed| {
                SimWorld::with_raft_election(seed, SimConfig::default())
            })
            .map_err(SimRouteRejection::from)?
            .world
        }
    } else {
        SimWorld::with_raft_election(request.seed.unwrap_or(50), SimConfig::default())
    };
    if let Some(steps) = request.steps {
        let current_step = world.snapshot().step;
        if steps > current_step {
            world.run(steps - current_step);
        }
    }
    let snapshot = world.snapshot();
    assert_current_snapshot_schema(&snapshot);
    *state.sim_world.write().await = world;
    Ok((cors_headers(), Json(snapshot)))
}

async fn sim_step(
    State(state): State<SandboxState>,
    Json(request): Json<SimStepRequest>,
) -> SimRouteResult {
    let mut world = state.sim_world.write().await;
    world.run(request.steps.unwrap_or(1));
    let snapshot = world.snapshot();
    assert_current_snapshot_schema(&snapshot);
    Ok((cors_headers(), Json(snapshot)))
}

async fn sim_inject(
    State(state): State<SandboxState>,
    Json(request): Json<SimInjectRequest>,
) -> SimRouteResult {
    let mut world = state.sim_world.write().await;
    apply_injection(&mut world, request)?;
    let snapshot = world.snapshot();
    assert_current_snapshot_schema(&snapshot);
    Ok((cors_headers(), Json(snapshot)))
}

async fn sim_control(
    State(state): State<SandboxState>,
    Json(script): Json<ReplayScriptV1>,
) -> SimRouteResult {
    let world = replay_world(&script, |seed| {
        SimWorld::with_raft_election(seed, SimConfig::default())
    })?;
    let snapshot = world.snapshot();
    assert_current_snapshot_schema(&snapshot);
    *state.sim_world.write().await = world;
    Ok((cors_headers(), Json(snapshot)))
}

async fn sim_snapshot(
    State(state): State<SandboxState>,
) -> ([(&'static str, &'static str); 3], Json<SimSnapshot>) {
    (
        cors_headers(),
        Json(state.sim_world.read().await.snapshot()),
    )
}

/// Return the replay script for the current world, so a server-backed lab run
/// can be shared and replayed with every recorded intervention.
async fn sim_replay(
    State(state): State<SandboxState>,
) -> ([(&'static str, &'static str); 3], Json<ReplayScriptV1>) {
    (
        cors_headers(),
        Json(state.sim_world.read().await.replay_script()),
    )
}

async fn sim_options() -> impl IntoResponse {
    (StatusCode::NO_CONTENT, cors_headers())
}

fn apply_injection(
    world: &mut SimWorld,
    request: SimInjectRequest,
) -> Result<(), SimRouteRejection> {
    let at_step = world.outcome().steps;
    let action = match request {
        SimInjectRequest::Workload { enabled } => ControlActionV1::Workload { at_step, enabled },
        SimInjectRequest::Crash { node } => ControlActionV1::Crash { at_step, node },
        SimInjectRequest::Restart { node } => ControlActionV1::Restart { at_step, node },
        SimInjectRequest::Partition { from, to } => {
            ControlActionV1::Partition { at_step, from, to }
        }
        SimInjectRequest::Heal { from, to } => ControlActionV1::Heal { at_step, from, to },
        SimInjectRequest::Isolate { node } => ControlActionV1::Isolate { at_step, node },
        SimInjectRequest::Rejoin { node } => ControlActionV1::Rejoin { at_step, node },
        SimInjectRequest::Disable { node } => ControlActionV1::Disable { at_step, node },
        SimInjectRequest::Enable { node } => ControlActionV1::Enable { at_step, node },
        SimInjectRequest::AddNode => ControlActionV1::AddNode { at_step },
        SimInjectRequest::ModeChange { mode } => ControlActionV1::ModeChange { at_step, mode },
        SimInjectRequest::Drop { from, to } => ControlActionV1::DropNext { at_step, from, to },
        SimInjectRequest::Delay { from, to, millis } => ControlActionV1::DelayNext {
            at_step,
            from,
            to,
            millis,
        },
        SimInjectRequest::PushEvent {
            client,
            ns,
            key,
            value,
        } => ControlActionV1::PushEvent {
            at_step,
            client,
            ns,
            key,
            value,
        },
        SimInjectRequest::Subscribe { client, ns } => ControlActionV1::Subscribe {
            at_step,
            client,
            ns,
        },
        SimInjectRequest::AddClient { client, ns } => ControlActionV1::AddClient {
            at_step,
            client,
            ns,
        },
        SimInjectRequest::Unsubscribe { client, ns } => ControlActionV1::Unsubscribe {
            at_step,
            client,
            ns,
        },
        SimInjectRequest::RemoveClient { client } => {
            ControlActionV1::RemoveClient { at_step, client }
        }
    };
    world
        .apply_control_action(action)
        .map_err(SimRouteRejection::from)
}

fn assert_current_snapshot_schema(snapshot: &SimSnapshot) {
    debug_assert_eq!(snapshot.schema_version, SIM_SNAPSHOT_SCHEMA_VERSION);
}

#[derive(Debug)]
struct SimRouteRejection {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl SimRouteRejection {
    fn bad_request(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code,
            message: message.into(),
        }
    }
}

impl From<ScenarioError> for SimRouteRejection {
    fn from(error: ScenarioError) -> Self {
        Self::bad_request("unknown_sim_scenario", error.to_string())
    }
}

impl From<ControlApplyError> for SimRouteRejection {
    fn from(error: ControlApplyError) -> Self {
        Self::bad_request("sim_control_rejected", error.to_string())
    }
}

impl IntoResponse for SimRouteRejection {
    fn into_response(self) -> Response {
        (
            self.status,
            cors_headers(),
            Json(SimRouteError {
                code: self.code,
                message: self.message,
            }),
        )
            .into_response()
    }
}

fn cors_headers() -> [(&'static str, &'static str); 3] {
    [
        ("access-control-allow-origin", "*"),
        ("access-control-allow-methods", "GET,POST,OPTIONS"),
        ("access-control-allow-headers", "content-type,authorization"),
    ]
}
