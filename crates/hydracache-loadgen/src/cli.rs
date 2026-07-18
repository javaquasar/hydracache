use std::collections::VecDeque;
use std::path::PathBuf;

/// Canonical command forms consumed by direct tier runs and aggregate evidence lanes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadgenCommand {
    TierLocal {
        profile: String,
        report: PathBuf,
    },
    TierClientSurface {
        profile: String,
        report: PathBuf,
    },
    SuiteCore {
        profile: String,
        output_dir: PathBuf,
    },
}

impl LoadgenCommand {
    /// The W1 local artifact path; both public command forms route to this exact file.
    pub fn local_report_path(&self) -> PathBuf {
        match self {
            Self::TierLocal { report, .. } => report.clone(),
            Self::TierClientSurface { report, .. } => report.clone(),
            Self::SuiteCore { output_dir, .. } => output_dir.join("local.json"),
        }
    }

    /// The W2 client-surface artifact path. A direct W2 command and the
    /// aggregate core suite resolve to the same canonical file name.
    pub fn client_surface_report_path(&self) -> PathBuf {
        match self {
            Self::TierClientSurface { report, .. } => report.clone(),
            Self::SuiteCore { output_dir, .. } => output_dir.join("client-surface.json"),
            Self::TierLocal { report, .. } => report.clone(),
        }
    }

    pub fn profile(&self) -> &str {
        match self {
            Self::TierLocal { profile, .. }
            | Self::TierClientSurface { profile, .. }
            | Self::SuiteCore { profile, .. } => profile,
        }
    }
}

/// Parse the intentionally small release-0.67 CLI without adding a product dependency.
pub fn parse(arguments: impl IntoIterator<Item = String>) -> Result<LoadgenCommand, String> {
    let mut arguments = arguments.into_iter().collect::<VecDeque<_>>();
    let family = arguments
        .pop_front()
        .ok_or_else(|| "missing command family (tier or suite)".to_owned())?;
    let name = arguments
        .pop_front()
        .ok_or_else(|| format!("missing {family} name"))?;
    let mut profile = None;
    let mut report = None;
    let mut output_dir = None;
    while let Some(flag) = arguments.pop_front() {
        let value = arguments
            .pop_front()
            .ok_or_else(|| format!("{flag} requires a value"))?;
        let duplicate = match flag.as_str() {
            "--profile" => profile.replace(value).is_some(),
            "--report" => report.replace(PathBuf::from(value)).is_some(),
            "--output-dir" => output_dir.replace(PathBuf::from(value)).is_some(),
            _ => return Err(format!("unknown option: {flag}")),
        };
        if duplicate {
            return Err(format!("duplicate option: {flag}"));
        }
    }
    let profile = profile.ok_or_else(|| "--profile is required".to_owned())?;
    match (family.as_str(), name.as_str(), report, output_dir) {
        ("tier", "local", Some(report), None) => Ok(LoadgenCommand::TierLocal { profile, report }),
        ("tier", "client-surface", Some(report), None) => {
            Ok(LoadgenCommand::TierClientSurface { profile, report })
        }
        ("suite", "core", None, Some(output_dir)) => Ok(LoadgenCommand::SuiteCore {
            profile,
            output_dir,
        }),
        ("tier", "local", _, _) => {
            Err("tier local requires --report and forbids --output-dir".to_owned())
        }
        ("tier", "client-surface", _, _) => {
            Err("tier client-surface requires --report and forbids --output-dir".to_owned())
        }
        ("suite", "core", _, _) => {
            Err("suite core requires --output-dir and forbids --report".to_owned())
        }
        _ => Err(format!("unsupported command: {family} {name}")),
    }
}
