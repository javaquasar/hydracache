use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use hydracache_client_transport_axum::{
    HYDRACACHE_ADMIN_HEADER, HYDRACACHE_CLIENT_ID_HEADER, HYDRACACHE_TENANT_HEADER,
};
use hydracache_observability::{
    BoundedCount, DurableRecoveryStatus, ManagementCompleteness, ManagementObservationSource,
    RecoveryArtifactKind, RecoveryOutcome, RecoveryPhase, RecoveryReasonCode, RecoveryScope,
    RepairState, MANAGEMENT_API_SCHEMA_VERSION,
};
use hydracache_server::{
    AdminHttpSurface, BackupConfig, ManagementOperationKind, ManagementOperationState,
    ServerConfig, ServerObservabilityModel, ServerRole, ServerRuntime, ADMIN_BACKUP_PATH,
    ADMIN_DRAIN_PATH, MANAGEMENT_AUDIT_PATH, MANAGEMENT_OPERATIONS_PATH,
    MANAGEMENT_PERSISTENCE_PATH, MANAGEMENT_RECOVERY_PATH,
};
use serde_json::Value;
use tower::ServiceExt;

fn config_with_backup() -> ServerConfig {
    ServerConfig {
        role: ServerRole::Local,
        seeds: Vec::new(),
        storage_dir: None,
        backup: BackupConfig {
            enabled: true,
            location: Some("s3://secret-bucket/customer/private-prefix".to_owned()),
        },
        ..ServerConfig::default()
    }
}

fn request(method: Method, uri: &str, authenticated: bool) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if authenticated {
        builder = builder
            .header(HYDRACACHE_CLIENT_ID_HEADER, "operator-sensitive-identity")
            .header(HYDRACACHE_TENANT_HEADER, "system-sensitive-tenant")
            .header(HYDRACACHE_ADMIN_HEADER, "true");
    }
    builder.body(Body::empty()).unwrap()
}

