use hydracache_sql_lint::{DependencyLint, DependencyLintMode, LintStatus, Relation, SqlDialect};

#[test]
fn warn_mode_reports_missing() {
    let lint = DependencyLint::new(SqlDialect::Postgres, DependencyLintMode::Warn);
    let status = lint.check(
        "select * from users join roles on roles.id = users.role_id",
        &[Relation::table("users")],
    );

    assert_eq!(
        status,
        LintStatus::MissingDependencies(vec![Relation::table("roles")])
    );
}

#[test]
fn deny_mode_fails_on_missing() {
    let lint = DependencyLint::new(
        SqlDialect::Postgres,
        DependencyLintMode::DenyMissingDependencies,
    );
    let status = lint.check(
        "select * from users join roles on true",
        &[Relation::table("users")],
    );

    assert!(!status.is_clean());
}

#[test]
fn dynamic_sql_is_inconclusive() {
    let lint = DependencyLint::new(SqlDialect::Postgres, DependencyLintMode::Warn);
    let status = lint.check(
        "select * from ", /* intentionally incomplete dynamic SQL */
        &[],
    );

    assert!(matches!(status, LintStatus::Inconclusive(_)));
}

#[test]
fn unsupported_syntax_does_not_panic() {
    let lint = DependencyLint::new(SqlDialect::Postgres, DependencyLintMode::Warn);
    let status = lint.check("select * from users where", &[Relation::table("users")]);

    assert!(matches!(status, LintStatus::Inconclusive(_)));
}
