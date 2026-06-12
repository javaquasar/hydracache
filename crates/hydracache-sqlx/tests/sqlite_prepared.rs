use hydracache::HydraCache;
use hydracache_sqlx::{HydraCacheEntity, PreparedQueryPolicy, SqlxCache, SqlxQueryExt};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePoolOptions;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, HydraCacheEntity)]
#[hydracache(entity = "sqlite-user", collection = "sqlite-users", id = i64)]
struct User {
    id: i64,
    name: String,
}

#[tokio::test]
async fn prepared_sqlite_queries_cache_real_in_memory_database_results(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;

    sqlx::query("create table users (id integer primary key, name text not null)")
        .execute(&pool)
        .await?;
    sqlx::query("insert into users (id, name) values (?, ?)")
        .bind(42_i64)
        .bind("Ada")
        .execute(&pool)
        .await?;
    sqlx::query("insert into users (id, name) values (?, ?)")
        .bind(7_i64)
        .bind("Linus")
        .execute(&pool)
        .await?;

    let queries = SqlxCache::new(HydraCache::local().build(), "sqlite");
    let prepared_user = queries.prepare::<(i64, String)>(
        PreparedQueryPolicy::for_cache_entity::<User>().with_name("sqlite-load-user"),
    );

    let first = prepared_user
        .for_id(42)
        .fetch_one(
            pool.clone(),
            sqlx::query_as("select id, name from users where id = ?").bind(42_i64),
        )
        .await?;
    assert_eq!(first, (42, "Ada".to_owned()));

    sqlx::query("update users set name = ? where id = ?")
        .bind("Grace")
        .bind(42_i64)
        .execute(&pool)
        .await?;

    let cached = prepared_user
        .for_id(42)
        .fetch_one(
            pool.clone(),
            sqlx::query_as("select id, name from users where id = ?").bind(42_i64),
        )
        .await?;
    assert_eq!(cached, (42, "Ada".to_owned()));

    assert_eq!(queries.cache().invalidate_tag("sqlite-users").await?, 1);

    let reloaded = prepared_user
        .for_id(42)
        .fetch_one(
            pool.clone(),
            sqlx::query_as("select id, name from users where id = ?").bind(42_i64),
        )
        .await?;
    assert_eq!(reloaded, (42, "Grace".to_owned()));

    let prepared_list = queries.prepare::<(i64, String)>(
        PreparedQueryPolicy::named("sqlite-list-users").collection("sqlite-users:all"),
    );

    let listed = prepared_list
        .to_query()
        .fetch_all(
            pool.clone(),
            sqlx::query_as("select id, name from users order by id"),
        )
        .await?;
    assert_eq!(
        listed,
        vec![(7, "Linus".to_owned()), (42, "Grace".to_owned())]
    );

    sqlx::query("insert into users (id, name) values (?, ?)")
        .bind(99_i64)
        .bind("New")
        .execute(&pool)
        .await?;

    let listed_cached = prepared_list
        .to_query()
        .fetch_all(
            pool,
            sqlx::query_as("select id, name from users order by id"),
        )
        .await?;
    assert_eq!(
        listed_cached,
        vec![(7, "Linus".to_owned()), (42, "Grace".to_owned())]
    );

    Ok(())
}
