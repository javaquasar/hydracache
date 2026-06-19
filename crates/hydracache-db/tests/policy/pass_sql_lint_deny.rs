use hydracache_db::{query_cache_policy, table, DeclaredLintMode, LintFinding};

fn main() {
    let policy = query_cache_policy!(
        name = "load-user",
        key_segments = ["tenant", 7_u64, "user", 42_u64],
        sql = "select * from users where id = $1",
        depends_on = [table("users")],
        dependency_lint = deny_missing_dependencies,
        lint_allow = [(LintFinding::Inconclusive, "dynamic clause audited")],
        ttl_secs = 30,
    );

    assert_eq!(
        policy.lint_metadata().unwrap().mode(),
        DeclaredLintMode::DenyMissingDependencies
    );
}
