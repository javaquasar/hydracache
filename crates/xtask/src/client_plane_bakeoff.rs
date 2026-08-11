use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

const MANIFEST: &str = "docs/testing/hc2-transport-bakeoff.json";
const REQUIRED_EVIDENCE: &[&str] = &[
    "tls_identity",
    "correlation_256",
    "interleaved_push",
    "slow_consumer_gap",
    "cancellation",
    "half_close",
    "reset",
    "disconnect_zero",
    "hostile_corpus",
    "generation_isolation",
    "language_fixture",
];
const REQUIRED_CANDIDATES: &[&str] = &["dedicated-tcp", "http2", "grpc"];

#[derive(Deserialize)]
struct Bakeoff {
    schema: u32,
    protocol_generation: u16,
    selected_primary: String,
    measurement: Measurement,
    candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
struct Measurement {
    captured_at: String,
    platform: String,
    dependency_command: String,
    unique_dependency_lines: u64,
    binary_command: String,
    generated_rust_bytes: u64,
    generated_java_files: u64,
    generated_java_bytes: u64,
}

#[derive(Deserialize)]
struct Candidate {
    id: String,
    disposition: String,
    debug_test_binary_bytes: u64,
    transport_runtime: String,
    cross_language_cost: String,
    operational_cost: String,
    evidence: BTreeMap<String, bool>,
    evidence_paths: Vec<String>,
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if !args.is_empty() {
        return Err("client-plane-bakeoff-check does not accept arguments".into());
    }
    check_at_root(&workspace_root()?)
}

pub fn check_at_root(root: &Path) -> Result<(), Box<dyn Error>> {
    let manifest: Bakeoff = serde_json::from_slice(&fs::read(root.join(MANIFEST))?)?;
    if manifest.schema != 1 || manifest.protocol_generation != 5 {
        return Err("HC/2 bake-off schema or protocol generation is not the accepted value".into());
    }
    validate_measurement(&manifest.measurement)?;

    let ids = manifest
        .candidates
        .iter()
        .map(|candidate| candidate.id.as_str())
        .collect::<BTreeSet<_>>();
    let required = REQUIRED_CANDIDATES.iter().copied().collect::<BTreeSet<_>>();
    if ids != required || manifest.candidates.len() != REQUIRED_CANDIDATES.len() {
        return Err(format!("candidate set must be exactly {required:?}; found {ids:?}").into());
    }

    let primary = manifest
        .candidates
        .iter()
        .filter(|candidate| candidate.disposition == "production-primary")
        .collect::<Vec<_>>();
    if primary.len() != 1 || primary[0].id != manifest.selected_primary {
        return Err("exactly one production-primary candidate must match selected_primary".into());
    }

    let required_evidence = REQUIRED_EVIDENCE.iter().copied().collect::<BTreeSet<_>>();
    for candidate in &manifest.candidates {
        let evidence = candidate
            .evidence
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if evidence != required_evidence || candidate.evidence.values().any(|green| !green) {
            return Err(format!(
                "candidate {} does not have an exact all-green boolean evidence set",
                candidate.id
            )
            .into());
        }
        if candidate.debug_test_binary_bytes == 0
            || candidate.transport_runtime.trim().is_empty()
            || candidate.cross_language_cost.trim().is_empty()
            || candidate.operational_cost.trim().is_empty()
            || candidate.evidence_paths.is_empty()
        {
            return Err(format!(
                "candidate {} has incomplete cost/evidence metadata",
                candidate.id
            )
            .into());
        }
        for evidence_path in &candidate.evidence_paths {
            if Path::new(evidence_path).is_absolute() || !root.join(evidence_path).is_file() {
                return Err(format!(
                    "candidate {} evidence path is missing or not workspace-relative: {evidence_path}",
                    candidate.id
                )
                .into());
            }
        }
    }

    println!(
        "client-plane-bakeoff-check: OK (generation {}, primary {}, {} all-green candidates)",
        manifest.protocol_generation,
        manifest.selected_primary,
        manifest.candidates.len()
    );
    Ok(())
}

fn validate_measurement(measurement: &Measurement) -> Result<(), Box<dyn Error>> {
    if measurement.captured_at.trim().is_empty()
        || measurement.platform.trim().is_empty()
        || measurement.dependency_command.trim().is_empty()
        || measurement.binary_command.trim().is_empty()
        || measurement.unique_dependency_lines == 0
        || measurement.generated_rust_bytes == 0
        || measurement.generated_java_files == 0
        || measurement.generated_java_bytes == 0
    {
        return Err("bake-off measurement metadata is incomplete".into());
    }
    Ok(())
}

fn workspace_root() -> Result<PathBuf, Box<dyn Error>> {
    let mut candidate = std::env::current_dir()?;
    loop {
        if candidate.join(MANIFEST).is_file() && candidate.join("Cargo.toml").is_file() {
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
    fn accepted_bakeoff_manifest_is_complete() {
        check_at_root(&workspace_root().unwrap()).unwrap();
    }
}
