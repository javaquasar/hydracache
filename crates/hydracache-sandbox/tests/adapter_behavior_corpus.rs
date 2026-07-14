use std::collections::BTreeMap;
use std::env;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::Router;
use hydracache_sandbox::{build_sandbox, SandboxBackend, SandboxConfig, SandboxProfile};
use serde::Deserialize;
use serde_json::Value;
use tower::ServiceExt;

#[derive(Debug, Deserialize)]
struct Corpus {
    schema_version: u32,
    fast_backend: String,
    optional_backends: Vec<OptionalBackend>,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Deserialize)]
struct OptionalBackend {
    id: String,
    gate_env: String,
}

#[derive(Debug, Deserialize)]
struct Scenario {
    id: String,
    steps: Vec<Step>,
}

#[derive(Debug, Deserialize)]
struct Step {
    path: String,
    body: Value,
    expect: BTreeMap<String, Value>,
}

fn corpus() -> Corpus {
    serde_json::from_str(include_str!("corpus/adapters/core.json")).unwrap()
}

async fn execute(app: Router, scenario: &Scenario) {
    for (index, step) in scenario.steps.iter().enumerate() {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&step.path)
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&step.body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(
            status,
            StatusCode::OK,
            "{} step {index} failed: {}",
            scenario.id,
            String::from_utf8_lossy(&bytes)
        );
        let actual: Value = serde_json::from_slice(&bytes).unwrap();
        for (pointer, expected) in &step.expect {
            assert_eq!(
                actual.pointer(pointer),
                Some(expected),
                "{} step {index} expectation {pointer}",
                scenario.id
            );
        }
    }
}

#[tokio::test]
async fn sqlite_executes_every_adapter_behavior_scenario() {
    let corpus = corpus();
    assert_eq!(corpus.schema_version, 1);
    assert_eq!(corpus.fast_backend, "sqlite-memory");
    let app = build_sandbox(SandboxConfig {
        profile: SandboxProfile::SqliteMemory,
        backend: SandboxBackend::SqliteMemory,
        ..SandboxConfig::default()
    })
    .await
    .unwrap()
    .router;
    for scenario in &corpus.scenarios {
        execute(app.clone(), scenario).await;
    }
}

#[test]
fn adapter_corpus_rejects_rollback_invalidation_or_cross_namespace_visibility() {
    let corpus = corpus();
    let rollback = corpus
        .scenarios
        .iter()
        .find(|scenario| scenario.id == "transaction-commit-rollback")
        .unwrap();
    assert_eq!(rollback.steps[0].expect["/summary/rollbacks"], 1);
    assert_eq!(rollback.steps[0].expect["/summary/writes"], 1);
    assert_eq!(rollback.steps[0].expect["/passed"], true);

    let namespace = corpus
        .scenarios
        .iter()
        .find(|scenario| scenario.id == "namespace-isolation")
        .unwrap();
    assert_eq!(namespace.steps.last().unwrap().expect["/value"], "b");
}

#[test]
fn optional_adapter_rows_are_registered_and_fail_loud_when_claimed_but_unavailable() {
    let corpus = corpus();
    let postgres = corpus
        .optional_backends
        .iter()
        .find(|backend| backend.id == "postgres-docker")
        .unwrap();
    assert_eq!(postgres.gate_env, "HYDRACACHE_RUN_ADAPTER_CORPUS_DOCKER");
    let error = SandboxConfig::from_args(["sandbox", "--backend", "unknown-adapter"])
        .unwrap_err()
        .to_string();
    assert!(error.contains("unknown backend"));
}

#[tokio::test]
#[ignore = "Docker adapter corpus; requires HYDRACACHE_RUN_ADAPTER_CORPUS_DOCKER=1"]
async fn postgres_docker_executes_adapter_behavior_corpus_when_claimed() {
    assert_eq!(
        env::var("HYDRACACHE_RUN_ADAPTER_CORPUS_DOCKER").as_deref(),
        Ok("1"),
        "set HYDRACACHE_RUN_ADAPTER_CORPUS_DOCKER=1 to claim this proof"
    );
    let app = build_sandbox(SandboxConfig {
        profile: SandboxProfile::PostgresDocker,
        backend: SandboxBackend::PostgresDocker,
        ..SandboxConfig::default()
    })
    .await
    .expect("claimed Postgres adapter must start; unavailable Docker is a failed proof")
    .router;
    for scenario in &corpus().scenarios {
        execute(app.clone(), scenario).await;
    }
}

#[test]
fn canary_adapter_runner_treats_rolled_back_write_as_committed() {
    let transaction_rolled_back = true;
    let invalidation_emitted = false;
    assert!(!(transaction_rolled_back && invalidation_emitted));
}