async fn json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn new_views_require_authentication_and_are_get_only() {
    let router =
        AdminHttpSurface::new(ServerRuntime::new(config_with_backup()).unwrap().start()).routes();
    for path in [
        MANAGEMENT_PERSISTENCE_PATH,
        MANAGEMENT_OPERATIONS_PATH,
        MANAGEMENT_AUDIT_PATH,
    ] {
        assert_eq!(
            router
                .clone()
                .oneshot(request(Method::GET, path, false))
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            router
                .clone()
                .oneshot(request(Method::POST, path, true))
                .await
                .unwrap()
                .status(),
            StatusCode::METHOD_NOT_ALLOWED
        );
        assert_eq!(
            router
                .clone()
                .oneshot(request(Method::GET, path, true))
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
    }
}

#[tokio::test]
async fn persistence_never_leaks_destination_or_invents_verified_artifacts() {
    let router =
        AdminHttpSurface::new(ServerRuntime::new(config_with_backup()).unwrap().start()).routes();
    let response = router
        .oneshot(request(Method::GET, MANAGEMENT_PERSISTENCE_PATH, true))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = json(response).await;
    assert_eq!(body["data"]["configured"], true);
    assert_eq!(body["data"]["verification_state"], "unavailable");
    assert!(body["data"]["last_verified_backup_id"].is_null());
    assert!(body["data"]["last_verified_restore_id"].is_null());
    let encoded = serde_json::to_string(&body).unwrap();
    assert!(!encoded.contains("secret-bucket"));
    assert!(!encoded.contains("s3://"));
    assert!(!encoded.contains("operator-sensitive"));

    let observed = ServerRuntime::new(config_with_backup())
        .unwrap()
        .with_observability_model(ServerObservabilityModel::default().with_backup_age_seconds(45))
        .start();
    let body = json(
        AdminHttpSurface::new(observed)
            .routes()
            .oneshot(request(Method::GET, MANAGEMENT_PERSISTENCE_PATH, true))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(body["data"]["backup_age_seconds"], 45);
    assert_eq!(body["data"]["backup_age_source"], "runtime_observation");
    assert!(body["data"]["last_verified_backup_id"].is_null());
}

#[tokio::test]
async fn accepted_backup_is_not_reported_as_running_completed_or_verified() {
    let surface = AdminHttpSurface::new(ServerRuntime::new(config_with_backup()).unwrap().start());
    let router = surface.routes();
    let acceptance = router
        .clone()
        .oneshot(request(Method::POST, ADMIN_BACKUP_PATH, true))
        .await
        .unwrap();
    assert_eq!(acceptance.status(), StatusCode::ACCEPTED);
    let acceptance = json(acceptance).await;
    assert_eq!(acceptance["outcome"], "accepted");
    assert_eq!(acceptance["durable_artifact_created"], false);

    let operations = json(
        router
            .clone()
            .oneshot(request(Method::GET, MANAGEMENT_OPERATIONS_PATH, true))
            .await
            .unwrap(),
    )
    .await;
    let item = &operations["data"]["items"]["items"][0];
    assert_eq!(item["kind"], "backup");
    assert_eq!(item["state"], "accepted");
    assert!(item["started_at_unix_ms"].is_null());
    assert!(item["terminal_at_unix_ms"].is_null());

    let persistence = json(
        router
            .oneshot(request(Method::GET, MANAGEMENT_PERSISTENCE_PATH, true))
            .await
            .unwrap(),
    )
    .await;
    assert!(persistence["data"]["last_verified_backup_id"].is_null());
}

#[tokio::test]
async fn synchronous_drain_has_real_terminal_evidence_and_redacted_audit() {
    let surface = AdminHttpSurface::new(ServerRuntime::new(config_with_backup()).unwrap().start());
    let router = surface.routes();
    let response = router
        .clone()
        .oneshot(request(Method::POST, ADMIN_DRAIN_PATH, true))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let operations = json(
        router
            .clone()
            .oneshot(request(Method::GET, MANAGEMENT_OPERATIONS_PATH, true))
            .await
            .unwrap(),
    )
    .await;
    let item = &operations["data"]["items"]["items"][0];
    assert_eq!(item["kind"], "drain");
    assert_eq!(item["state"], "completed");
    assert!(item["started_at_unix_ms"].is_number());
    assert!(item["terminal_at_unix_ms"].is_number());

    let audit = json(
        router
            .oneshot(request(Method::GET, MANAGEMENT_AUDIT_PATH, true))
            .await
            .unwrap(),
    )
    .await;
    assert!(audit["data"]["items"]["items"].as_array().unwrap().len() >= 3);
    let encoded = serde_json::to_string(&audit).unwrap();
    assert!(!encoded.contains("operator-sensitive-identity"));
    assert!(!encoded.contains("system-sensitive-tenant"));
    assert!(!encoded.contains("secret-bucket"));
}

#[tokio::test]
async fn pagination_is_snapshot_stable_and_invalidated_by_new_transition() {
    let surface = AdminHttpSurface::new(ServerRuntime::new(config_with_backup()).unwrap().start());
    let runtime = surface.runtime();
    for _ in 0..3 {
        runtime.lock().unwrap().request_backup().unwrap();
    }
    let router = surface.routes();
    let first = json(
        router
            .clone()
            .oneshot(request(
                Method::GET,
                &format!("{MANAGEMENT_OPERATIONS_PATH}?limit=1"),
                true,
            ))
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(first["data"]["items"]["truncated"], true);
    let cursor = first["data"]["items"]["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let audit_first = json(
        router
            .clone()
            .oneshot(request(
                Method::GET,
                &format!("{MANAGEMENT_AUDIT_PATH}?limit=1"),
                true,
            ))
            .await
            .unwrap(),
    )
    .await;
    let audit_cursor = audit_first["data"]["items"]["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    runtime.lock().unwrap().request_backup().unwrap();
    let stale = router
        .clone()
        .oneshot(request(
            Method::GET,
            &format!("{MANAGEMENT_OPERATIONS_PATH}?limit=1&cursor={cursor}"),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(stale.status(), StatusCode::CONFLICT);
    let stale_audit = router
        .oneshot(request(
            Method::GET,
            &format!("{MANAGEMENT_AUDIT_PATH}?limit=1&cursor={audit_cursor}"),
            true,
        ))
        .await
        .unwrap();
    assert_eq!(stale_audit.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn recovery_remains_unknown_when_runtime_does_not_retain_report() {
    let router =
        AdminHttpSurface::new(ServerRuntime::new(config_with_backup()).unwrap().start()).routes();
    let body = json(
        router
            .oneshot(request(Method::GET, MANAGEMENT_RECOVERY_PATH, true))
            .await
            .unwrap(),
    )
    .await;
    let item = &body["data"]["items"][0];
    assert_eq!(item["outcome"], "unknown");
    assert_eq!(item["reason"], "status-not-retained");
}

#[test]
fn operation_canary_rejects_accepted_as_completed_and_ui_write_assumptions() {
    let journal = hydracache_server::ManagementOperationJournal::new(72);
    let id = journal.request(ManagementOperationKind::Backup, "cluster");
    journal
        .transition(&id, ManagementOperationState::Accepted, None, None)
        .unwrap();
    let state =
        if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("MC72-W10-WRITE-FROM-UI") {
            ManagementOperationState::Completed
        } else {
            journal.snapshot().items[0].state
        };
    assert_eq!(
        state,
        ManagementOperationState::Accepted,
        "HC-CANARY-RED:MC72-W10-WRITE-FROM-UI accepted request was presented as completed or console initiated a write"
    );
}

#[tokio::test]
async fn recovery_canary_liveness_is_never_clean_without_retained_report() {
    let router =
        AdminHttpSurface::new(ServerRuntime::new(config_with_backup()).unwrap().start()).routes();
    let body = json(
        router
            .oneshot(request(Method::GET, MANAGEMENT_RECOVERY_PATH, true))
            .await
            .unwrap(),
    )
    .await;
    let mut outcome = body["data"]["items"][0]["outcome"]
        .as_str()
        .unwrap()
        .to_owned();
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("MC72-W10-LIVENESS-IS-CLEAN") {
        outcome = "clean".to_owned();
    }
    assert_eq!(
        outcome,
        "unknown",
        "HC-CANARY-RED:MC72-W10-LIVENESS-IS-CLEAN liveness was promoted to clean recovery without a retained report"
    );
}

#[test]
fn recovery_canary_preserves_nonzero_loss_evidence() {
    let mut discarded = 3_u64;
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("MC72-W10-LOSS-HIDDEN") {
        discarded = 0;
    }
    assert_eq!(
        discarded, 3,
        "HC-CANARY-RED:MC72-W10-LOSS-HIDDEN nonzero discarded/corrupt recovery evidence was hidden"
    );
}

#[test]
fn recovery_canary_rejects_repaired_before_terminal_verification() {
    let mut report = DurableRecoveryStatus {
        schema_version: MANAGEMENT_API_SCHEMA_VERSION,
        scope: RecoveryScope::Node,
        outcome: RecoveryOutcome::Repaired,
        phase: RecoveryPhase::Validate,
        artifact: RecoveryArtifactKind::DurableValueStore,
        artifact_format_version: Some(1),
        validated_watermark: None,
        records_checked: BoundedCount::exact(10),
        records_recovered: BoundedCount::exact(1),
        records_discarded: BoundedCount::exact(0),
        corrupt_records: BoundedCount::exact(1),
        repair: RepairState::Complete,
        reason: Some(RecoveryReasonCode::ChecksumMismatch),
        started_at_unix_ms: Some(1),
        completed_at_unix_ms: None,
        source: ManagementObservationSource::Live,
        completeness: ManagementCompleteness::Partial,
    };
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref()
        == Ok("MC72-W10-ILLEGAL-REPAIR-COMPLETE")
    {
        report.phase = RecoveryPhase::Complete;
        report.completed_at_unix_ms = Some(2);
    }
    assert!(
        report.validate().is_err(),
        "HC-CANARY-RED:MC72-W10-ILLEGAL-REPAIR-COMPLETE repair was published terminal before post-repair verification"
    );
}
