use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::Router;
use hydracache_sandbox::{build_sandbox, SandboxConfig};
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
    assert_eq!(created["schema_version"], 1);
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
    assert_eq!(scenario["schema_version"], 1);

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
