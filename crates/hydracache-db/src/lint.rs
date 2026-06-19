#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DeclaredRelation {
    schema: Option<String>,
    name: String,
}

impl DeclaredRelation {
    pub fn table(name: impl Into<String>) -> Self {
        Self {
            schema: None,
            name: normalize_ident(name.into()),
        }
    }

    pub fn schema_table(schema: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            schema: Some(normalize_ident(schema.into())),
            name: normalize_ident(name.into()),
        }
    }

    pub fn schema(&self) -> Option<&str> {
        self.schema.as_deref()
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

pub fn table(name: impl Into<String>) -> DeclaredRelation {
    DeclaredRelation::table(name)
}

pub fn schema_table(schema: impl Into<String>, name: impl Into<String>) -> DeclaredRelation {
    DeclaredRelation::schema_table(schema, name)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclaredLintMode {
    Warn,
    DenyMissingDependencies,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintFinding {
    MissingDependencies,
    ExtraDependencies,
    Inconclusive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintSuppression {
    finding: LintFinding,
    reason: String,
}

impl LintSuppression {
    pub fn new(finding: LintFinding, reason: impl Into<String>) -> Self {
        Self {
            finding,
            reason: reason.into(),
        }
    }

    pub fn finding(&self) -> LintFinding {
        self.finding
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyLintMetadata {
    pub(crate) sql: Option<String>,
    pub(crate) declared: Vec<DeclaredRelation>,
    pub(crate) mode: DeclaredLintMode,
    pub(crate) suppressions: Vec<LintSuppression>,
}

impl Default for PolicyLintMetadata {
    fn default() -> Self {
        Self {
            sql: None,
            declared: Vec::new(),
            mode: DeclaredLintMode::Warn,
            suppressions: Vec::new(),
        }
    }
}

impl PolicyLintMetadata {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn sql(&self) -> Option<&str> {
        self.sql.as_deref()
    }

    pub fn declared(&self) -> &[DeclaredRelation] {
        &self.declared
    }

    pub fn mode(&self) -> DeclaredLintMode {
        self.mode
    }

    pub fn suppressions(&self) -> &[LintSuppression] {
        &self.suppressions
    }

    pub fn with_sql(mut self, sql: impl Into<String>) -> Self {
        self.sql = Some(sql.into());
        self
    }

    pub fn with_mode(mut self, mode: DeclaredLintMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_declared(mut self, relation: DeclaredRelation) -> Self {
        self.declared.push(relation);
        self
    }

    pub fn with_suppression(mut self, finding: LintFinding, reason: impl Into<String>) -> Self {
        self.suppressions
            .push(LintSuppression::new(finding, reason));
        self
    }
}

fn normalize_ident(identifier: impl Into<String>) -> String {
    identifier
        .into()
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
    fn table_metadata_normalizes_names() {
        let relation = schema_table("App", "\"Users\"");

        assert_eq!(relation.schema(), Some("app"));
        assert_eq!(relation.name(), "users");
    }
}
