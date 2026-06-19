use hydracache_sql_lint::{
    Baseline, DependencyLint, DependencyLintMode, LintFindingKind, LintSuppression, Relation,
    SqlDialect,
};

#[test]
fn suppressed_finding_does_not_fail_deny() {
    let lint = DependencyLint::new(
        SqlDialect::Postgres,
        DependencyLintMode::DenyMissingDependencies,
    );
    let diagnostics = lint.diagnostics(
        "load-roles",
        "select * from users join roles on true",
        &[Relation::table("users")],
        &[LintSuppression::new(
            LintFindingKind::MissingDependencies,
            "roles checked manually",
        )
        .unwrap()],
    );

    assert!(diagnostics.is_empty());
}

#[test]
fn baseline_finding_does_not_fail_but_new_one_does() {
    let lint = DependencyLint::new(
        SqlDialect::Postgres,
        DependencyLintMode::DenyMissingDependencies,
    );
    let first = lint.diagnostics(
        "load-roles",
        "select * from users join roles on true",
        &[Relation::table("users")],
        &[],
    );
    let baseline = Baseline::from_diagnostics(first.iter());
    let mut current = first.clone();
    current.extend(lint.diagnostics(
        "load-permissions",
        "select * from users join permissions on true",
        &[Relation::table("users")],
        &[],
    ));

    let diff = baseline.diff(current);

    assert_eq!(diff.accepted_findings.len(), 1);
    assert_eq!(diff.new_findings.len(), 1);
}

#[test]
fn stale_baseline_entry_is_reported() {
    let lint = DependencyLint::new(SqlDialect::Postgres, DependencyLintMode::Warn);
    let old = lint.diagnostics(
        "load-roles",
        "select * from users join roles on true",
        &[Relation::table("users")],
        &[],
    );
    let baseline = Baseline::from_diagnostics(old.iter());
    let diff = baseline.diff(Vec::new());

    assert_eq!(diff.stale_entries.len(), 1);
}
