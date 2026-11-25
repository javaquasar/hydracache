use hydracache_sql_lint::{
    Baseline, DependencyLint, DependencyLintMode, LintFindingKind, LintStatus, LintSuppression,
    Relation, SqlDialect,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ANCHOR: check
    let lint = DependencyLint::new(
        SqlDialect::Postgres,
        DependencyLintMode::DenyMissingDependencies,
    );

    // The cached query reads `users` and `roles`...
    let sql = "SELECT u.id, u.name, r.name AS role \
               FROM users u JOIN roles r ON r.id = u.role_id \
               WHERE u.id = $1";

    // ...but its cache policy only declares (and is invalidated by) `users`.
    let declared = [Relation::table("users")];

    assert_eq!(
        lint.check(sql, &declared),
        LintStatus::MissingDependencies(vec![Relation::table("roles")])
    );

    // Declaring both relations makes the policy honest.
    let declared = [Relation::table("users"), Relation::table("roles")];
    assert!(lint.check(sql, &declared).is_clean());
    // ANCHOR_END: check

    // ANCHOR: inconclusive
    // SQL that cannot be parsed is reported, never silently accepted.
    let status = lint.check("SELECT * FROM ", &[]);
    assert!(matches!(status, LintStatus::Inconclusive(_)));
    // ANCHOR_END: inconclusive

    // ANCHOR: suppress-and-baseline
    // A deliberate gap needs a written reason.
    let suppressed = lint.diagnostics(
        "load-user-with-role",
        "SELECT * FROM users JOIN roles ON true",
        &[Relation::table("users")],
        &[LintSuppression::new(
            LintFindingKind::MissingDependencies,
            "roles are immutable reference data",
        )?],
    );
    assert!(suppressed.is_empty());

    // Existing findings can be accepted into a baseline; only new ones fail.
    let existing = lint.diagnostics(
        "legacy-report",
        "SELECT * FROM orders JOIN customers ON true",
        &[Relation::table("orders")],
        &[],
    );
    let baseline = Baseline::from_diagnostics(existing.iter());
    let diff = baseline.diff(existing);
    assert!(diff.new_findings.is_empty());
    // ANCHOR_END: suppress-and-baseline

    println!("sql lint ok");
    Ok(())
}
