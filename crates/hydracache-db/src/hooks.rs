use std::fmt;

use thiserror::Error;

pub const HOOK_SCHEMA_ARTIFACT: &str = "hydracache_hook_schema";
pub const HOOK_SCHEMA_VERSION: i64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookDialect {
    Postgres,
    MySql,
    Sqlite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookOperation {
    Insert,
    Update,
    Delete,
}

impl HookOperation {
    fn as_str(self) -> &'static str {
        match self {
            Self::Insert => "insert",
            Self::Update => "update",
            Self::Delete => "delete",
        }
    }

    fn sqlite_row_ref(self) -> &'static str {
        match self {
            Self::Insert | Self::Update => "NEW",
            Self::Delete => "OLD",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookInvalidationTarget {
    KeyColumn { column: String },
    Tag { tag: String },
    TagColumn { prefix: String, column: String },
    Entity { entity: String, key_column: String },
    Collection { collection: String },
}

impl HookInvalidationTarget {
    pub fn key_column(column: impl Into<String>) -> Self {
        Self::KeyColumn {
            column: column.into(),
        }
    }

    pub fn tag(tag: impl Into<String>) -> Self {
        Self::Tag { tag: tag.into() }
    }

    pub fn tag_column(prefix: impl Into<String>, column: impl Into<String>) -> Self {
        Self::TagColumn {
            prefix: prefix.into(),
            column: column.into(),
        }
    }

    pub fn entity(entity: impl Into<String>, key_column: impl Into<String>) -> Self {
        Self::Entity {
            entity: entity.into(),
            key_column: key_column.into(),
        }
    }

    pub fn collection(collection: impl Into<String>) -> Self {
        Self::Collection {
            collection: collection.into(),
        }
    }

    fn validate(&self) -> Result<(), HookError> {
        match self {
            Self::KeyColumn { column } if column.trim().is_empty() => {
                Err(HookError::MissingColumn("key".to_owned()))
            }
            Self::TagColumn { prefix, column } => {
                if prefix.trim().is_empty() {
                    return Err(HookError::MissingLiteral("tag prefix".to_owned()));
                }
                if column.trim().is_empty() {
                    return Err(HookError::MissingColumn("tag".to_owned()));
                }
                Ok(())
            }
            Self::Entity { entity, key_column } => {
                if entity.trim().is_empty() {
                    return Err(HookError::MissingLiteral("entity".to_owned()));
                }
                if key_column.trim().is_empty() {
                    return Err(HookError::MissingColumn("entity key".to_owned()));
                }
                Ok(())
            }
            Self::Tag { tag } if tag.trim().is_empty() => {
                Err(HookError::MissingLiteral("tag".to_owned()))
            }
            Self::Collection { collection } if collection.trim().is_empty() => {
                Err(HookError::MissingLiteral("collection".to_owned()))
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookOp {
    operation: HookOperation,
    target: HookInvalidationTarget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookSchemaVersion {
    pub artifact: String,
    pub version: i64,
    pub table: String,
    pub dialect: HookDialect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookPlan {
    dialect: HookDialect,
    table: String,
    namespace: String,
    ops: Vec<HookOp>,
}

impl HookPlan {
    pub fn sqlite(table: &str) -> Self {
        Self::new(HookDialect::Sqlite, table)
    }

    pub fn postgres(table: &str) -> Self {
        Self::new(HookDialect::Postgres, table)
    }

    pub fn mysql(table: &str) -> Self {
        Self::new(HookDialect::MySql, table)
    }

    pub fn new(dialect: HookDialect, table: &str) -> Self {
        Self {
            dialect,
            table: table.to_owned(),
            namespace: "db".to_owned(),
            ops: Vec::new(),
        }
    }

    pub fn namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    pub fn on_insert(self, target: HookInvalidationTarget) -> Self {
        self.on(HookOperation::Insert, target)
    }

    pub fn on_update(self, target: HookInvalidationTarget) -> Self {
        self.on(HookOperation::Update, target)
    }

    pub fn on_delete(self, target: HookInvalidationTarget) -> Self {
        self.on(HookOperation::Delete, target)
    }

    pub fn on(mut self, operation: HookOperation, target: HookInvalidationTarget) -> Self {
        self.ops.push(HookOp { operation, target });
        self
    }

    pub fn schema_version(&self) -> HookSchemaVersion {
        HookSchemaVersion {
            artifact: HOOK_SCHEMA_ARTIFACT.to_owned(),
            version: HOOK_SCHEMA_VERSION,
            table: self.table.clone(),
            dialect: self.dialect,
        }
    }

    pub fn render_sql(&self) -> Result<String, HookError> {
        Ok(self.render_statements()?.join("\n\n"))
    }

    pub fn render_statements(&self) -> Result<Vec<String>, HookError> {
        self.validate()?;
        match self.dialect {
            HookDialect::Sqlite => self.render_sqlite(),
            HookDialect::Postgres => self.render_postgres(),
            HookDialect::MySql => self.render_mysql(),
        }
    }

    #[cfg(feature = "sqlx-outbox")]
    pub async fn install_sqlite(&self, pool: &sqlx::SqlitePool) -> crate::Result<()> {
        for statement in self.render_statements()? {
            sqlx::query(&statement)
                .execute(pool)
                .await
                .map_err(|error| {
                    crate::DbCacheError::from(hydracache::CacheError::Backend(format!(
                        "sqlite hook install error: {error}"
                    )))
                })?;
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), HookError> {
        validate_identifier(&self.table)?;
        if self.namespace.trim().is_empty() {
            return Err(HookError::MissingLiteral("namespace".to_owned()));
        }
        if self.ops.is_empty() {
            return Err(HookError::NoOperations);
        }
        for op in &self.ops {
            op.target.validate()?;
        }
        Ok(())
    }

    fn render_sqlite(&self) -> Result<Vec<String>, HookError> {
        let mut statements = hook_schema_statements_sqlite(&self.table);
        for operation in [
            HookOperation::Insert,
            HookOperation::Update,
            HookOperation::Delete,
        ] {
            let targets: Vec<_> = self
                .ops
                .iter()
                .filter(|op| op.operation == operation)
                .map(|op| &op.target)
                .collect();
            if targets.is_empty() {
                continue;
            }

            let trigger_name = format!("hydracache_{}_{}_outbox", self.table, operation.as_str());
            let mut body = String::new();
            for target in targets {
                body.push_str(&render_sqlite_insert(
                    &self.namespace,
                    &self.table,
                    operation,
                    target,
                )?);
                body.push('\n');
            }
            statements.push(format!(
                "create trigger if not exists {trigger_name}\nafter {} on {}\nbegin\n{}end;",
                operation.as_str(),
                self.table,
                body
            ));
        }
        Ok(statements)
    }

    fn render_postgres(&self) -> Result<Vec<String>, HookError> {
        let mut statements = hook_schema_statements_postgres(&self.table);
        for operation in [
            HookOperation::Insert,
            HookOperation::Update,
            HookOperation::Delete,
        ] {
            let targets: Vec<_> = self
                .ops
                .iter()
                .filter(|op| op.operation == operation)
                .map(|op| &op.target)
                .collect();
            if targets.is_empty() {
                continue;
            }
            let function_name =
                format!("hydracache_{}_{}_outbox_fn", self.table, operation.as_str());
            let trigger_name = format!("hydracache_{}_{}_outbox", self.table, operation.as_str());
            let mut body = String::new();
            for target in targets {
                body.push_str(&render_postgres_insert(
                    &self.namespace,
                    &self.table,
                    operation,
                    target,
                )?);
                body.push('\n');
            }
            statements.push(format!(
                "create or replace function {function_name}() returns trigger as $$\nbegin\n{}return {};\nend;\n$$ language plpgsql;",
                body,
                operation.sqlite_row_ref()
            ));
            statements.push(format!(
                "drop trigger if exists {trigger_name} on {};\ncreate trigger {trigger_name}\nafter {} on {}\nfor each row execute function {function_name}();",
                self.table,
                operation.as_str(),
                self.table
            ));
        }
        Ok(statements)
    }

    fn render_mysql(&self) -> Result<Vec<String>, HookError> {
        let mut statements = hook_schema_statements_mysql(&self.table);
        for operation in [
            HookOperation::Insert,
            HookOperation::Update,
            HookOperation::Delete,
        ] {
            let targets: Vec<_> = self
                .ops
                .iter()
                .filter(|op| op.operation == operation)
                .map(|op| &op.target)
                .collect();
            if targets.is_empty() {
                continue;
            }
            let trigger_name = format!("hydracache_{}_{}_outbox", self.table, operation.as_str());
            let mut body = String::new();
            for target in targets {
                body.push_str(&render_mysql_insert(
                    &self.namespace,
                    &self.table,
                    operation,
                    target,
                )?);
                body.push('\n');
            }
            statements.push(format!(
                "drop trigger if exists {trigger_name};\ncreate trigger {trigger_name}\nafter {} on {}\nfor each row\nbegin\n{}end;",
                operation.as_str(),
                self.table,
                body
            ));
        }
        Ok(statements)
    }
}

#[derive(Debug, Error)]
pub enum HookError {
    #[error("hook plan requires at least one operation")]
    NoOperations,
    #[error("invalid SQL identifier `{0}`")]
    InvalidIdentifier(String),
    #[error("hook target is missing required {0} column")]
    MissingColumn(String),
    #[error("hook target is missing required {0}")]
    MissingLiteral(String),
}

impl From<HookError> for crate::DbCacheError {
    fn from(error: HookError) -> Self {
        hydracache::CacheError::Backend(format!("hydracache hook error: {error}")).into()
    }
}

fn hook_schema_statements_sqlite(table: &str) -> Vec<String> {
    vec![
        "create table if not exists hydracache_hook_schema (
    artifact text primary key,
    version integer not null,
    table_name text not null,
    installed_at_ms integer not null
)"
        .to_owned(),
        format!(
            "insert or replace into hydracache_hook_schema (artifact, version, table_name, installed_at_ms)
values ('{HOOK_SCHEMA_ARTIFACT}', {HOOK_SCHEMA_VERSION}, '{}', cast(strftime('%s', 'now') as integer) * 1000)",
            sql_literal(table)
        ),
    ]
}

fn hook_schema_statements_postgres(table: &str) -> Vec<String> {
    vec![
        "create table if not exists hydracache_hook_schema (
    artifact text primary key,
    version bigint not null,
    table_name text not null,
    installed_at_ms bigint not null
);"
        .to_owned(),
        format!(
            "insert into hydracache_hook_schema (artifact, version, table_name, installed_at_ms)
values ('{HOOK_SCHEMA_ARTIFACT}', {HOOK_SCHEMA_VERSION}, '{}', (extract(epoch from clock_timestamp()) * 1000)::bigint)
on conflict (artifact) do update set version = excluded.version, table_name = excluded.table_name, installed_at_ms = excluded.installed_at_ms;",
            sql_literal(table)
        ),
    ]
}

fn hook_schema_statements_mysql(table: &str) -> Vec<String> {
    vec![
        "create table if not exists hydracache_hook_schema (
    artifact varchar(191) primary key,
    version bigint not null,
    table_name varchar(191) not null,
    installed_at_ms bigint not null
);"
        .to_owned(),
        format!(
            "insert into hydracache_hook_schema (artifact, version, table_name, installed_at_ms)
values ('{HOOK_SCHEMA_ARTIFACT}', {HOOK_SCHEMA_VERSION}, '{}', unix_timestamp(current_timestamp(3)) * 1000)
on duplicate key update version = values(version), table_name = values(table_name), installed_at_ms = values(installed_at_ms);",
            sql_literal(table)
        ),
    ]
}

fn render_sqlite_insert(
    namespace: &str,
    table: &str,
    operation: HookOperation,
    target: &HookInvalidationTarget,
) -> Result<String, HookError> {
    let row = operation.sqlite_row_ref();
    let target = SqlTargetExpr::sqlite(row, target)?;
    let commit = format!(
        "'hook:{table}:{}:' || {}",
        operation.as_str(),
        target.value_expr
    );
    let target_hash = target.hash_expr.clone();
    Ok(format!(
        "insert or ignore into hydracache_invalidation_outbox (
    id, namespace, commit_position, target_hash, intent_kind,
    cache_key, cache_tag, entity_name, collection_name, reason,
    created_at_ms, available_at_ms
) values (
    'hook:{namespace}:' || {commit} || ':' || {target_hash},
    '{namespace}',
    {commit},
    {target_hash},
    '{}',
    {},
    {},
    {},
    {},
    'hydracache hook {table} {}',
    cast(strftime('%s', 'now') as integer) * 1000,
    cast(strftime('%s', 'now') as integer) * 1000
);",
        target.intent_kind,
        target.cache_key_expr,
        target.cache_tag_expr,
        target.entity_name_expr,
        target.collection_name_expr,
        operation.as_str()
    ))
}

fn render_postgres_insert(
    namespace: &str,
    table: &str,
    operation: HookOperation,
    target: &HookInvalidationTarget,
) -> Result<String, HookError> {
    let row = operation.sqlite_row_ref();
    let target = SqlTargetExpr::postgres(row, target)?;
    let commit = format!(
        "'hook:{table}:{}:' || {}",
        operation.as_str(),
        target.value_expr
    );
    Ok(format!(
        "insert into hydracache_invalidation_outbox (
    id, namespace, commit_position, target_hash, intent_kind,
    cache_key, cache_tag, entity_name, collection_name, reason,
    created_at_ms, available_at_ms
) values (
    'hook:{namespace}:' || {commit} || ':' || {hash},
    '{namespace}',
    {commit},
    {hash},
    '{kind}',
    {cache_key},
    {cache_tag},
    {entity_name},
    {collection_name},
    'hydracache hook {table} {op}',
    (extract(epoch from clock_timestamp()) * 1000)::bigint,
    (extract(epoch from clock_timestamp()) * 1000)::bigint
) on conflict (namespace, commit_position, target_hash) do nothing;",
        hash = target.hash_expr,
        kind = target.intent_kind,
        cache_key = target.cache_key_expr,
        cache_tag = target.cache_tag_expr,
        entity_name = target.entity_name_expr,
        collection_name = target.collection_name_expr,
        op = operation.as_str(),
    ))
}

fn render_mysql_insert(
    namespace: &str,
    table: &str,
    operation: HookOperation,
    target: &HookInvalidationTarget,
) -> Result<String, HookError> {
    let row = operation.sqlite_row_ref();
    let target = SqlTargetExpr::mysql(row, target)?;
    let commit = format!(
        "concat('hook:{table}:{}:', {})",
        operation.as_str(),
        target.value_expr
    );
    Ok(format!(
        "insert ignore into hydracache_invalidation_outbox (
    id, namespace, commit_position, target_hash, intent_kind,
    cache_key, cache_tag, entity_name, collection_name, reason,
    created_at_ms, available_at_ms
) values (
    concat('hook:{namespace}:', {commit}, ':', {hash}),
    '{namespace}',
    {commit},
    {hash},
    '{kind}',
    {cache_key},
    {cache_tag},
    {entity_name},
    {collection_name},
    'hydracache hook {table} {op}',
    unix_timestamp(current_timestamp(3)) * 1000,
    unix_timestamp(current_timestamp(3)) * 1000
);",
        hash = target.hash_expr,
        kind = target.intent_kind,
        cache_key = target.cache_key_expr,
        cache_tag = target.cache_tag_expr,
        entity_name = target.entity_name_expr,
        collection_name = target.collection_name_expr,
        op = operation.as_str(),
    ))
}

struct SqlTargetExpr {
    intent_kind: &'static str,
    value_expr: String,
    hash_expr: String,
    cache_key_expr: String,
    cache_tag_expr: String,
    entity_name_expr: String,
    collection_name_expr: String,
}

impl SqlTargetExpr {
    fn sqlite(row: &str, target: &HookInvalidationTarget) -> Result<Self, HookError> {
        Self::new(row, target, SqlFlavor::Sqlite)
    }

    fn postgres(row: &str, target: &HookInvalidationTarget) -> Result<Self, HookError> {
        Self::new(row, target, SqlFlavor::Postgres)
    }

    fn mysql(row: &str, target: &HookInvalidationTarget) -> Result<Self, HookError> {
        Self::new(row, target, SqlFlavor::MySql)
    }

    fn new(
        row: &str,
        target: &HookInvalidationTarget,
        flavor: SqlFlavor,
    ) -> Result<Self, HookError> {
        target.validate()?;
        let null = "null".to_owned();
        match target {
            HookInvalidationTarget::KeyColumn { column } => {
                validate_identifier(column)?;
                let value = flavor.column_text(row, column);
                Ok(Self {
                    intent_kind: "key",
                    value_expr: value.clone(),
                    hash_expr: flavor.concat(&["'key:'", &value]),
                    cache_key_expr: value,
                    cache_tag_expr: null.clone(),
                    entity_name_expr: null.clone(),
                    collection_name_expr: null,
                })
            }
            HookInvalidationTarget::Tag { tag } => {
                let value = flavor.literal(tag);
                Ok(Self {
                    intent_kind: "tag",
                    value_expr: value.clone(),
                    hash_expr: flavor.concat(&["'tag:'", &value]),
                    cache_key_expr: null.clone(),
                    cache_tag_expr: value,
                    entity_name_expr: null.clone(),
                    collection_name_expr: null,
                })
            }
            HookInvalidationTarget::TagColumn { prefix, column } => {
                validate_identifier(column)?;
                let column = flavor.column_text(row, column);
                let value = flavor.concat(&[&flavor.literal(&format!("{prefix}:")), &column]);
                Ok(Self {
                    intent_kind: "tag",
                    value_expr: value.clone(),
                    hash_expr: flavor.concat(&["'tag:'", &value]),
                    cache_key_expr: null.clone(),
                    cache_tag_expr: value,
                    entity_name_expr: null.clone(),
                    collection_name_expr: null,
                })
            }
            HookInvalidationTarget::Entity { entity, key_column } => {
                validate_identifier(key_column)?;
                let key = flavor.column_text(row, key_column);
                let entity_literal = flavor.literal(entity);
                Ok(Self {
                    intent_kind: "entity",
                    value_expr: key.clone(),
                    hash_expr: flavor.concat(&["'entity:'", &entity_literal, "':'", &key]),
                    cache_key_expr: key,
                    cache_tag_expr: null.clone(),
                    entity_name_expr: entity_literal,
                    collection_name_expr: null,
                })
            }
            HookInvalidationTarget::Collection { collection } => {
                let value = flavor.literal(collection);
                Ok(Self {
                    intent_kind: "collection",
                    value_expr: value.clone(),
                    hash_expr: flavor.concat(&["'collection:'", &value]),
                    cache_key_expr: null.clone(),
                    cache_tag_expr: null.clone(),
                    entity_name_expr: null.clone(),
                    collection_name_expr: value,
                })
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum SqlFlavor {
    Sqlite,
    Postgres,
    MySql,
}

impl SqlFlavor {
    fn literal(self, value: &str) -> String {
        format!("'{}'", sql_literal(value))
    }

    fn column_text(self, row: &str, column: &str) -> String {
        match self {
            Self::Sqlite | Self::Postgres => format!("cast({row}.{column} as text)"),
            Self::MySql => format!("cast({row}.{column} as char)"),
        }
    }

    fn concat(self, parts: &[&str]) -> String {
        match self {
            Self::Sqlite | Self::Postgres => parts.join(" || "),
            Self::MySql => format!("concat({})", parts.join(", ")),
        }
    }
}

fn validate_identifier(identifier: &str) -> Result<(), HookError> {
    let valid = !identifier.is_empty()
        && identifier
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        && identifier
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_');
    if valid {
        Ok(())
    } else {
        Err(HookError::InvalidIdentifier(identifier.to_owned()))
    }
}

fn sql_literal(value: &str) -> String {
    value.replace('\'', "''")
}

impl fmt::Display for HookDialect {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Postgres => "postgres",
            Self::MySql => "mysql",
            Self::Sqlite => "sqlite",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_version_names_table_and_dialect() {
        let version = HookPlan::sqlite("users").schema_version();

        assert_eq!(version.artifact, HOOK_SCHEMA_ARTIFACT);
        assert_eq!(version.version, HOOK_SCHEMA_VERSION);
        assert_eq!(version.table, "users");
        assert_eq!(version.dialect, HookDialect::Sqlite);
    }
}
