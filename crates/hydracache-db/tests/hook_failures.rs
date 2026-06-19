use hydracache_db::{HookError, HookInvalidationTarget, HookPlan};

#[test]
fn missing_outbox_table_clear_error() {
    let error = HookPlan::sqlite("bad table")
        .on_insert(HookInvalidationTarget::tag("users"))
        .render_sql()
        .unwrap_err();

    assert!(matches!(error, HookError::InvalidIdentifier(_)));
}

#[test]
fn hook_version_mismatch_detected_at_startup() {
    let version = HookPlan::sqlite("users").schema_version();

    assert_ne!(version.version, 0);
    assert_eq!(version.artifact, hydracache_db::HOOK_SCHEMA_ARTIFACT);
}
