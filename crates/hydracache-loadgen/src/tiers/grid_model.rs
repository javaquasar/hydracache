//! W4B orchestration for the explicitly in-process library/model tier.

use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::targets::grid_model::{
    run_grid_model_reference, run_grid_model_smoke, GridModelPrebuildAttestation,
    GridModelReferenceAttestation, GridModelReport, GridModelRunnerAttestation, GridModelScenario,
    GridModelSourceAttestation,
};

use super::resp::RespReferenceRunInputs;
use super::resp_reference::load_reference_context;

pub const GRID_MODEL_SCENARIO_PATH: &str =
    "docs/testing/perf-scenarios/0.67/grid-model-primitives-v1.toml";

#[derive(Debug, Error)]
pub enum GridModelTierError {
    #[error("W4B repository root is unavailable: {0}")]
    Repository(String),
    #[error("W4B reference context is unavailable: {0}")]
    Reference(String),
    #[error(transparent)]
    Model(#[from] crate::targets::grid_model::GridModelError),
    #[error("W4B report IO failed for {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("W4B report serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Produce an explicitly unclaimed fast report, or the exact receipt-bound
/// reference report. No profile string can relabel the reduced smoke shape.
pub async fn write_grid_model_report(
    profile_name: &str,
    report_path: &Path,
) -> Result<GridModelReport, GridModelTierError> {
    let repo_root = repository_root()?;
    let committed = GridModelScenario::load(&repo_root.join(GRID_MODEL_SCENARIO_PATH))?;
    let report = match profile_name {
        "reference-v1" => run_reference(&repo_root, &committed).await?,
        "smoke-v1" | "ci-shared" => {
            let smoke = reduced_smoke_scenario(committed);
            run_grid_model_smoke(&smoke).await?
        }
        other => {
            return Err(GridModelTierError::Reference(format!(
                "unsupported W4B profile {other:?}; expected reference-v1, ci-shared, or smoke-v1"
            )))
        }
    };
    write_report(report_path, &report)?;
    Ok(report)
}

async fn run_reference(
    repo_root: &Path,
    scenario: &GridModelScenario,
) -> Result<GridModelReport, GridModelTierError> {
    let inputs = RespReferenceRunInputs::load(repo_root)
        .map_err(|error| GridModelTierError::Reference(error.to_string()))?;
    let context = load_reference_context(repo_root, Some(&inputs.prerequisites))
        .map_err(|error| GridModelTierError::Reference(error.to_string()))?;
    context
        .verify_binaries_unchanged()
        .map_err(|error| GridModelTierError::Reference(error.to_string()))?;

    let source = GridModelSourceAttestation::from_verified_w7(
        context.source.git_commit.clone(),
        context.source.cargo_lock_sha256.clone(),
    )?;
    let runner = GridModelRunnerAttestation::from_observed_w7(&context.runner)?;
    let prebuild = GridModelPrebuildAttestation::from_verified_manifest(
        scenario,
        source.clone(),
        &runner,
        context.manifest_path.clone(),
        context.manifest_sha256.clone(),
    )?;
    let attestation =
        GridModelReferenceAttestation::from_verified_parts(scenario, source, runner, prebuild)?;
    let report = run_grid_model_reference(scenario, attestation).await?;
    context
        .verify_binaries_unchanged()
        .map_err(|error| GridModelTierError::Reference(error.to_string()))?;
    report.validate(scenario)?;
    Ok(report)
}

fn reduced_smoke_scenario(mut scenario: GridModelScenario) -> GridModelScenario {
    scenario.dimensions.iterations = 128;
    scenario.dimensions.replica_shapes = vec![1, 3];
    scenario.dimensions.region_shapes = vec![1, 2];
    scenario.dimensions.replication_peer_shapes = vec![1, 2];
    scenario.dimensions.payload_bytes = vec![64, 1_024];
    scenario.dimensions.invalidation_subscribers = vec![1, 3];
    scenario.dimensions.watermark_entries = 8;
    scenario.measurement.warmup_iterations = 32;
    scenario.measurement.raw_repeats = 3;
    scenario.measurement.maximum_robust_spread_ratio_millionths = 1_000_000;
    scenario
}

fn repository_root() -> Result<PathBuf, GridModelTierError> {
    std::env::current_dir()
        .map_err(|error| GridModelTierError::Repository(error.to_string()))?
        .canonicalize()
        .map_err(|error| GridModelTierError::Repository(error.to_string()))
}

fn write_report(path: &Path, report: &GridModelReport) -> Result<(), GridModelTierError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| GridModelTierError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let mut bytes = serde_json::to_vec_pretty(report)?;
    bytes.push(b'\n');
    fs::write(path, bytes).map_err(|source| GridModelTierError::Io {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reduced_smoke_shape_cannot_become_reference() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let committed = GridModelScenario::load(&root.join(GRID_MODEL_SCENARIO_PATH)).unwrap();
        let reduced = reduced_smoke_scenario(committed);
        assert!(reduced.validate().is_ok());
        assert!(reduced.validate_exact_reference_shape().is_err());
    }
}
