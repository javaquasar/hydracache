use async_trait::async_trait;
use hydracache_core::CacheCodec;
use hydracache_db::{DbAdapterKind, DbQuery, DbResultShape};
use serde::{de::DeserializeOwned, Serialize};
use sqlx::{query::QueryAs, Database, Executor, FromRow, IntoArguments};

use crate::Result;

/// Convenience SQLx execution methods for [`DbQuery`].
///
/// These helpers keep SQLx responsible for query construction and row mapping,
/// while HydraCache owns keying, tags, TTL, serialization, and local
/// single-flight. Use [`DbQuery::fetch_with`] when you need a transaction,
/// custom repository call, or a database client that is not pool-like.
#[async_trait]
pub trait SqlxQueryExt<T, C>
where
    C: CacheCodec,
{
    /// Execute a SQLx query on miss and cache exactly one row.
    async fn sqlx_one<'q, DB, A, E>(self, executor: E, query: QueryAs<'q, DB, T, A>) -> Result<T>
    where
        'q: 'static,
        T: Serialize + DeserializeOwned + Send + Unpin + for<'r> FromRow<'r, DB::Row> + 'static,
        DB: Database + Send + Sync + 'static,
        A: IntoArguments<'q, DB> + Send + 'static,
        E: Send + Sync + 'static,
        for<'c> &'c E: Executor<'c, Database = DB>;

    /// Execute a SQLx query on miss and cache either one row or `None`.
    async fn sqlx_optional<'q, DB, A, E>(
        self,
        executor: E,
        query: QueryAs<'q, DB, T, A>,
    ) -> Result<Option<T>>
    where
        'q: 'static,
        T: Serialize + DeserializeOwned + Send + Unpin + for<'r> FromRow<'r, DB::Row> + 'static,
        DB: Database + Send + Sync + 'static,
        A: IntoArguments<'q, DB> + Send + 'static,
        E: Send + Sync + 'static,
        for<'c> &'c E: Executor<'c, Database = DB>;

    /// Execute a SQLx query on miss and cache all returned rows.
    async fn sqlx_all<'q, DB, A, E>(
        self,
        executor: E,
        query: QueryAs<'q, DB, T, A>,
    ) -> Result<Vec<T>>
    where
        'q: 'static,
        T: Serialize + DeserializeOwned + Send + Unpin + for<'r> FromRow<'r, DB::Row> + 'static,
        DB: Database + Send + Sync + 'static,
        A: IntoArguments<'q, DB> + Send + 'static,
        E: Send + Sync + 'static,
        for<'c> &'c E: Executor<'c, Database = DB>;
}

#[async_trait]
impl<T, C> SqlxQueryExt<T, C> for DbQuery<T, C>
where
    C: CacheCodec,
{
    async fn sqlx_one<'q, DB, A, E>(self, executor: E, query: QueryAs<'q, DB, T, A>) -> Result<T>
    where
        'q: 'static,
        T: Serialize + DeserializeOwned + Send + Unpin + for<'r> FromRow<'r, DB::Row> + 'static,
        DB: Database + Send + Sync + 'static,
        A: IntoArguments<'q, DB> + Send + 'static,
        E: Send + Sync + 'static,
        for<'c> &'c E: Executor<'c, Database = DB>,
    {
        self.adapter_context(DbAdapterKind::Sqlx, DbResultShape::One)
            .fetch_value_with(move || async move { query.fetch_one(&executor).await })
            .await
            .map_err(Into::into)
    }

    async fn sqlx_optional<'q, DB, A, E>(
        self,
        executor: E,
        query: QueryAs<'q, DB, T, A>,
    ) -> Result<Option<T>>
    where
        'q: 'static,
        T: Serialize + DeserializeOwned + Send + Unpin + for<'r> FromRow<'r, DB::Row> + 'static,
        DB: Database + Send + Sync + 'static,
        A: IntoArguments<'q, DB> + Send + 'static,
        E: Send + Sync + 'static,
        for<'c> &'c E: Executor<'c, Database = DB>,
    {
        self.adapter_context(DbAdapterKind::Sqlx, DbResultShape::Optional)
            .fetch_value_with(move || async move { query.fetch_optional(&executor).await })
            .await
            .map_err(Into::into)
    }

    async fn sqlx_all<'q, DB, A, E>(
        self,
        executor: E,
        query: QueryAs<'q, DB, T, A>,
    ) -> Result<Vec<T>>
    where
        'q: 'static,
        T: Serialize + DeserializeOwned + Send + Unpin + for<'r> FromRow<'r, DB::Row> + 'static,
        DB: Database + Send + Sync + 'static,
        A: IntoArguments<'q, DB> + Send + 'static,
        E: Send + Sync + 'static,
        for<'c> &'c E: Executor<'c, Database = DB>,
    {
        self.adapter_context(DbAdapterKind::Sqlx, DbResultShape::All)
            .fetch_value_with(move || async move { query.fetch_all(&executor).await })
            .await
            .map_err(Into::into)
    }
}
