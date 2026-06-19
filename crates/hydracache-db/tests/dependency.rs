use hydracache_db::{query_cache_policy, table, DeclaredLintMode, LintFinding, QueryCachePolicy};

#[test]
fn policy_records_sql_dependency_metadata() {
    let policy = QueryCachePolicy::named("load-user-permissions")
        .key("tenant:7:user:42")
        .lint_sql("select * from users join user_roles on true")
        .declared_dependency(table("users"))
        .declared_dependency(table("user_roles"))
        .dependency_lint_mode(DeclaredLintMode::DenyMissingDependencies)
        .lint_allow(LintFinding::Inconclusive, "dynamic predicate audited");

    let metadata = policy.lint_metadata().expect("metadata should be attached");

    assert_eq!(
        metadata.sql(),
        Some("select * from users join user_roles on true")
    );
    assert_eq!(metadata.declared().len(), 2);
    assert_eq!(metadata.mode(), DeclaredLintMode::DenyMissingDependencies);
    assert_eq!(
        metadata.suppressions()[0].reason(),
        "dynamic predicate audited"
    );
}

#[test]
fn macro_records_sql_dependency_metadata() {
    let tenant_id = 7_u64;
    let user_id = 42_u64;
    let policy = query_cache_policy!(
        name = "load-user-permissions",
        key_segments = ["tenant", tenant_id, "user", user_id],
        sql = "select * from users join user_roles on true",
        depends_on = [table("users"), table("user_roles")],
        dependency_lint = deny_missing_dependencies,
        lint_allow = [(LintFinding::Inconclusive, "dynamic predicate audited")],
        ttl_secs = 30,
    );

    let metadata = policy.lint_metadata().expect("metadata should be attached");

    assert_eq!(metadata.declared().len(), 2);
    assert_eq!(metadata.mode(), DeclaredLintMode::DenyMissingDependencies);
    assert_eq!(
        metadata.suppressions()[0].finding(),
        LintFinding::Inconclusive
    );
}
