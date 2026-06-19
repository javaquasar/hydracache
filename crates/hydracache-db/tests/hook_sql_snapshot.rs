use hydracache_db::{HookError, HookInvalidationTarget, HookPlan};

#[test]
fn pg_insert_update_delete_snapshot() {
    let sql = HookPlan::postgres("users")
        .namespace("accounts")
        .on_insert(HookInvalidationTarget::entity("user", "id"))
        .on_update(HookInvalidationTarget::tag("users"))
        .on_delete(HookInvalidationTarget::collection("users"))
        .render_sql()
        .unwrap();

    assert!(sql.contains("create or replace function hydracache_users_insert_outbox_fn()"));
    assert!(sql.contains("after insert on users"));
    assert!(sql.contains("'accounts'"));
    assert!(sql.contains("'entity'"));
    assert!(sql.contains("'collection'"));
    assert!(sql.contains("on conflict (namespace, commit_position, target_hash) do nothing"));
}

#[test]
fn sqlite_snapshot() {
    let sql = HookPlan::sqlite("users")
        .on_insert(HookInvalidationTarget::entity("user", "id"))
        .on_update(HookInvalidationTarget::tag_column("tenant", "tenant_id"))
        .render_sql()
        .unwrap();

    assert!(sql.contains("create trigger if not exists hydracache_users_insert_outbox"));
    assert!(sql.contains("after insert on users"));
    assert!(sql.contains("insert or ignore into hydracache_invalidation_outbox"));
    assert!(sql.contains("cast(NEW.id as text)"));
    assert!(sql.contains("'tenant:' || cast(NEW.tenant_id as text)"));
}

#[test]
fn mysql_snapshot() {
    let sql = HookPlan::mysql("users")
        .on_insert(HookInvalidationTarget::entity("user", "id"))
        .on_delete(HookInvalidationTarget::collection("users"))
        .render_sql()
        .unwrap();

    assert!(sql.contains("create trigger hydracache_users_insert_outbox"));
    assert!(sql.contains("insert ignore into hydracache_invalidation_outbox"));
    assert!(sql.contains("concat('entity:', 'user', ':', cast(NEW.id as char))"));
    assert!(sql.contains("on duplicate key update version = values(version)"));
}

#[test]
fn render_rejects_missing_tag_columns() {
    let error = HookPlan::sqlite("users")
        .on_update(HookInvalidationTarget::tag_column("tenant", ""))
        .render_sql()
        .unwrap_err();

    assert!(matches!(error, HookError::MissingColumn(_)));
}

#[test]
fn render_rejects_invalid_table_identifier() {
    let error = HookPlan::sqlite("users; drop table users")
        .on_update(HookInvalidationTarget::tag("users"))
        .render_sql()
        .unwrap_err();

    assert!(matches!(error, HookError::InvalidIdentifier(_)));
}
