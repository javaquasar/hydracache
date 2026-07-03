use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlparser::ast::{Expr, Join, ObjectName, Query, Select, SetExpr, Statement, TableFactor};
use sqlparser::dialect::{Dialect, MySqlDialect, PostgreSqlDialect, SQLiteDialect};
use sqlparser::parser::Parser;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Relation {
    pub schema: Option<String>,
    pub name: String,
}

impl Relation {
    pub fn table(name: impl Into<String>) -> Self {
        Self {
            schema: None,
            name: normalize_ident(&name.into()),
        }
    }

    pub fn schema_table(schema: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            schema: Some(normalize_ident(&schema.into())),
            name: normalize_ident(&name.into()),
        }
    }

    fn from_object_name(name: &ObjectName) -> Self {
        let parts: Vec<String> = name
            .to_string()
            .split('.')
            .map(normalize_ident)
            .filter(|part| !part.is_empty())
            .collect();
        match parts.as_slice() {
            [table] => Self::table(table),
            [schema, table] => Self::schema_table(schema, table),
            parts if parts.len() >= 2 => {
                let table = parts.last().expect("len checked").clone();
                let schema = parts.get(parts.len() - 2).expect("len checked").clone();
                Self {
                    schema: Some(schema),
                    name: table,
                }
            }
            _ => Self::table(name.to_string()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SqlDialect {
    Postgres,
    MySql,
    Sqlite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyLintMode {
    Warn,
    DenyMissingDependencies,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LintStatus {
    Clean,
    MissingDependencies(Vec<Relation>),
    ExtraDependencies(Vec<Relation>),
    Inconclusive(String),
}

impl LintStatus {
    pub fn is_clean(&self) -> bool {
        matches!(self, Self::Clean)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LintFinding {
    MissingDependencies(Vec<Relation>),
    ExtraDependencies(Vec<Relation>),
    Inconclusive(String),
}

impl LintFinding {
    pub fn kind(&self) -> LintFindingKind {
        match self {
            Self::MissingDependencies(_) => LintFindingKind::MissingDependencies,
            Self::ExtraDependencies(_) => LintFindingKind::ExtraDependencies,
            Self::Inconclusive(_) => LintFindingKind::Inconclusive,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LintFindingKind {
    MissingDependencies,
    ExtraDependencies,
    Inconclusive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LintSuppression {
    pub finding: LintFindingKind,
    pub reason: String,
}

impl LintSuppression {
    pub fn new(finding: LintFindingKind, reason: impl Into<String>) -> Result<Self, LintError> {
        let reason = reason.into();
        if reason.trim().is_empty() {
            return Err(LintError::EmptySuppressionReason);
        }
        Ok(Self { finding, reason })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LintDiagnostic {
    pub policy: String,
    pub finding: LintFinding,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Baseline {
    pub entries: HashSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaselineDiff {
    pub new_findings: Vec<LintDiagnostic>,
    pub accepted_findings: Vec<LintDiagnostic>,
    pub stale_entries: Vec<String>,
}

impl Baseline {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, LintError> {
        let bytes = fs::read(path)?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), LintError> {
        let bytes = serde_json::to_vec_pretty(self)?;
        fs::write(path, bytes)?;
        Ok(())
    }

    pub fn from_diagnostics<'a>(diagnostics: impl IntoIterator<Item = &'a LintDiagnostic>) -> Self {
        Self {
            entries: diagnostics
                .into_iter()
                .map(|diagnostic| diagnostic.fingerprint.clone())
                .collect(),
        }
    }

    pub fn diff(&self, current: Vec<LintDiagnostic>) -> BaselineDiff {
        let current_fingerprints: HashSet<String> = current
            .iter()
            .map(|diagnostic| diagnostic.fingerprint.clone())
            .collect();
        let mut new_findings = Vec::new();
        let mut accepted_findings = Vec::new();
        for diagnostic in current {
            if self.entries.contains(&diagnostic.fingerprint) {
                accepted_findings.push(diagnostic);
            } else {
                new_findings.push(diagnostic);
            }
        }

        let stale_entries = self
            .entries
            .difference(&current_fingerprints)
            .cloned()
            .collect();

        BaselineDiff {
            new_findings,
            accepted_findings,
            stale_entries,
        }
    }
}

#[derive(Debug, Error)]
pub enum LintError {
    #[error("sql parser error: {0}")]
    Parse(String),
    #[error("suppression reason cannot be empty")]
    EmptySuppressionReason,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct DependencyLint {
    dialect: SqlDialect,
    mode: DependencyLintMode,
}

impl DependencyLint {
    pub fn new(dialect: SqlDialect, mode: DependencyLintMode) -> Self {
        Self { dialect, mode }
    }

    pub fn mode(&self) -> DependencyLintMode {
        self.mode
    }

    pub fn observed_relations(&self, sql: &str) -> Result<Vec<Relation>, LintError> {
        let dialect = self.dialect_impl();
        let statements = Parser::parse_sql(dialect.as_ref(), sql)
            .map_err(|error| LintError::Parse(error.to_string()))?;
        let mut relations = BTreeSet::new();
        for statement in statements {
            collect_statement_relations(&statement, &mut relations);
        }
        Ok(relations.into_iter().collect())
    }

    pub fn check(&self, sql: &str, declared: &[Relation]) -> LintStatus {
        let observed = match self.observed_relations(sql) {
            Ok(relations) => relations,
            Err(error) => return LintStatus::Inconclusive(error.to_string()),
        };
        let observed: BTreeSet<_> = observed.into_iter().collect();
        let declared: BTreeSet<_> = declared.iter().cloned().collect();
        let missing: Vec<_> = observed.difference(&declared).cloned().collect();
        if !missing.is_empty() {
            return LintStatus::MissingDependencies(missing);
        }

        let extra: Vec<_> = declared.difference(&observed).cloned().collect();
        if !extra.is_empty() {
            return LintStatus::ExtraDependencies(extra);
        }

        LintStatus::Clean
    }

    pub fn diagnostics(
        &self,
        policy: impl Into<String>,
        sql: &str,
        declared: &[Relation],
        suppressions: &[LintSuppression],
    ) -> Vec<LintDiagnostic> {
        let policy = policy.into();
        let status = self.check(sql, declared);
        let Some(finding) = finding_from_status(status) else {
            return Vec::new();
        };
        if suppressions
            .iter()
            .any(|suppression| suppression.finding == finding.kind())
        {
            return Vec::new();
        }

        vec![LintDiagnostic {
            fingerprint: fingerprint(&policy, &finding),
            policy,
            finding,
        }]
    }

    fn dialect_impl(&self) -> Box<dyn Dialect> {
        match self.dialect {
            SqlDialect::Postgres => Box::new(PostgreSqlDialect {}),
            SqlDialect::MySql => Box::new(MySqlDialect {}),
            SqlDialect::Sqlite => Box::new(SQLiteDialect {}),
        }
    }
}

fn finding_from_status(status: LintStatus) -> Option<LintFinding> {
    match status {
        LintStatus::Clean => None,
        LintStatus::MissingDependencies(relations) => {
            Some(LintFinding::MissingDependencies(relations))
        }
        LintStatus::ExtraDependencies(relations) => Some(LintFinding::ExtraDependencies(relations)),
        LintStatus::Inconclusive(reason) => Some(LintFinding::Inconclusive(reason)),
    }
}

fn collect_statement_relations(statement: &Statement, relations: &mut BTreeSet<Relation>) {
    match statement {
        Statement::Query(query) => collect_query_relations(query, relations),
        Statement::Insert(insert) => {
            if let Some(source) = &insert.source {
                collect_query_relations(source, relations);
            }
        }
        _ => {}
    }
}

fn collect_query_relations(query: &Query, relations: &mut BTreeSet<Relation>) {
    let cte_names: BTreeSet<String> = query
        .with
        .as_ref()
        .map(|with| {
            with.cte_tables
                .iter()
                .map(|cte| normalize_ident(&cte.alias.name.to_string()))
                .collect()
        })
        .unwrap_or_default();

    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            collect_query_relations(&cte.query, relations);
        }
    }

    collect_set_expr_relations(&query.body, &cte_names, relations);
}

fn collect_set_expr_relations(
    set_expr: &SetExpr,
    cte_names: &BTreeSet<String>,
    relations: &mut BTreeSet<Relation>,
) {
    match set_expr {
        SetExpr::Select(select) => collect_select_relations(select, cte_names, relations),
        SetExpr::Query(query) => collect_query_relations(query, relations),
        SetExpr::SetOperation { left, right, .. } => {
            collect_set_expr_relations(left, cte_names, relations);
            collect_set_expr_relations(right, cte_names, relations);
        }
        _ => {}
    }
}

fn collect_select_relations(
    select: &Select,
    cte_names: &BTreeSet<String>,
    relations: &mut BTreeSet<Relation>,
) {
    for table in &select.from {
        collect_table_factor_relations(&table.relation, cte_names, relations);
        for join in &table.joins {
            collect_join_relations(join, cte_names, relations);
        }
    }
    if let Some(selection) = &select.selection {
        collect_expr_relations(selection, relations);
    }
}

fn collect_join_relations(
    join: &Join,
    cte_names: &BTreeSet<String>,
    relations: &mut BTreeSet<Relation>,
) {
    collect_table_factor_relations(&join.relation, cte_names, relations);
}

fn collect_table_factor_relations(
    factor: &TableFactor,
    cte_names: &BTreeSet<String>,
    relations: &mut BTreeSet<Relation>,
) {
    match factor {
        TableFactor::Table { name, .. } => {
            let relation = Relation::from_object_name(name);
            if !cte_names.contains(&relation.name) {
                relations.insert(relation);
            }
        }
        TableFactor::Derived { subquery, .. } => collect_query_relations(subquery, relations),
        TableFactor::NestedJoin {
            table_with_joins, ..
        } => {
            collect_table_factor_relations(&table_with_joins.relation, cte_names, relations);
            for join in &table_with_joins.joins {
                collect_join_relations(join, cte_names, relations);
            }
        }
        _ => {}
    }
}

fn collect_expr_relations(expr: &Expr, relations: &mut BTreeSet<Relation>) {
    match expr {
        Expr::Subquery(query)
        | Expr::Exists {
            subquery: query, ..
        } => {
            collect_query_relations(query, relations);
        }
        Expr::InSubquery { subquery, .. } => collect_query_relations(subquery, relations),
        Expr::BinaryOp { left, right, .. } => {
            collect_expr_relations(left, relations);
            collect_expr_relations(right, relations);
        }
        Expr::UnaryOp { expr, .. } | Expr::Nested(expr) | Expr::Cast { expr, .. } => {
            collect_expr_relations(expr, relations)
        }
        Expr::Between {
            expr, low, high, ..
        } => {
            collect_expr_relations(expr, relations);
            collect_expr_relations(low, relations);
            collect_expr_relations(high, relations);
        }
        _ => {}
    }
}

fn fingerprint(policy: &str, finding: &LintFinding) -> String {
    let mut hasher = Sha256::new();
    hasher.update(policy.as_bytes());
    hasher.update([0]);
    hasher.update(format!("{finding:?}").as_bytes());
    encode_lower_hex(hasher.finalize().as_ref())
}

fn encode_lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn normalize_ident(identifier: &str) -> String {
    identifier
        .trim()
        .trim_matches('"')
        .trim_matches('`')
        .trim_matches('[')
        .trim_matches(']')
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suppression_reason_is_required() {
        let error = LintSuppression::new(LintFindingKind::Inconclusive, " ").unwrap_err();
        assert!(matches!(error, LintError::EmptySuppressionReason));
    }
}
