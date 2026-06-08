use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::Router;
use hydracache_sandbox::{
    build_sandbox, SandboxBackend, SandboxConfig, SandboxError, SandboxProfile,
};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn postgres_docker_profile_smoke_test_skips_when_docker_is_unavailable(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let sandbox = match build_sandbox(SandboxConfig {
        profile: SandboxProfile::PostgresDocker,
        backend: SandboxBackend::PostgresDocker,
        ..SandboxConfig::default()
    })
    .await
    {
        Ok(app) => app,
        Err(SandboxError::Docker(error)) => {
            eprintln!(
                "skipping hydracache-sandbox Postgres smoke test because Docker is unavailable: {error}"
            );
            return Ok(());
        }
        Err(error) => return Err(Box::new(error) as Box<dyn std::error::Error + Send + Sync>),
    };
    let app = sandbox.router.clone();

    let first = post_json(app.clone(), "/demo/load/42", Body::empty()).await?;
    assert_eq!(first["user"]["name"], "Ada");
    assert_eq!(first["source"], "loader");

    let cached = post_json(app.clone(), "/demo/load/42", Body::empty()).await?;
    assert_eq!(cached["source"], "cache");

    let updated = post_json(
        app.clone(),
        "/demo/users/42",
        Body::from(r#"{"name":"Grace"}"#),
    )
    .await?;
    assert_eq!(updated["name"], "Grace");

    let still_cached = post_json(app.clone(), "/demo/load/42", Body::empty()).await?;
    assert_eq!(still_cached["user"]["name"], "Ada");

    let invalidated = post_json(app.clone(), "/demo/invalidate/user/42", Body::empty()).await?;
    assert_eq!(invalidated["removed"], 1);

    let reloaded = post_json(app, "/demo/load/42", Body::empty()).await?;
    assert_eq!(reloaded["user"]["name"], "Grace");
    assert_eq!(reloaded["source"], "loader");

    Ok(())
}

async fn post_json(
    app: Router,
    uri: &str,
    body: Body,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let response = app
        .oneshot(post(uri, body))
        .await
        .expect("sandbox router should be infallible");
    json_body(response).await
}

fn post(uri: &str, body: Body) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(body)
        .unwrap()
}

async fn json_body(
    response: axum::response::Response,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "unexpected response body: {}",
        String::from_utf8_lossy(&bytes)
    );
    Ok(serde_json::from_slice(&bytes)?)
}
