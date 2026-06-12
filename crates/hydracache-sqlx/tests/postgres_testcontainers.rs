use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use hydracache::HydraCache;
use hydracache_sqlx::{HydraCacheEntity, PreparedQueryPolicy, SqlxCache, SqlxQueryExt};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use testcontainers_modules::{
    postgres,
    testcontainers::{runners::AsyncRunner, ImageExt},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, HydraCacheEntity)]
#[hydracache(
    entity = "cache-entity-user",
    collection = "cache-entity-users",
    id = i64
)]
struct User {
    id: i64,
    name: String,
}

#[tokio::test]
async fn sqlx_adapter_caches_real_postgres_query_results_when_docker_is_available(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let Some(container) = start_postgres_or_skip().await else {
        return Ok(());
    };

    let host = container.get_host().await?;
    let port = container.get_host_port_ipv4(5432).await?;
    let database_url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await?;

    sqlx::query("create table users (id bigint primary key, name text not null)")
        .execute(&pool)
        .await?;
    sqlx::query("insert into users (id, name) values ($1, $2)")
        .bind(42_i64)
        .bind("Ada")
        .execute(&pool)
        .await?;
    sqlx::query("insert into users (id, name) values ($1, $2)")
        .bind(7_i64)
        .bind("Linus")
        .execute(&pool)
        .await?;

    let cache = HydraCache::local().build();
    let queries = SqlxCache::new(cache, "postgres");
    let loader_calls = Arc::new(AtomicUsize::new(0));

    let first = load_user(&queries, &pool, &loader_calls).await?;
    assert_eq!(first.name, "Ada");
    assert_eq!(loader_calls.load(Ordering::SeqCst), 1);

    sqlx::query("update users set name = $1 where id = $2")
        .bind("Grace")
        .bind(42_i64)
        .execute(&pool)
        .await?;

    let cached = load_user(&queries, &pool, &loader_calls).await?;
    assert_eq!(cached.name, "Ada");
    assert_eq!(loader_calls.load(Ordering::SeqCst), 1);

    assert_eq!(queries.cache().invalidate_tag("user:42").await?, 1);

    let reloaded = load_user(&queries, &pool, &loader_calls).await?;
    assert_eq!(reloaded.name, "Grace");
    assert_eq!(loader_calls.load(Ordering::SeqCst), 2);

    let helper_first = queries
        .cached::<(i64, String)>()
        .key("helper:user:42")
        .tag("user:42")
        .sqlx_one(
            pool.clone(),
            sqlx::query_as("select id, name from users where id = $1").bind(42_i64),
        )
        .await?;
    assert_eq!(helper_first, (42, "Grace".to_owned()));

    sqlx::query("update users set name = $1 where id = $2")
        .bind("Katherine")
        .bind(42_i64)
        .execute(&pool)
        .await?;

    let helper_cached = queries
        .cached::<(i64, String)>()
        .key("helper:user:42")
        .tag("user:42")
        .sqlx_one(
            pool.clone(),
            sqlx::query_as("select id, name from users where id = $1").bind(42_i64),
        )
        .await?;
    assert_eq!(helper_cached, (42, "Grace".to_owned()));

    let entity_helper_first = queries
        .entity::<(i64, String)>("helper-user", 42)
        .collection_tag("helper-users")
        .sqlx_one(
            pool.clone(),
            sqlx::query_as("select id, name from users where id = $1").bind(42_i64),
        )
        .await?;
    assert_eq!(entity_helper_first, (42, "Katherine".to_owned()));

    sqlx::query("update users set name = $1 where id = $2")
        .bind("Margaret")
        .bind(42_i64)
        .execute(&pool)
        .await?;

    let entity_helper_cached = queries
        .entity::<(i64, String)>("helper-user", 42)
        .collection_tag("helper-users")
        .sqlx_one(
            pool.clone(),
            sqlx::query_as("select id, name from users where id = $1").bind(42_i64),
        )
        .await?;
    assert_eq!(entity_helper_cached, (42, "Katherine".to_owned()));

    assert_eq!(queries.cache().invalidate_tag("helper-user:42").await?, 1);

    let entity_helper_reloaded = queries
        .entity::<(i64, String)>("helper-user", 42)
        .collection_tag("helper-users")
        .sqlx_one(
            pool.clone(),
            sqlx::query_as("select id, name from users where id = $1").bind(42_i64),
        )
        .await?;
    assert_eq!(entity_helper_reloaded, (42, "Margaret".to_owned()));

    let missing = queries
        .cached::<(i64, String)>()
        .key("helper:user:missing")
        .tag("user:missing")
        .sqlx_optional(
            pool.clone(),
            sqlx::query_as("select id, name from users where id = $1").bind(999_i64),
        )
        .await?;
    assert_eq!(missing, None);

    let optional_user = queries
        .cached::<(i64, String)>()
        .key("helper:user:7")
        .tag("user:7")
        .sqlx_optional(
            pool.clone(),
            sqlx::query_as("select id, name from users where id = $1").bind(7_i64),
        )
        .await?;
    assert_eq!(optional_user, Some((7, "Linus".to_owned())));

    sqlx::query("update users set name = $1 where id = $2")
        .bind("Barbara")
        .bind(7_i64)
        .execute(&pool)
        .await?;

    let optional_cached = queries
        .cached::<(i64, String)>()
        .key("helper:user:7")
        .tag("user:7")
        .sqlx_optional(
            pool.clone(),
            sqlx::query_as("select id, name from users where id = $1").bind(7_i64),
        )
        .await?;
    assert_eq!(optional_cached, Some((7, "Linus".to_owned())));

    let all_users = queries
        .cached::<(i64, String)>()
        .key("helper:users:all")
        .tag("users")
        .sqlx_all(
            pool.clone(),
            sqlx::query_as("select id, name from users order by id"),
        )
        .await?;
    assert_eq!(
        all_users,
        vec![(7, "Barbara".to_owned()), (42, "Margaret".to_owned())]
    );

    let collection_helper_first = queries
        .collection::<(i64, String)>("helper-users:all")
        .sqlx_all(
            pool.clone(),
            sqlx::query_as("select id, name from users order by id"),
        )
        .await?;
    assert_eq!(
        collection_helper_first,
        vec![(7, "Barbara".to_owned()), (42, "Margaret".to_owned())]
    );

    sqlx::query("insert into users (id, name) values ($1, $2)")
        .bind(99_i64)
        .bind("New")
        .execute(&pool)
        .await?;

    let collection_helper_cached = queries
        .collection::<(i64, String)>("helper-users:all")
        .sqlx_all(
            pool.clone(),
            sqlx::query_as("select id, name from users order by id"),
        )
        .await?;
    assert_eq!(
        collection_helper_cached,
        vec![(7, "Barbara".to_owned()), (42, "Margaret".to_owned())]
    );

    assert_eq!(
        queries.cache().invalidate_tag("helper-users%3Aall").await?,
        1
    );

    let collection_helper_reloaded = queries
        .collection::<(i64, String)>("helper-users:all")
        .sqlx_all(
            pool.clone(),
            sqlx::query_as("select id, name from users order by id"),
        )
        .await?;
    assert_eq!(
        collection_helper_reloaded,
        vec![
            (7, "Barbara".to_owned()),
            (42, "Margaret".to_owned()),
            (99, "New".to_owned())
        ]
    );

    let cache_entity_first = load_user_with_cache_entity(&queries, &pool).await?;
    assert_eq!(cache_entity_first.name, "Margaret");

    sqlx::query("update users set name = $1 where id = $2")
        .bind("Rosalind")
        .bind(42_i64)
        .execute(&pool)
        .await?;

    let cache_entity_cached = load_user_with_cache_entity(&queries, &pool).await?;
    assert_eq!(cache_entity_cached.name, "Margaret");

    assert_eq!(
        queries.cache().invalidate_tag("cache-entity-users").await?,
        1
    );

    let cache_entity_reloaded = load_user_with_cache_entity(&queries, &pool).await?;
    assert_eq!(cache_entity_reloaded.name, "Rosalind");

    let prepared_user = queries.prepare::<(i64, String)>(
        PreparedQueryPolicy::for_cache_entity::<User>().with_name("prepared-load-user"),
    );

    let prepared_first = prepared_user
        .for_id(42)
        .sqlx_one(
            pool.clone(),
            sqlx::query_as("select id, name from users where id = $1").bind(42_i64),
        )
        .await?;
    assert_eq!(prepared_first, (42, "Rosalind".to_owned()));

    sqlx::query("update users set name = $1 where id = $2")
        .bind("Hedy")
        .bind(42_i64)
        .execute(&pool)
        .await?;

    let prepared_cached = prepared_user
        .for_id(42)
        .sqlx_one(
            pool.clone(),
            sqlx::query_as("select id, name from users where id = $1").bind(42_i64),
        )
        .await?;
    assert_eq!(prepared_cached, (42, "Rosalind".to_owned()));

    assert_eq!(
        queries.cache().invalidate_tag("cache-entity-users").await?,
        1
    );

    let prepared_reloaded = prepared_user
        .for_id(42)
        .sqlx_one(
            pool.clone(),
            sqlx::query_as("select id, name from users where id = $1").bind(42_i64),
        )
        .await?;
    assert_eq!(prepared_reloaded, (42, "Hedy".to_owned()));

    let prepared_collection = queries.prepare::<(i64, String)>(
        PreparedQueryPolicy::named("prepared-list-users").collection("prepared-users:all"),
    );
    let prepared_collection_first = prepared_collection
        .to_query()
        .sqlx_all(
            pool.clone(),
            sqlx::query_as("select id, name from users where id > $1 order by id").bind(0_i64),
        )
        .await?;
    assert_eq!(
        prepared_collection_first,
        vec![
            (7, "Barbara".to_owned()),
            (42, "Hedy".to_owned()),
            (99, "New".to_owned())
        ]
    );

    let no_users = queries
        .cached::<(i64, String)>()
        .key("helper:users:none")
        .tag("users:none")
        .sqlx_all(
            pool.clone(),
            sqlx::query_as("select id, name from users where id < $1 order by id").bind(0_i64),
        )
        .await?;
    assert!(no_users.is_empty());

    sqlx::query("insert into users (id, name) values ($1, $2)")
        .bind(-1_i64)
        .bind("Negative")
        .execute(&pool)
        .await?;

    let no_users_cached = queries
        .cached::<(i64, String)>()
        .key("helper:users:none")
        .tag("users:none")
        .sqlx_all(
            pool.clone(),
            sqlx::query_as("select id, name from users where id < $1 order by id").bind(0_i64),
        )
        .await?;
    assert!(no_users_cached.is_empty());

    let failed = queries
        .cached::<(i64, String)>()
        .key("helper:broken")
        .sqlx_one(
            pool.clone(),
            sqlx::query_as("select id, missing_column from users where id = $1").bind(42_i64),
        )
        .await;
    assert!(failed.is_err());
    assert!(!queries.cache().contains_key("postgres:helper:broken").await);

    Ok(())
}

