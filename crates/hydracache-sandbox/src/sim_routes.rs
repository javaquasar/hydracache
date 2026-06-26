use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::options;
use axum::{Json, Router};
use hydracache_sim::{
    run_scenario, ControlActionV1, ControlApplyError, ReplayScriptV1, ScenarioError, SimConfig,
    SimMode, SimSnapshot, SimWorld, SIM_SNAPSHOT_SCHEMA_VERSION,
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
}

async fn sim_new(
    State(state): State<SandboxState>,
    Json(request): Json<SimNewRequest>,
) -> SimRouteResult {
    let mut world = if let Some(scenario) = request.scenario.as_deref() {
        if scenario == "default" {
            SimWorld::with_raft_election(request.seed.unwrap_or(50), SimConfig::default())
        } else {
            run_scenario(scenario)
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
    script
        .validate()
        .map_err(ControlApplyError::from)
        .map_err(SimRouteRejection::from)?;
    let mut world = SimWorld::with_raft_election(script.seed, SimConfig::default());
    if let Some(scenario) = script.scenario.as_deref() {
        if scenario != "default" {
            world = run_scenario(scenario)
                .map_err(SimRouteRejection::from)?
                .world;
        }
    }
    world.apply_replay_script(&script)?;
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

async fn sim_options() -> impl IntoResponse {
    (StatusCode::NO_CONTENT, cors_headers())
}

fn apply_injection(
    world: &mut SimWorld,
    request: SimInjectRequest,
) -> Result<(), SimRouteRejection> {
    let applied = match request {
        SimInjectRequest::Workload { enabled } => {
            world.set_workload_enabled(enabled);
            true
        }
        SimInjectRequest::Crash { node } => world.crash_node(node),
        SimInjectRequest::Restart { node } => world.restart_node(node),
        SimInjectRequest::Partition { from, to } => world.partition_link(from, to),
        SimInjectRequest::Heal { from, to } => world.heal_link(from, to),
        SimInjectRequest::Isolate { node } => {
            return world
                .apply_control_action(ControlActionV1::Isolate {
                    at_step: world.outcome().steps,
                    node,
                })
                .map_err(SimRouteRejection::from);
        }
        SimInjectRequest::Rejoin { node } => {
            return world
                .apply_control_action(ControlActionV1::Rejoin {
                    at_step: world.outcome().steps,
                    node,
                })
                .map_err(SimRouteRejection::from);
        }
        SimInjectRequest::Disable { node } => {
            return world
                .apply_control_action(ControlActionV1::Disable {
                    at_step: world.outcome().steps,
                    node,
                })
                .map_err(SimRouteRejection::from);
        }
        SimInjectRequest::Enable { node } => {
            return world
                .apply_control_action(ControlActionV1::Enable {
                    at_step: world.outcome().steps,
                    node,
                })
                .map_err(SimRouteRejection::from);
        }
        SimInjectRequest::AddNode => {
            return world
                .apply_control_action(ControlActionV1::AddNode {
                    at_step: world.outcome().steps,
                })
                .map_err(SimRouteRejection::from);
        }
        SimInjectRequest::ModeChange { mode } => {
            return world
                .apply_control_action(ControlActionV1::ModeChange {
                    at_step: world.outcome().steps,
                    mode,
                })
                .map_err(SimRouteRejection::from);
        }
        SimInjectRequest::Drop { from, to } => world.drop_next_on_link(from, to),
        SimInjectRequest::Delay { from, to, millis } => {
            world.delay_next_on_link_millis(from, to, millis)
        }
        SimInjectRequest::PushEvent {
            client,
            ns,
            key,
            value,
        } => {
            return world
                .apply_control_action(ControlActionV1::PushEvent {
                    at_step: world.outcome().steps,
                    client,
                    ns,
                    key,
                    value,
                })
                .map_err(SimRouteRejection::from);
        }
        SimInjectRequest::Subscribe { client, ns } => {
            return world
                .apply_control_action(ControlActionV1::Subscribe {
                    at_step: world.outcome().steps,
                    client,
                    ns,
                })
                .map_err(SimRouteRejection::from);
        }
    };
    if applied {
        Ok(())
    } else {
        Err(SimRouteRejection::bad_request(
            "invalid_sim_action",
            "sim injection references an unknown node or link",
        ))
    }
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
