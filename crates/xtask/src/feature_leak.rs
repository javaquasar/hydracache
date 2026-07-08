//! Release-graph guard for test-only dependencies and features.

use std::error::Error;
use std::path::Path;
use std::process::Command;

const RELEASE_PACKAGES: &[&str] = &[
    "hydracache-server",
    "hydracache-operator",
    "hydracache-cluster-raft",
];

const FORBIDDEN_MARKERS: &[ForbiddenMarker] = &[
    ForbiddenMarker {
        marker: "fail v",
        reason: "`fail` crate is only allowed behind `test-failpoints`",
    },
    ForbiddenMarker {
        marker: "feature \"test-failpoints\"",
        reason: "`test-failpoints` must not be enabled in the default release graph",
    },
    ForbiddenMarker {
        marker: "feature \"test-support\"",
        reason: "`test-support` must not be enabled in the default release graph",
    },
    ForbiddenMarker {
        marker: "hydracache-cluster-testkit v",
        reason: "`hydracache-cluster-testkit` is a dev-only harness",
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ForbiddenMarker {
    marker: &'static str,
    reason: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeatureLeak {
    pub package: String,
    pub marker: &'static str,
    pub reason: &'static str,
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if !args.is_empty() {
        return Err("verify-no-test-features does not accept arguments".into());
    }

    let root = crate::doc_check::find_repo_root()?;
    let leaks = check(&root)?;
    if !leaks.is_empty() {
        for leak in &leaks {
            eprintln!(
                "release feature leak in {}: {} ({})",
                leak.package, leak.marker, leak.reason
            );
        }
        return Err(format!("found {} release feature leak(s)", leaks.len()).into());
    }

    println!("verify-no-test-features: OK");
    Ok(())
}

pub fn check(root: &Path) -> Result<Vec<FeatureLeak>, Box<dyn Error>> {
    let mut leaks = Vec::new();
    for package in RELEASE_PACKAGES {
        let tree = cargo_tree(root, package)?;
        leaks.extend(detect_leaks(package, &tree));
    }
    Ok(leaks)
}

fn cargo_tree(root: &Path, package: &str) -> Result<String, Box<dyn Error>> {
    let output = Command::new("cargo")
        .args([
            "tree",
            "-p",
            package,
            "--edges",
            "normal,build,features",
            "--locked",
            "--prefix",
            "none",
            "--format",
            "{p} {f}",
        ])
        .current_dir(root)
        .output()
        .map_err(|err| format!("could not run cargo tree for {package}: {err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("cargo tree for {package} failed: {stderr}").into());
    }

    Ok(String::from_utf8(output.stdout)?)
}

fn detect_leaks(package: &str, tree: &str) -> Vec<FeatureLeak> {
    FORBIDDEN_MARKERS
        .iter()
        .filter(|marker| tree.contains(marker.marker))
        .map(|marker| FeatureLeak {
            package: package.to_owned(),
            marker: marker.marker,
            reason: marker.reason,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::detect_leaks;

    #[test]
    fn clean_release_tree_has_no_leaks() {
        let tree = r#"
hydracache-server v0.62.0 default
hydracache-cluster-raft v0.62.0 durable-log
"#;

        assert!(detect_leaks("hydracache-server", tree).is_empty());
    }

    #[test]
    fn detects_failpoint_feature_and_dependency_leaks() {
        let tree = r#"
hydracache-cluster-raft feature "test-failpoints"
fail v0.5.1 default
"#;

        let leaks = detect_leaks("hydracache-cluster-raft", tree);

        assert_eq!(leaks.len(), 2);
        assert!(leaks
            .iter()
            .any(|leak| leak.marker == "feature \"test-failpoints\""));
        assert!(leaks.iter().any(|leak| leak.marker == "fail v"));
    }

    #[test]
    fn detects_testkit_and_test_support_leaks() {
        let tree = r#"
hydracache-server feature "test-support"
hydracache-cluster-testkit v0.62.0
"#;

        let leaks = detect_leaks("hydracache-server", tree);

        assert_eq!(leaks.len(), 2);
        assert!(leaks
            .iter()
            .any(|leak| leak.marker == "feature \"test-support\""));
        assert!(leaks
            .iter()
            .any(|leak| leak.marker == "hydracache-cluster-testkit v"));
    }
}
