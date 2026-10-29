use axum::body::{to_bytes, Body};
use axum::http::{header, Method, Request, StatusCode};
use hydracache_sandbox::{build_sandbox, SandboxConfig};
use serde_json::Value;
use tower::ServiceExt;

const BODY_LIMIT: usize = 16 * 1024 * 1024;

fn request(method: Method, uri: &str, token: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    builder.body(Body::empty()).unwrap()
}

#[tokio::test]
async fn swagger_document_matches_the_existing_typed_openapi_bytes() {
    let app = build_sandbox(SandboxConfig::default())
        .await
        .unwrap()
        .router;
    let mut documents = Vec::new();
    for uri in ["/openapi.json", "/swagger-ui/openapi.json"] {
        let response = app
            .clone()
            .oneshot(request(Method::GET, uri, None))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{uri}");
        assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
        documents.push(to_bytes(response.into_body(), BODY_LIMIT).await.unwrap());
    }
    assert_eq!(documents[0], documents[1]);
    let document: Value = serde_json::from_slice(&documents[0]).unwrap();
    assert_eq!(document["openapi"], "3.1.0");
    assert!(document["paths"]["/demo/cache/get"].is_object());
    assert!(document["components"]["schemas"].is_object());
    // A retained focused --nocapture command compares the entire document before
    // and after dependency migration, not a version-only schema assertion.
    println!(
        "HC074_SWAGGER_OPENAPI {}",
        std::str::from_utf8(&documents[0]).unwrap()
    );
}

#[tokio::test]
async fn swagger_assets_are_local_and_point_to_the_original_document() {
    let app = build_sandbox(SandboxConfig::default())
        .await
        .unwrap()
        .router;
    let redirect = app
        .clone()
        .oneshot(request(Method::GET, "/swagger-ui", None))
        .await
        .unwrap();
    assert_eq!(redirect.status(), StatusCode::SEE_OTHER);
    assert_eq!(redirect.headers()[header::LOCATION], "/swagger-ui/");

    for (path, content_type, marker) in [
        ("", "text/html", "swagger-ui"),
        ("swagger-ui.css", "text/css", ".swagger-ui"),
        ("swagger-ui-bundle.js", "javascript", "SwaggerUIBundle"),
        (
            "swagger-ui-standalone-preset.js",
            "javascript",
            "SwaggerUIStandalonePreset",
        ),
        (
            "swagger-initializer.js",
            "javascript",
            "/swagger-ui/openapi.json",
        ),
    ] {
        let uri = format!("/swagger-ui/{path}");
        let response = app
            .clone()
            .oneshot(request(Method::GET, &uri, None))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{uri}");
        assert!(
            response.headers()[header::CONTENT_TYPE]
                .to_str()
                .unwrap()
                .contains(content_type),
            "{uri}"
        );
        assert!(
            response.headers().get(header::CONTENT_ENCODING).is_none(),
            "unrequested encoding: {uri}"
        );
        let body = to_bytes(response.into_body(), BODY_LIMIT).await.unwrap();
        assert!(
            std::str::from_utf8(&body).unwrap().contains(marker),
            "missing marker: {uri}"
        );
        if path.is_empty() {
            let html = std::str::from_utf8(&body).unwrap();
            assert!(!html.contains("unpkg.com"));
            assert!(!html.contains("cdn.jsdelivr.net"));
        }
        if path == "swagger-initializer.js" {
            assert!(!std::str::from_utf8(&body)
                .unwrap()
                .contains("petstore.swagger.io"));
        }
    }
}

#[tokio::test]
async fn swagger_preserves_the_existing_auth_boundary() {
    let app = build_sandbox(SandboxConfig {
        auth_token: Some("swagger-fixture-token".to_owned()),
        ..SandboxConfig::default()
    })
    .await
    .unwrap()
    .router;
    for uri in ["/demo/report", "/openapi.json"] {
        let denied = app
            .clone()
            .oneshot(request(Method::GET, uri, None))
            .await
            .unwrap();
        assert_eq!(denied.status(), StatusCode::UNAUTHORIZED, "{uri}");
        let permitted = app
            .clone()
            .oneshot(request(Method::GET, uri, Some("swagger-fixture-token")))
            .await
            .unwrap();
        assert_eq!(permitted.status(), StatusCode::OK, "{uri}");
    }
    // Documentation was public before this migration; sandbox data stays guarded.
    for uri in [
        "/swagger-ui/",
        "/swagger-ui/openapi.json",
        "/swagger-ui/swagger-initializer.js",
    ] {
        assert_eq!(
            app.clone()
                .oneshot(request(Method::GET, uri, None))
                .await
                .unwrap()
                .status(),
            StatusCode::OK,
            "{uri}"
        );
    }
}

#[tokio::test]
async fn swagger_rejects_unknown_files_traversal_and_unsupported_methods() {
    let app = build_sandbox(SandboxConfig::default())
        .await
        .unwrap()
        .router;
    for uri in [
        "/swagger-ui/no-such-file.js",
        "/swagger-ui/../Cargo.toml",
        "/swagger-ui/%2e%2e/Cargo.toml",
    ] {
        assert_eq!(
            app.clone()
                .oneshot(request(Method::GET, uri, None))
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND,
            "{uri}"
        );
    }
    for uri in ["/swagger-ui/", "/swagger-ui/openapi.json"] {
        assert_eq!(
            app.clone()
                .oneshot(request(Method::POST, uri, None))
                .await
                .unwrap()
                .status(),
            StatusCode::METHOD_NOT_ALLOWED,
            "{uri}"
        );
    }
}
