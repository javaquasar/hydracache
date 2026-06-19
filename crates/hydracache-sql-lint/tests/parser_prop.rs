use hydracache_sql_lint::{DependencyLint, DependencyLintMode, LintStatus, Relation, SqlDialect};
use proptest::prelude::*;

proptest! {
    #[test]
    fn declared_superset_is_never_missing(extra in "[a-z]{1,12}") {
        let lint = DependencyLint::new(SqlDialect::Postgres, DependencyLintMode::Warn);
        let declared = vec![Relation::table("users"), Relation::table(extra)];
        let status = lint.check("select * from users", &declared);

        prop_assert!(!matches!(status, LintStatus::MissingDependencies(_)));
    }
}
