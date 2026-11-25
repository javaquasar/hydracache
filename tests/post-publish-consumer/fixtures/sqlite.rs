use diesel::{Connection, QueryableByName, RunQueryDsl};
use hydracache_sqlx::sqlx;

#[derive(QueryableByName)]
struct Version {
    #[diesel(sql_type = diesel::sql_types::Text)]
    value: String,
}

#[tokio::test]
async fn sqlite_versions_match() {
    let mut diesel = diesel::SqliteConnection::establish(":memory:").unwrap();
    let diesel = diesel::sql_query("SELECT sqlite_version() AS value")
        .get_result::<Version>(&mut diesel)
        .unwrap()
        .value;
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    let sqlx: String = sqlx::query_scalar("SELECT sqlite_version()")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(diesel, sqlx);
    assert_eq!(sqlx, "3.54.1");
}
