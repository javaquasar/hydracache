use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::Router;
use hydracache_sandbox::{build_sandbox, SandboxConfig};
use hydracache_sim::{ControlActionV1, ReplayScriptV1, SimMode, SIM_SNAPSHOT_SCHEMA_VERSION};
use serde_json::{json, Value};
use tower::ServiceExt;

#[tokio::test]
async fn sim_routes_emit_w2_schema_and_step_deterministically() {
    let app = build_sandbox(SandboxConfig::default())
        .await
        .unwrap()
        .router;

    let created = post_json(
        app.clone(),
        "/sim/new",
        json!({"seed": 700, "steps": 3}).to_string(),
    )
    .await;
    assert_eq!(created["schema_version"], SIM_SNAPSHOT_SCHEMA_VERSION);
    assert_eq!(created["seed"], 700);
    assert_eq!(created["step"], 3);
    assert_eq!(created["verdict"]["status"], "holding");

    let preflight = app.clone().oneshot(options("/sim/step")).await.unwrap();
    assert_eq!(preflight.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        preflight
            .headers()
            .get("access-control-allow-origin")
            .unwrap(),
        "*"
    );

    let stepped = post_json(app.clone(), "/sim/step", json!({"steps": 2}).to_string()).await;
    assert_eq!(stepped["schema_version"], created["schema_version"]);
    assert_eq!(stepped["seed"], 700);
    assert_eq!(stepped["step"], 5);

    let partitioned = post_json(
        app.clone(),
        "/sim/inject",
        json!({"action": "partition", "from": "node-0", "to": "node-1"}).to_string(),
    )
    .await;
    assert!(
        partitioned["links"]
            .as_array()
            .unwrap()
            .iter()
            .any(|link| link["from"] == "node-0"
                && link["to"] == "node-1"
                && link["state"] == "partitioned"),
        "partition injection should update the real simulator link state"
    );

    let snapshot = get_json(app.clone(), "/sim/snapshot").await;
    assert_eq!(snapshot, partitioned);

    let scenario = post_json(
        app.clone(),
        "/sim/new",
        json!({"scenario": "leader_crash_failover_no_committed_loss"}).to_string(),
    )
    .await;
    assert_eq!(scenario["seed"], 5002);
    assert_eq!(scenario["step"], 12);
    assert_eq!(scenario["schema_version"], SIM_SNAPSHOT_SCHEMA_VERSION);

    let response = app
        .oneshot(post(
            "/sim/new",
            json!({"scenario": "not-a-scenario"}).to_string(),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response
            .headers()
            .get("access-control-allow-origin")
            .unwrap(),
        "*"
    );
    let body = body_json(response).await;
    assert_eq!(body["code"], "unknown_sim_scenario");
}

#[tokio::test]
async fn sim_routes_accept_same_control_script() {
    let app = build_sandbox(SandboxConfig::default())
        .await
        .unwrap()
        .router;
    let script = ReplayScriptV1::new(
        0x5334,
        SimMode::Manual,
        vec![
            ControlActionV1::Step { at_step: 0, n: 8 },
            ControlActionV1::Subscribe {
                at_step: 8,
                client: "client-a".to_owned(),
                ns: "profiles".to_owned(),
            },
            ControlActionV1::PushEvent {
                at_step: 8,
                client: "client-a".to_owned(),
                ns: "profiles".to_owned(),
                key: "profile-42".to_owned(),
                value: "fresh".to_owned(),
            },
            ControlActionV1::Step { at_step: 8, n: 2 },
        ],
    );

    let snapshot = post_json(app, "/sim/control", script.to_json()).await;

    assert_eq!(snapshot["schema_version"], SIM_SNAPSHOT_SCHEMA_VERSION);
    assert!(snapshot["subscribers"]
        .as_array()
        .unwrap()
        .iter()
        .any(|subscriber| subscriber["last_event"]["kind"] == "upserted"));
}

#[tokio::test]
async fn sim_routes_accept_topology_control_actions() {
    let app = build_sandbox(SandboxConfig::default())
        .await
        .unwrap()
        .router;
    post_json(
        app.clone(),
        "/sim/new",
        json!({"seed": 701, "steps": 8}).to_string(),
    )
    .await;

    let isolated = post_json(
        app.clone(),
        "/sim/inject",
        json!({"action": "isolate", "node": "node-0"}).to_string(),
    )
    .await;
    assert!(isolated["links"]
        .as_array()
        .unwrap()
        .iter()
        .any(|link| link["from"] == "node-0" && link["state"] == "partitioned"));

    let rejoined = post_json(
        app.clone(),
        "/sim/inject",
        json!({"action": "rejoin", "node": "node-0"}).to_string(),
    )
    .await;
    assert!(rejoined["rebalance"].is_object());

    let added = post_json(
        app,
        "/sim/inject",
        json!({"action": "add_node"}).to_string(),
    )
    .await;
    assert_eq!(added["schema_version"], SIM_SNAPSHOT_SCHEMA_VERSION);
    assert!(added["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|node| node["id"] == "node-3"));
}

async fn post_json(app: Router, uri: &str, body: String) -> Value {
    let response = app.oneshot(post(uri, body)).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    body_json(response).await
}

async fn get_json(app: Router, uri: &str) -> Value {
    let response = app.oneshot(get(uri)).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    body_json(response).await
}

fn post(uri: &str, body: String) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap()
}

fn get(uri: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

fn options(uri: &str) -> Request<Body> {
    Request::builder()
        .method("OPTIONS")
        .uri(uri)
        .header("access-control-request-method", "POST")
        .header("access-control-request-headers", "content-type")
        .body(Body::empty())
        .unwrap()
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}