async fn start_postgres_or_skip(
) -> Option<testcontainers_modules::testcontainers::ContainerAsync<postgres::Postgres>> {
    match postgres::Postgres::default()
        .with_tag("16-alpine")
        .start()
        .await
    {
        Ok(container) => Some(container),
        Err(error) => {
            eprintln!(
                "skipping Postgres testcontainers integration test because Docker is unavailable: {error}"
            );
            None
        }
    }
}

async fn load_user(
    queries: &SqlxCache,
    pool: &sqlx::PgPool,
    loader_calls: &Arc<AtomicUsize>,
) -> hydracache_sqlx::Result<User> {
    let pool = pool.clone();
    let loader_calls = Arc::clone(loader_calls);

    queries
        .cached::<User>()
        .key("user:42")
        .tag("user:42")
        .fetch_with(move || async move {
            loader_calls.fetch_add(1, Ordering::SeqCst);
            let (id, name): (i64, String) =
                sqlx::query_as("select id, name from users where id = $1")
                    .bind(42_i64)
                    .fetch_one(&pool)
                    .await?;
            Ok::<_, sqlx::Error>(User { id, name })
        })
        .await
        .map_err(Into::into)
}

async fn load_user_with_cache_entity(
    queries: &SqlxCache,
    pool: &sqlx::PgPool,
) -> hydracache_sqlx::Result<User> {
    let pool = pool.clone();

    queries
        .for_entity::<User>(42)
        .fetch_with(move || async move {
            let (id, name): (i64, String) =
                sqlx::query_as("select id, name from users where id = $1")
                    .bind(42_i64)
                    .fetch_one(&pool)
                    .await?;
            Ok::<_, sqlx::Error>(User { id, name })
        })
        .await
        .map_err(Into::into)
}
