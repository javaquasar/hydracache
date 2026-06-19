use hydracache_sql_lint::{DependencyLint, DependencyLintMode, Relation, SqlDialect};

fn lint(dialect: SqlDialect) -> DependencyLint {
    DependencyLint::new(dialect, DependencyLintMode::Warn)
}

#[test]
fn observes_single_table() {
    let relations = lint(SqlDialect::Postgres)
        .observed_relations("select * from users")
        .unwrap();

    assert_eq!(relations, vec![Relation::table("users")]);
}

#[test]
fn observes_three_way_join() {
    let relations = lint(SqlDialect::Postgres)
        .observed_relations(
            "select u.id, r.name from users u
             join user_roles ur on ur.user_id = u.id
             join roles r on r.id = ur.role_id",
        )
        .unwrap();

    assert_eq!(
        relations,
        vec![
            Relation::table("roles"),
            Relation::table("user_roles"),
            Relation::table("users"),
        ]
    );
}

#[test]
fn resolves_aliases() {
    let relations = lint(SqlDialect::Postgres)
        .observed_relations("select u.id from users u where u.id = $1")
        .unwrap();

    assert_eq!(relations, vec![Relation::table("users")]);
}

#[test]
fn schema_qualified() {
    let relations = lint(SqlDialect::Postgres)
        .observed_relations("select * from app.users")
        .unwrap();

    assert_eq!(relations, vec![Relation::schema_table("app", "users")]);
}

#[test]
fn cte_and_subquery() {
    let relations = lint(SqlDialect::Postgres)
        .observed_relations(
            "with active_users as (select * from users)
             select * from active_users where exists (select 1 from roles)",
        )
        .unwrap();

    assert_eq!(
        relations,
        vec![Relation::table("roles"), Relation::table("users")]
    );
}

#[test]
fn ignores_string_literals_and_comments() {
    let relations = lint(SqlDialect::Postgres)
        .observed_relations("select 'users' as label from roles -- users")
        .unwrap();

    assert_eq!(relations, vec![Relation::table("roles")]);
}

#[test]
fn dialect_placeholders() {
    lint(SqlDialect::Postgres)
        .observed_relations("select * from users where id = $1")
        .unwrap();
    lint(SqlDialect::MySql)
        .observed_relations("select * from users where id = ?")
        .unwrap();
    lint(SqlDialect::Sqlite)
        .observed_relations("select * from users where id = ?")
        .unwrap();
}
