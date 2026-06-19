use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DimensionProfile {
    TenantScoped,
    PermissionScoped,
    TenantPermissionScoped,
    TenantPermissionSearch,
    PagedSearch,
    CursorList,
    LocaleRegionScoped,
    FeatureFlagScoped,
    Custom(CustomProfile),
}

impl DimensionProfile {
    pub fn name(&self) -> &str {
        match self {
            Self::TenantScoped => "tenant_scoped",
            Self::PermissionScoped => "permission_scoped",
            Self::TenantPermissionScoped => "tenant_permission_scoped",
            Self::TenantPermissionSearch => "tenant_permission_search",
            Self::PagedSearch => "paged_search",
            Self::CursorList => "cursor_list",
            Self::LocaleRegionScoped => "locale_region_scoped",
            Self::FeatureFlagScoped => "feature_flag_scoped",
            Self::Custom(profile) => profile.name(),
        }
    }

    pub fn requirements(&self) -> Vec<DimensionRequirement> {
        match self {
            Self::TenantScoped => vec![DimensionRequirement::linked("tenant")],
            Self::PermissionScoped => vec![DimensionRequirement::linked("permission")],
            Self::TenantPermissionScoped => vec![
                DimensionRequirement::linked("tenant"),
                DimensionRequirement::linked("permission"),
            ],
            Self::TenantPermissionSearch => vec![
                DimensionRequirement::linked("tenant"),
                DimensionRequirement::linked("permission"),
                DimensionRequirement::linked("q"),
                DimensionRequirement::linked("page"),
                DimensionRequirement::linked("sort"),
            ],
            Self::PagedSearch => vec![
                DimensionRequirement::linked("q"),
                DimensionRequirement::linked("page"),
                DimensionRequirement::linked("sort"),
            ],
            Self::CursorList => vec![DimensionRequirement::linked("cursor")],
            Self::LocaleRegionScoped => vec![
                DimensionRequirement::linked("locale"),
                DimensionRequirement::linked("region"),
            ],
            Self::FeatureFlagScoped => vec![DimensionRequirement::linked("feature")],
            Self::Custom(profile) => profile.requirements().to_vec(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomProfile {
    name: String,
    requirements: Vec<DimensionRequirement>,
}

impl CustomProfile {
    pub fn new(
        name: impl Into<String>,
        requirements: impl IntoIterator<Item = DimensionRequirement>,
    ) -> Self {
        Self {
            name: name.into(),
            requirements: requirements.into_iter().collect(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn requirements(&self) -> &[DimensionRequirement] {
        &self.requirements
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DimensionRequirement {
    label: String,
    require_key_tag_link: bool,
}

impl DimensionRequirement {
    pub fn linked(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            require_key_tag_link: true,
        }
    }

    pub fn key_only(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            require_key_tag_link: false,
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn require_key_tag_link(&self) -> bool {
        self.require_key_tag_link
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DimensionValidationMode {
    Warn,
    Deny,
}

impl Default for DimensionValidationMode {
    fn default() -> Self {
        Self::Warn
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileValidation {
    Pass,
    MissingDimensions(Vec<String>),
    UnlinkedDimensions(Vec<String>),
    Allowed {
        status: Box<ProfileValidation>,
        reason: String,
    },
}

impl ProfileValidation {
    pub fn is_pass(&self) -> bool {
        matches!(self, Self::Pass | Self::Allowed { .. })
    }
}

impl fmt::Display for ProfileValidation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pass => formatter.write_str("pass"),
            Self::MissingDimensions(labels) => {
                write!(formatter, "missing dimensions: {}", labels.join(", "))
            }
            Self::UnlinkedDimensions(labels) => {
                write!(formatter, "unlinked dimensions: {}", labels.join(", "))
            }
            Self::Allowed { status, reason } => {
                write!(formatter, "allowed {status} because {reason}")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DimensionAllow {
    label: String,
    reason: String,
}

impl DimensionAllow {
    pub fn new(
        label: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<Self, DimensionAllowError> {
        let reason = reason.into();
        if reason.trim().is_empty() {
            return Err(DimensionAllowError::EmptyReason);
        }
        Ok(Self {
            label: label.into(),
            reason,
        })
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DimensionAllowError {
    EmptyReason,
}

impl fmt::Display for DimensionAllowError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyReason => formatter.write_str("dimension allow reason cannot be empty"),
        }
    }
}

impl std::error::Error for DimensionAllowError {}
