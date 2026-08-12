use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

use crate::{client_plane_java, client_plane_python, client_plane_rust};

const MANIFEST: &str = "docs/testing/hc2-client-conformance-v1.json";
const SCHEMA: &str = "hydracache.hc2.client-conformance.v1";
const GENERATION: u32 = 6;
const REQUIRED_SDKS: [&str; 3] = ["java", "python", "rust"];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: String,
    protocol_generation: u32,
    required_sdks: Vec<String>,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    id: String,
    semantics: String,
    proofs: BTreeMap<String, Proof>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Proof {
    source: String,
    function: String,
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if args.as_slice() != ["--all-sdks"] {
        return Err("client-conformance requires exactly --all-sdks".into());
    }
    check_at_root(&workspace_root()?, true)
}

pub(crate) fn check_at_root(root: &Path, all_sdks: bool) -> Result<(), Box<dyn Error>> {
    let scenario_count = validate_manifest_at_root(root)?;
    if all_sdks {
        client_plane_rust::check_at_root(root)?;
        client_plane_java::check_at_root(root)?;
        client_plane_python::check_at_root(root)?;
    }
    println!(
        "client-conformance: OK ({scenario_count} scenarios, generation {GENERATION}, all_sdks={all_sdks})"
    );
    Ok(())
}

pub fn validate_manifest_at_root(root: &Path) -> Result<usize, Box<dyn Error>> {
    let path = root.join(MANIFEST);
    let manifest: Manifest = serde_json::from_slice(&fs::read(&path)?)
        .map_err(|error| format!("parsing {}: {error}", path.display()))?;
    if manifest.schema_version != SCHEMA {
        return Err(format!(
            "{} schema_version must be {SCHEMA}, got {}",
            path.display(),
            manifest.schema_version
        )
        .into());
    }
    if manifest.protocol_generation != GENERATION {
        return Err(format!(
            "{} protocol_generation must be {GENERATION}, got {}",
            path.display(),
            manifest.protocol_generation
        )
        .into());
    }
    let required = manifest
        .required_sdks
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if required != REQUIRED_SDKS {
        return Err(format!(
            "{} required_sdks must be {:?} in stable order, got {:?}",
            path.display(),
            REQUIRED_SDKS,
            manifest.required_sdks
        )
        .into());
    }
    if manifest.scenarios.len() < 9 {
        return Err(format!(
            "{} must retain at least 9 cross-SDK scenarios, got {}",
            path.display(),
            manifest.scenarios.len()
        )
        .into());
    }

    let mut previous = None::<&str>;
    let mut ids = BTreeSet::new();
    for scenario in &manifest.scenarios {
        if scenario.id.trim().is_empty() || scenario.semantics.trim().is_empty() {
            return Err("HC/2 conformance scenario id and semantics must be non-empty".into());
        }
        if previous.is_some_and(|value| value >= scenario.id.as_str()) {
            return Err(format!(
                "HC/2 conformance scenarios must be strictly sorted; {:?} precedes {}",
                previous, scenario.id
            )
            .into());
        }
        previous = Some(&scenario.id);
        if !ids.insert(&scenario.id) {
            return Err(format!("duplicate HC/2 conformance scenario {}", scenario.id).into());
        }
        let proof_sdks = scenario
            .proofs
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>();
        if proof_sdks != REQUIRED_SDKS {
            return Err(format!(
                "scenario {} proofs must cover {:?}, got {:?}",
                scenario.id, REQUIRED_SDKS, proof_sdks
            )
            .into());
        }
        for (sdk, proof) in &scenario.proofs {
            validate_proof(root, &scenario.id, sdk, proof)?;
        }
    }
    Ok(manifest.scenarios.len())
}

fn validate_proof(
    root: &Path,
    scenario: &str,
    sdk: &str,
    proof: &Proof,
) -> Result<(), Box<dyn Error>> {
    let relative = Path::new(&proof.source);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
    {
        return Err(format!(
            "scenario {scenario} {sdk} proof source must be a safe workspace-relative path: {}",
            proof.source
        )
        .into());
    }
    if proof.function.trim().is_empty() {
        return Err(format!("scenario {scenario} {sdk} proof function is empty").into());
    }
    let source_path = root.join(relative);
    let source = fs::read_to_string(&source_path).map_err(|error| {
        format!(
            "reading scenario {scenario} {sdk} proof {}: {error}",
            source_path.display()
        )
    })?;
    if !source.contains(&proof.function) {
        return Err(format!(
            "scenario {scenario} {sdk} proof function {} is absent from {}",
            proof.function,
            source_path.display()
        )
        .into());
    }
    Ok(())
}

fn workspace_root() -> Result<PathBuf, Box<dyn Error>> {
    let mut candidate = std::env::current_dir()?;
    loop {
        if candidate.join("Cargo.toml").is_file() && candidate.join(MANIFEST).is_file() {
            return Ok(candidate);
        }
        if !candidate.pop() {
            return Err("could not locate HydraCache workspace root".into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cross_sdk_manifest_is_complete_and_source_bound() {
        assert_eq!(
            validate_manifest_at_root(&workspace_root().unwrap()).unwrap(),
            9
        );
    }
}
