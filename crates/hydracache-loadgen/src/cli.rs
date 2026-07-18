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
    TierNodeResp {
        profile: String,
        report: PathBuf,
    },
    TierControlPlane {
        profile: String,
        report: PathBuf,
        nodes: u8,
        target_roles: Vec<String>,
    },
    TierGridModel {
        profile: String,
        report: PathBuf,
    },
    SuiteCore {
        profile: String,
        output_dir: PathBuf,
    },
    SuiteResp {
        profile: String,
        output_dir: PathBuf,
    },
    SuiteControlPlane {
        profile: String,
        output_dir: PathBuf,
    },
}

impl LoadgenCommand {
    /// The W1 local artifact path; both public command forms route to this exact file.
    pub fn local_report_path(&self) -> Option<PathBuf> {
        match self {
            Self::TierLocal { report, .. } => Some(report.clone()),
            Self::SuiteCore { output_dir, .. } => Some(output_dir.join("local.json")),
            Self::TierClientSurface { .. }
            | Self::TierNodeResp { .. }
            | Self::TierControlPlane { .. }
            | Self::TierGridModel { .. }
            | Self::SuiteResp { .. }
            | Self::SuiteControlPlane { .. } => None,
        }
    }

    /// The W2 client-surface artifact path. A direct W2 command and the
    /// aggregate core suite resolve to the same canonical file name.
    pub fn client_surface_report_path(&self) -> Option<PathBuf> {
        match self {
            Self::TierClientSurface { report, .. } => Some(report.clone()),
            Self::SuiteCore { output_dir, .. } => Some(output_dir.join("client-surface.json")),
            Self::TierLocal { .. }
            | Self::TierNodeResp { .. }
            | Self::TierControlPlane { .. }
            | Self::TierGridModel { .. }
            | Self::SuiteResp { .. }
            | Self::SuiteControlPlane { .. } => None,
        }
    }

    pub fn resp_open_loop_report_path(&self) -> Option<PathBuf> {
        match self {
            Self::TierNodeResp { report, .. } => Some(report.clone()),
            Self::SuiteResp { output_dir, .. } => Some(output_dir.join("node-resp-open-loop.json")),
            Self::TierLocal { .. }
            | Self::TierClientSurface { .. }
            | Self::TierControlPlane { .. }
            | Self::TierGridModel { .. }
            | Self::SuiteCore { .. }
            | Self::SuiteControlPlane { .. } => None,
        }
    }

    pub fn resp_external_report_path(&self) -> Option<PathBuf> {
        match self {
            Self::SuiteResp { output_dir, .. } => {
                Some(output_dir.join("node-resp-redis-benchmark.json"))
            }
            Self::TierLocal { .. }
            | Self::TierClientSurface { .. }
            | Self::TierNodeResp { .. }
            | Self::TierControlPlane { .. }
            | Self::TierGridModel { .. }
            | Self::SuiteCore { .. }
            | Self::SuiteControlPlane { .. } => None,
        }
    }

    pub fn grid_model_report_path(&self) -> Option<PathBuf> {
        match self {
            Self::TierGridModel { report, .. } => Some(report.clone()),
            Self::SuiteCore { output_dir, .. } => Some(output_dir.join("grid-model.json")),
            _ => None,
        }
    }

    pub fn control_plane_report_path(&self) -> Option<PathBuf> {
        match self {
            Self::TierControlPlane { report, .. } => Some(report.clone()),
            _ => None,
        }
    }

    /// Canonical W4A output set for the aggregate control-plane evidence lane.
    pub fn control_plane_suite_report_paths(&self) -> Option<Vec<(u8, PathBuf)>> {
        match self {
            Self::SuiteControlPlane { output_dir, .. } => Some(
                [3_u8, 5, 7]
                    .into_iter()
                    .map(|nodes| {
                        (
                            nodes,
                            output_dir.join(format!("control-plane-{nodes}.json")),
                        )
                    })
                    .collect(),
            ),
            _ => None,
        }
    }

