use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use hydracache::HydraCache;
use hydracache_sqlx::SqlxCache;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use testcontainers_modules::{
    postgres,
    testcontainers::{runners::AsyncRunner, ImageExt},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
}