    pub fn control_plane_shape(&self) -> Option<(u8, &[String])> {
        match self {
            Self::TierControlPlane {
                nodes,
                target_roles,
                ..
            } => Some((*nodes, target_roles)),
            _ => None,
        }
    }

    pub fn profile(&self) -> &str {
        match self {
            Self::TierLocal { profile, .. }
            | Self::TierClientSurface { profile, .. }
            | Self::TierNodeResp { profile, .. }
            | Self::TierControlPlane { profile, .. }
            | Self::TierGridModel { profile, .. }
            | Self::SuiteCore { profile, .. }
            | Self::SuiteResp { profile, .. }
            | Self::SuiteControlPlane { profile, .. } => profile,
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
    let mut nodes = None;
    let mut target_roles = None;
    while let Some(flag) = arguments.pop_front() {
        let value = arguments
            .pop_front()
            .ok_or_else(|| format!("{flag} requires a value"))?;
        let duplicate = match flag.as_str() {
            "--profile" => profile.replace(value).is_some(),
            "--report" => report.replace(PathBuf::from(value)).is_some(),
            "--output-dir" => output_dir.replace(PathBuf::from(value)).is_some(),
            "--nodes" => nodes.replace(value).is_some(),
            "--target-roles" => target_roles.replace(value).is_some(),
            _ => return Err(format!("unknown option: {flag}")),
        };
        if duplicate {
            return Err(format!("duplicate option: {flag}"));
        }
    }
    let profile = profile.ok_or_else(|| "--profile is required".to_owned())?;
    if !(family == "tier" && name == "control-plane") && (nodes.is_some() || target_roles.is_some())
    {
        return Err("--nodes and --target-roles are valid only for tier control-plane".to_owned());
    }
    match (family.as_str(), name.as_str(), report, output_dir) {
        ("tier", "local", Some(report), None) => Ok(LoadgenCommand::TierLocal { profile, report }),
        ("tier", "client-surface", Some(report), None) => {
            Ok(LoadgenCommand::TierClientSurface { profile, report })
        }
        ("tier", "node-resp", Some(report), None) => {
            Ok(LoadgenCommand::TierNodeResp { profile, report })
        }
        ("tier", "control-plane", Some(report), None) => {
            let nodes = parse_control_plane_nodes(nodes)?;
            let target_roles = parse_control_plane_roles(target_roles)?;
            Ok(LoadgenCommand::TierControlPlane {
                profile,
                report,
                nodes,
                target_roles,
            })
        }
        ("tier", "grid-model", Some(report), None) => {
            Ok(LoadgenCommand::TierGridModel { profile, report })
        }
        ("suite", "core", None, Some(output_dir)) => Ok(LoadgenCommand::SuiteCore {
            profile,
            output_dir,
        }),
        ("suite", "resp", None, Some(output_dir)) => Ok(LoadgenCommand::SuiteResp {
            profile,
            output_dir,
        }),
        ("suite", "control-plane", None, Some(output_dir)) => {
            Ok(LoadgenCommand::SuiteControlPlane {
                profile,
                output_dir,
            })
        }
        ("tier", "local", _, _) => {
            Err("tier local requires --report and forbids --output-dir".to_owned())
        }
        ("tier", "client-surface", _, _) => {
            Err("tier client-surface requires --report and forbids --output-dir".to_owned())
        }
        ("tier", "node-resp", _, _) => {
            Err("tier node-resp requires --report and forbids --output-dir".to_owned())
        }
        ("tier", "control-plane", _, _) => Err(
            "tier control-plane requires --report, --nodes, and --target-roles and forbids --output-dir"
                .to_owned(),
        ),
        ("tier", "grid-model", _, _) => {
            Err("tier grid-model requires --report and forbids --output-dir".to_owned())
        }
        ("suite", "core", _, _) => {
            Err("suite core requires --output-dir and forbids --report".to_owned())
        }
        ("suite", "resp", _, _) => {
            Err("suite resp requires --output-dir and forbids --report".to_owned())
        }
        ("suite", "control-plane", _, _) => {
            Err("suite control-plane requires --output-dir and forbids --report".to_owned())
        }
        _ => Err(format!("unsupported command: {family} {name}")),
    }
}

fn parse_control_plane_nodes(value: Option<String>) -> Result<u8, String> {
    let raw = value.ok_or_else(|| "tier control-plane requires --nodes".to_owned())?;
    let nodes = raw
        .parse::<u8>()
        .map_err(|_| "--nodes must be one of 3, 5, or 7".to_owned())?;
    if ![3, 5, 7].contains(&nodes) {
        return Err("--nodes must be one of 3, 5, or 7".to_owned());
    }
    Ok(nodes)
}

fn parse_control_plane_roles(value: Option<String>) -> Result<Vec<String>, String> {
    let raw = value.ok_or_else(|| "tier control-plane requires --target-roles".to_owned())?;
    let roles = raw.split(',').map(str::to_owned).collect::<Vec<_>>();
    if roles != ["leader", "follower"] {
        return Err("--target-roles must be exactly leader,follower".to_owned());
    }
    Ok(roles)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn w4_direct_commands_preserve_exact_surface_shapes() {
        let control = parse(args(&[
            "tier",
            "control-plane",
            "--nodes",
            "5",
            "--target-roles",
            "leader,follower",
            "--profile",
            "reference-v1",
            "--report",
            "target/test-evidence/0.67/control-plane-5.json",
        ]))
        .unwrap();
        assert_eq!(control.control_plane_shape().unwrap().0, 5);
        assert_eq!(
            control.control_plane_shape().unwrap().1,
            ["leader", "follower"]
        );
        assert_eq!(
            control.control_plane_report_path().unwrap(),
            Path::new("target/test-evidence/0.67/control-plane-5.json")
        );

        let grid = parse(args(&[
            "tier",
            "grid-model",
            "--profile",
            "reference-v1",
            "--report",
            "target/test-evidence/0.67/grid-model.json",
        ]))
        .unwrap();
        assert_eq!(
            grid.grid_model_report_path().unwrap(),
            Path::new("target/test-evidence/0.67/grid-model.json")
        );
    }

    #[test]
    fn w4_control_plane_cli_rejects_partial_or_relabelled_shapes() {
        for nodes in ["1", "4", "8"] {
            assert!(parse(args(&[
                "tier",
                "control-plane",
                "--nodes",
                nodes,
                "--target-roles",
                "leader,follower",
                "--profile",
                "reference-v1",
                "--report",
                "report.json",
            ]))
            .is_err());
        }
        for roles in [
            "leader",
            "follower",
            "follower,leader",
            "leader,follower,kind",
        ] {
            assert!(parse(args(&[
                "tier",
                "control-plane",
                "--nodes",
                "3",
                "--target-roles",
                roles,
                "--profile",
                "reference-v1",
                "--report",
                "report.json",
            ]))
            .is_err());
        }
    }

    #[test]
    fn w4_control_plane_suite_has_exact_three_canonical_reports() {
        let suite = parse(args(&[
            "suite",
            "control-plane",
            "--profile",
            "reference-v1",
            "--output-dir",
            "target/test-evidence/0.67",
        ]))
        .unwrap();
        assert_eq!(
            suite.control_plane_suite_report_paths().unwrap(),
            vec![
                (
                    3,
                    PathBuf::from("target/test-evidence/0.67/control-plane-3.json")
                ),
                (
                    5,
                    PathBuf::from("target/test-evidence/0.67/control-plane-5.json")
                ),
                (
                    7,
                    PathBuf::from("target/test-evidence/0.67/control-plane-7.json")
                ),
            ]
        );
    }
}
