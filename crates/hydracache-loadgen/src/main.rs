use std::path::{Path, PathBuf};

use hydracache_loadgen::budget_receipt::{
    build_control_plane_brownout_macro_envelope, build_control_plane_macro_envelope,
    build_grid_model_brownout_macro_envelope, build_grid_model_macro_envelope,
    build_overload_macro_envelope, build_resp_brownout_macro_envelope, prepare_macro_artifact,
    publish_macro_batch, PreparedMacroArtifact,
};
use hydracache_loadgen::cli;
use hydracache_loadgen::cli::{BrownoutTarget, LoadgenCommand, OverloadTarget};
use hydracache_loadgen::compare_redis::{
    run_and_write_same_box_redis_comparison, RedisComparisonOutcome, RedisComparisonRunMode,
    W3ReferenceArtifactSet,
};
use hydracache_loadgen::overload::{
    write_reference_overload_report, EligibleOverloadSurface, OverloadReport, OverloadScenario,
    ReferencePredecessorRequest,
};
use hydracache_loadgen::resp_external::{
    ExternalToolProvenanceRegistry, RedisBenchmarkContract,
    REDIS_BENCHMARK_PROVENANCE_REGISTRY_PATH,
};
use hydracache_loadgen::targets::brownout::{
    ControlPlaneBrownoutReport, ControlPlaneBrownoutScenario, GridModelBrownoutReport,
    GridModelBrownoutScenario, RespBrownoutReport, RespBrownoutScenario,
};
use hydracache_loadgen::targets::control_plane::{ControlPlaneReport, ControlPlaneScenario};
use hydracache_loadgen::targets::grid_model::{GridModelReport, GridModelScenario};
use hydracache_loadgen::tiers::brownout::{
    produce_control_plane_reference as produce_control_plane_brownout,
    produce_grid_model_reference as produce_grid_model_brownout,
    produce_resp_reference as produce_resp_brownout,
};
use hydracache_loadgen::tiers::client_surface::{
    write_client_surface_report, write_client_surface_report_with_context,
};
use hydracache_loadgen::tiers::control_plane::run_control_plane_reference;
use hydracache_loadgen::tiers::grid_model::write_grid_model_report;
use hydracache_loadgen::tiers::local::{write_local_report, write_local_report_with_context};
use hydracache_loadgen::tiers::resp::{
    resp_reference_report_on, run_resp_reference_suite, write_resp_report, RespReferenceRunInputs,
    RespReferenceSuiteReceipt,
};
use hydracache_loadgen::tiers::resp_reference::{
    load_reference_context, start_reference_daemon, RespDaemonLaunch, RespReferencePorts,
};
use sha2::{Digest, Sha256};

const RESP_EXTERNAL_SCENARIO: &str =
    "docs/testing/perf-scenarios/0.67/resp-external-redis-benchmark-v1.toml";
const CONTROL_PLANE_SCENARIO: &str =
    "docs/testing/perf-scenarios/0.67/control-plane-real-daemon-v1.toml";
const GRID_MODEL_SCENARIO: &str = "docs/testing/perf-scenarios/0.67/grid-model-primitives-v1.toml";
const BROWNOUT_CONTROL_PLANE_SCENARIO: &str =
    "docs/testing/perf-scenarios/0.67/brownout-control-plane-v1.toml";
const BROWNOUT_RESP_SCENARIO: &str =
    "docs/testing/perf-scenarios/0.67/brownout-resp-endpoint-v1.toml";
const BROWNOUT_GRID_MODEL_SCENARIO: &str =
    "docs/testing/perf-scenarios/0.67/brownout-grid-model-v1.toml";
const OVERLOAD_SCENARIO: &str = "docs/testing/perf-scenarios/0.67/overload-capacity-v1.toml";

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("hydracache-loadgen: {error}");
        std::process::exit(2);
    }
}

async fn run() -> Result<(), String> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments.is_empty()
        || arguments
            .iter()
            .any(|argument| argument == "--help" || argument == "-h")
    {
        print_help();
        return Ok(());
    }
    let command = cli::parse(arguments)?;
    match &command {
        LoadgenCommand::TierLocal { .. } => {
            let path = command
                .local_report_path()
                .ok_or_else(|| "local command lost its canonical report path".to_owned())?;
            if command.profile() == "reference-v1" {
                let repo_root = repository_root()?;
                let context = load_reference_context_for_run(&repo_root)?;
                write_local_report_with_context(command.profile(), &path, Some(&context))
                    .await
                    .map_err(|error| error.to_string())?;
                eprintln!(
                    "hydracache-loadgen: wrote receipt-bound local reference report to {}",
                    path.display()
                );
            } else {
                write_local_report(command.profile(), &path)
                    .await
                    .map_err(|error| error.to_string())?;
                eprintln!(
                    "hydracache-loadgen: wrote plumbing-only local smoke report to {}",
                    path.display()
                );
            }
        }
        LoadgenCommand::TierClientSurface { .. } => {
            let path = command.client_surface_report_path().ok_or_else(|| {
                "client-surface command lost its canonical report path".to_owned()
            })?;
            if command.profile() == "reference-v1" {
                let repo_root = repository_root()?;
                let context = load_reference_context_for_run(&repo_root)?;
                write_client_surface_report_with_context(command.profile(), &path, Some(&context))
                    .await
                    .map_err(|error| error.to_string())?;
            } else {
                write_client_surface_report(command.profile(), &path)
                    .await
                    .map_err(|error| error.to_string())?;
            }
        }
        LoadgenCommand::SuiteCore { .. } => {
            let repo_root = repository_root()?;
            let reference_context = if command.profile() == "reference-v1" {
                Some(load_reference_context_for_run(&repo_root)?)
            } else {
                None
            };
            let local_path = command
                .local_report_path()
                .ok_or_else(|| "core suite lost its local report path".to_owned())?;
            write_local_report_with_context(
                command.profile(),
                &local_path,
                reference_context.as_ref(),
            )
            .await
            .map_err(|error| error.to_string())?;
            let client_surface_path = command
                .client_surface_report_path()
                .ok_or_else(|| "core suite lost its client-surface report path".to_owned())?;
            write_client_surface_report_with_context(
                command.profile(),
                &client_surface_path,
                reference_context.as_ref(),
            )
            .await
            .map_err(|error| error.to_string())?;
            let grid_model_path = command
                .grid_model_report_path()
                .ok_or_else(|| "core suite lost its grid-model report path".to_owned())?;
            write_grid_model_report(command.profile(), &grid_model_path)
                .await
                .map_err(|error| error.to_string())?;
            if command.profile() == "reference-v1" {
                let brownout_path = grid_model_path.with_file_name("brownout-grid-model.json");
                write_reference_brownout(
                    command.profile(),
                    BrownoutTarget::GridModelReplica,
                    &brownout_path,
                )
                .await?;
                let overload_local_path = local_path.with_file_name("overload-local.json");
                write_reference_overload(
                    command.profile(),
                    OverloadTarget::Local,
                    &overload_local_path,
                )
                .await?;
                let overload_client_path =
                    client_surface_path.with_file_name("overload-client-surface.json");
                write_reference_overload(
                    command.profile(),
                    OverloadTarget::ClientSurface,
                    &overload_client_path,
                )
                .await?;
            }
            eprintln!(
                "hydracache-loadgen: wrote core suite reports to {}, {}, and {}",
                local_path.display(),
                client_surface_path.display(),
                grid_model_path.display()
            );
        }
        LoadgenCommand::TierNodeResp { .. } => {
            let path = command.resp_open_loop_report_path().ok_or_else(|| {
                "RESP command lost its canonical open-loop report path".to_owned()
            })?;
            if command.profile() == "reference-v1" {
                write_reference_resp_open_loop(&path).await?;
            } else {
                write_resp_report(command.profile(), &path)
                    .await
                    .map_err(|error| error.to_string())?;
            }
        }
        LoadgenCommand::CompareRedis { .. } => {
            let path = command.redis_comparison_report_path().ok_or_else(|| {
                "Redis comparison command lost its canonical report path".to_owned()
            })?;
            let run_mode = direct_redis_comparison_run_mode()?;
            write_reference_redis_comparison(command.profile(), &path, run_mode).await?;
        }
        LoadgenCommand::SuiteResp { .. } => {
            let path = command
                .resp_open_loop_report_path()
                .ok_or_else(|| "RESP suite lost its canonical open-loop report path".to_owned())?;
            if command.profile() == "reference-v1" {
                let external_path = command.resp_external_report_path().ok_or_else(|| {
                    "RESP suite lost its canonical external-tool report path".to_owned()
                })?;
                write_reference_resp_suite(&path, &external_path).await?;
                // W9 publishes its metrics-honesty artifact at this reserved point,
                // after the sealed W3 suite and before W8 consumes that archive.
                let comparison_path = command.redis_comparison_report_path().ok_or_else(|| {
                    "RESP suite lost its canonical Redis-comparison report path".to_owned()
                })?;
                write_reference_redis_comparison(
                    command.profile(),
                    &comparison_path,
                    RedisComparisonRunMode::MandatoryReference,
                )
                .await?;
                let brownout_path = path.with_file_name("brownout-resp-endpoint.json");
                write_reference_brownout(
                    command.profile(),
                    BrownoutTarget::RespEndpointKill,
                    &brownout_path,
                )
                .await?;
                let overload_path = path.with_file_name("overload-node-resp.json");
                write_reference_overload(
                    command.profile(),
                    OverloadTarget::NodeResp,
                    &overload_path,
                )
                .await?;
            } else {
                write_resp_report(command.profile(), &path)
                    .await
                    .map_err(|error| error.to_string())?;
                eprintln!(
                    "hydracache-loadgen: supplemental redis-benchmark and W8 comparison evidence skipped loudly for fixture smoke; they require the selected receipt-bound daemon endpoint"
                );
            }
        }
        LoadgenCommand::TierGridModel { .. } => {
            let path = command
                .grid_model_report_path()
                .ok_or_else(|| "grid-model command lost its canonical report path".to_owned())?;
            write_grid_model_report(command.profile(), &path)
                .await
                .map_err(|error| error.to_string())?;
        }
        LoadgenCommand::TierControlPlane { .. } => {
            let path = command
                .control_plane_report_path()
                .ok_or_else(|| "control-plane command lost its canonical report path".to_owned())?;
            let (nodes, roles) = command
                .control_plane_shape()
                .ok_or_else(|| "control-plane command lost its exact shape".to_owned())?;
            if roles != ["leader", "follower"] {
                return Err(
                    "control-plane target roles must remain exactly leader,follower".to_owned(),
                );
            }
            write_reference_control_plane_reports(command.profile(), &[(nodes, path)]).await?;
        }
        LoadgenCommand::SuiteControlPlane { .. } => {
            let paths = command.control_plane_suite_report_paths().ok_or_else(|| {
                "control-plane suite lost its canonical 3/5/7 report set".to_owned()
            })?;
            write_reference_control_plane_reports(command.profile(), &paths).await?;
            let predecessor = paths
                .iter()
                .find(|(nodes, _)| *nodes == 3)
                .map(|(_, path)| path)
                .ok_or_else(|| "control-plane suite lost its 3-node predecessor".to_owned())?;
            let brownout_path = predecessor.with_file_name("brownout-control-plane.json");
            write_reference_brownout(
                command.profile(),
                BrownoutTarget::ControlPlaneLeader,
                &brownout_path,
            )
            .await?;
            publish_w7_macro_tail(&repository_root()?).await?;
        }
        LoadgenCommand::Brownout { .. } => {
            let (target, report) = command
                .brownout_shape()
                .ok_or_else(|| "brownout command lost its exact surface shape".to_owned())?;
            write_reference_brownout(command.profile(), target, &report).await?;
        }
        LoadgenCommand::Overload { .. } => {
            let (target, report) = command
                .overload_shape()
                .ok_or_else(|| "overload command lost its exact capacity surface".to_owned())?;
            write_reference_overload(command.profile(), target, &report).await?;
        }
    }
    Ok(())
}

async fn publish_w7_macro_tail(repo_root: &Path) -> Result<(), String> {
    let context = load_reference_context_for_run(repo_root)?;
    let evidence_root = repo_root.join("target/test-evidence/0.67");
    let control_scenario = ControlPlaneScenario::load(&repo_root.join(CONTROL_PLANE_SCENARIO))
        .map_err(|error| error.to_string())?;
    let grid_scenario = GridModelScenario::load(&repo_root.join(GRID_MODEL_SCENARIO))
        .map_err(|error| error.to_string())?;
    let control_brownout_scenario =
        ControlPlaneBrownoutScenario::load(&repo_root.join(BROWNOUT_CONTROL_PLANE_SCENARIO))
            .map_err(|error| error.to_string())?;
    let resp_brownout_scenario =
        RespBrownoutScenario::load(&repo_root.join(BROWNOUT_RESP_SCENARIO))
            .map_err(|error| error.to_string())?;
    let grid_brownout_scenario =
        GridModelBrownoutScenario::load(&repo_root.join(BROWNOUT_GRID_MODEL_SCENARIO))
            .map_err(|error| error.to_string())?;
    let overload_scenario = OverloadScenario::load(&repo_root.join(OVERLOAD_SCENARIO))
        .map_err(|error| error.to_string())?;

    let mut prepared = Vec::<PreparedMacroArtifact>::new();
    for nodes in [3_u8, 5, 7] {
        let path = evidence_root.join(format!("control-plane-{nodes}.json"));
        let report: ControlPlaneReport = read_typed_report(&path)?;
        let envelope = build_control_plane_macro_envelope(&context, &control_scenario, report)
            .map_err(|error| error.to_string())?;
        prepared.push(
            prepare_macro_artifact(repo_root, &path, &envelope)
                .map_err(|error| error.to_string())?,
        );
    }

    let grid_path = evidence_root.join("grid-model.json");
    let grid_report: GridModelReport = read_typed_report(&grid_path)?;
    let grid_envelope = build_grid_model_macro_envelope(&context, &grid_scenario, grid_report)
        .map_err(|error| error.to_string())?;
    prepared.push(
        prepare_macro_artifact(repo_root, &grid_path, &grid_envelope)
            .map_err(|error| error.to_string())?,
    );

    let control_brownout_path = evidence_root.join("brownout-control-plane.json");
    let control_brownout_report: ControlPlaneBrownoutReport =
        read_typed_report(&control_brownout_path)?;
    let control_brownout_envelope = build_control_plane_brownout_macro_envelope(
        &context,
        &control_brownout_scenario,
        control_brownout_report,
    )
    .map_err(|error| error.to_string())?;
    prepared.push(
        prepare_macro_artifact(
            repo_root,
            &control_brownout_path,
            &control_brownout_envelope,
        )
        .map_err(|error| error.to_string())?,
    );

    let resp_brownout_path = evidence_root.join("brownout-resp-endpoint.json");
    let resp_brownout_report: RespBrownoutReport = read_typed_report(&resp_brownout_path)?;
    let resp_brownout_envelope =
        build_resp_brownout_macro_envelope(&context, &resp_brownout_scenario, resp_brownout_report)
            .map_err(|error| error.to_string())?;
    prepared.push(
        prepare_macro_artifact(repo_root, &resp_brownout_path, &resp_brownout_envelope)
            .map_err(|error| error.to_string())?,
    );

    let grid_brownout_path = evidence_root.join("brownout-grid-model.json");
    let grid_brownout_report: GridModelBrownoutReport = read_typed_report(&grid_brownout_path)?;
    let grid_brownout_envelope = build_grid_model_brownout_macro_envelope(
        &context,
        &grid_brownout_scenario,
        grid_brownout_report,
    )
    .map_err(|error| error.to_string())?;
    prepared.push(
        prepare_macro_artifact(repo_root, &grid_brownout_path, &grid_brownout_envelope)
            .map_err(|error| error.to_string())?,
    );

    for name in [
        "overload-local.json",
        "overload-client-surface.json",
        "overload-node-resp.json",
    ] {
        let path = evidence_root.join(name);
        let report: OverloadReport = read_typed_report(&path)?;
        let envelope = build_overload_macro_envelope(&context, &overload_scenario, report)
            .map_err(|error| error.to_string())?;
        prepared.push(
            prepare_macro_artifact(repo_root, &path, &envelope)
                .map_err(|error| error.to_string())?,
        );
    }

    let receipt = publish_macro_batch(repo_root, prepared).map_err(|error| error.to_string())?;
    eprintln!(
        "hydracache-loadgen: atomically published {} W4-W6 budget envelopes ({})",
        receipt.artifacts.len(),
        receipt.receipt_sha256
    );
    Ok(())
}

fn read_typed_report<T>(path: &Path) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    let bytes = std::fs::read(path)
        .map_err(|error| format!("reading raw report {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("decoding raw report {}: {error}", path.display()))
}

async fn write_reference_overload(
    profile: &str,
    target: OverloadTarget,
    report_path: &Path,
) -> Result<(), String> {
    if profile != "reference-v1" {
        return Err(
            "W6 overload evidence has no promotable fixture mode; use reference-v1 with the exact predecessor surface gate"
                .to_owned(),
        );
    }
    let repo_root = repository_root()?;
    let context = load_reference_context_for_run(&repo_root)?;
    let report_path = absolute_output_path(&repo_root, report_path);
    let output_dir = report_path
        .parent()
        .ok_or_else(|| format!("W6 report path has no parent: {}", report_path.display()))?;
    let (surface, predecessor_name, lifecycle_name) = match target {
        OverloadTarget::Local => (EligibleOverloadSurface::Local, "local.json", None),
        OverloadTarget::ClientSurface => (
            EligibleOverloadSurface::ClientSurface,
            "client-surface.json",
            None,
        ),
        OverloadTarget::NodeResp => (
            EligibleOverloadSurface::NodeResp,
            "node-resp-open-loop.json",
            Some("node-resp-daemon-lifecycle.json"),
        ),
    };
    let predecessor_path = output_dir.join(predecessor_name);
    let predecessor_sha256 = sha256_file(&predecessor_path)?;
    let (lifecycle_path, lifecycle_sha256) = match lifecycle_name {
        Some(name) => {
            let path = output_dir.join(name);
            let digest = sha256_file(&path)?;
            (Some(path), Some(digest))
        }
        None => (None, None),
    };
    let publication = write_reference_overload_report(
        &repo_root,
        &context,
        &repo_root.join(OVERLOAD_SCENARIO),
        ReferencePredecessorRequest {
            surface,
            report_path: predecessor_path,
            expected_report_sha256: predecessor_sha256,
            lifecycle_path,
            expected_lifecycle_sha256: lifecycle_sha256,
            prebuild_manifest_path: context.manifest_path.clone(),
            expected_prebuild_manifest_sha256: context.manifest_sha256.clone(),
        },
        &output_dir.join("w6-run-artifacts"),
        &report_path,
    )
    .await
    .map_err(|error| error.to_string())?;
    eprintln!(
        "hydracache-loadgen: wrote receipt-bound W6 {:?} report {} ({})",
        target,
        publication.report_path.display(),
        publication.report_sha256
    );
    Ok(())
}

async fn write_reference_brownout(
    profile: &str,
    target: BrownoutTarget,
    report_path: &Path,
) -> Result<(), String> {
    if profile != "reference-v1" {
        return Err(
            "W5 brownout evidence has no fixture-capacity mode; use reference-v1 with the exact surface gate"
                .to_owned(),
        );
    }
    let repo_root = repository_root()?;
    let context = load_reference_context_for_run(&repo_root)?;
    let report_path = absolute_output_path(&repo_root, report_path);
    let output_dir = report_path
        .parent()
        .ok_or_else(|| format!("W5 report path has no parent: {}", report_path.display()))?;
    match target {
        BrownoutTarget::ControlPlaneLeader => {
            produce_control_plane_brownout(
                &repo_root,
                &context,
                &repo_root.join(BROWNOUT_CONTROL_PLANE_SCENARIO),
                &repo_root.join(CONTROL_PLANE_SCENARIO),
                &output_dir.join("control-plane-3.json"),
                &output_dir.join("w5a-run-artifacts"),
                &report_path,
            )
            .await
            .map_err(|error| error.to_string())?;
        }
        BrownoutTarget::RespEndpointKill => {
            produce_resp_brownout(
                &repo_root,
                &context,
                &repo_root.join(BROWNOUT_RESP_SCENARIO),
                &output_dir.join("node-resp-open-loop.json"),
                &output_dir.join("node-resp-daemon-lifecycle.json"),
                &output_dir.join("w5b-run-artifacts"),
                &report_path,
            )
            .await
            .map_err(|error| error.to_string())?;
        }
        BrownoutTarget::GridModelReplica => {
            produce_grid_model_brownout(
                &repo_root,
                &context,
                &repo_root.join(BROWNOUT_GRID_MODEL_SCENARIO),
                &repo_root.join(GRID_MODEL_SCENARIO),
                &output_dir.join("grid-model.json"),
                &report_path,
            )
            .await
            .map_err(|error| error.to_string())?;
        }
    }
    context
        .verify_binaries_unchanged()
        .map_err(|error| error.to_string())?;
    eprintln!(
        "hydracache-loadgen: wrote receipt-bound W5 {:?} report to {}",
        target,
        report_path.display()
    );
    Ok(())
}

async fn write_reference_control_plane_reports(
    profile: &str,
    reports: &[(u8, PathBuf)],
) -> Result<(), String> {
    if profile != "reference-v1" {
        return Err(
            "W4A has no fixture capacity mode; use reference-v1 with the mandatory real-daemon gate"
                .to_owned(),
        );
    }
    let repo_root = repository_root()?;
    let inputs = RespReferenceRunInputs::load(&repo_root).map_err(|error| error.to_string())?;
    let context = load_reference_context(&repo_root, Some(&inputs.prerequisites))
        .map_err(|error| error.to_string())?;
    let scenario = ControlPlaneScenario::load(&repo_root.join(CONTROL_PLANE_SCENARIO))
        .map_err(|error| error.to_string())?;
    for (nodes, report) in reports {
        let report = absolute_output_path(&repo_root, report);
        let evidence_root = report
            .parent()
            .ok_or_else(|| format!("W4A report path has no parent: {}", report.display()))?
            .join("w4a-run-artifacts");
        run_control_plane_reference(&context, &scenario, *nodes, &evidence_root, &report)
            .await
            .map_err(|error| error.to_string())?;
        eprintln!(
            "hydracache-loadgen: wrote receipt-bound {nodes}-daemon control-plane report to {}",
            report.display()
        );
    }
    context
        .verify_binaries_unchanged()
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn absolute_output_path(repo_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    }
}

fn direct_redis_comparison_run_mode() -> Result<RedisComparisonRunMode, String> {
    let reference = std::env::var_os("HYDRACACHE_RUN_PERF_REFERENCE");
    let resp = std::env::var_os("HYDRACACHE_RUN_PERF_RESP");
    match (reference, resp) {
        (None, None) => Ok(RedisComparisonRunMode::LocalInformational),
        (Some(reference), Some(resp))
            if reference.to_str() == Some("1") && resp.to_str() == Some("1") =>
        {
            Ok(RedisComparisonRunMode::MandatoryReference)
        }
        _ => Err(
            "direct W8 execution requires HYDRACACHE_RUN_PERF_REFERENCE=1 and HYDRACACHE_RUN_PERF_RESP=1 together for mandatory evidence, or both variables unset for a local informational run"
                .to_owned(),
        ),
    }
}

async fn write_reference_redis_comparison(
    profile: &str,
    report_path: &Path,
    run_mode: RedisComparisonRunMode,
) -> Result<(), String> {
    if profile != "reference-v1" {
        return Err(
            "W8 comparison accepts only reference-v1; local informational mode changes eligibility, not the pinned workload or identity contract"
                .to_owned(),
        );
    }
    let repo_root = repository_root()?;
    let inputs = RespReferenceRunInputs::load(&repo_root).map_err(|error| error.to_string())?;
    let context = load_reference_context(&repo_root, Some(&inputs.prerequisites))
        .map_err(|error| error.to_string())?;
    let w3_artifacts =
        W3ReferenceArtifactSet::canonical(&repo_root).map_err(|error| error.to_string())?;
    let report_path = absolute_output_path(&repo_root, report_path);
    let outcome = run_and_write_same_box_redis_comparison(
        &repo_root,
        &w3_artifacts,
        &context,
        &inputs.external_tool_prebuild,
        run_mode,
        &report_path,
    )
    .await
    .map_err(|error| error.to_string())?;
    if let RedisComparisonOutcome::Completed(report) = outcome {
        eprintln!(
            "hydracache-loadgen: wrote receipt-bound W8 same-box comparison to {} (stable={}, ship-eligible={})",
            report_path.display(),
            report.measurements_stable,
            report.ship_evidence_eligible
        );
    }
    Ok(())
}

async fn write_reference_resp_open_loop(report_path: &Path) -> Result<(), String> {
    require_reference_resp_gate()?;
    let repo_root = repository_root()?;
    let inputs = RespReferenceRunInputs::load(&repo_root).map_err(|error| error.to_string())?;
    let context = load_reference_context(&repo_root, Some(&inputs.prerequisites))
        .map_err(|error| error.to_string())?;
    let ports = RespReferencePorts::select_available().map_err(|error| error.to_string())?;
    let launch = RespDaemonLaunch::for_repeat(&repo_root, 0, ports);
    let daemon = start_reference_daemon(&context, &launch)
        .await
        .map_err(|error| error.to_string())?;
    let measurement = resp_reference_report_on(&context, &daemon).await;
    let lifecycle = daemon.stop().await;
    let report = measurement.map_err(|error| error.to_string())?;
    let lifecycle = lifecycle.map_err(|error| error.to_string())?;
    write_bytes(
        report_path,
        report.to_pretty_json().map_err(|error| error.to_string())?,
    )?;
    let lifecycle_path = report_path.with_file_name("node-resp-daemon-lifecycle.json");
    write_pretty_json(&lifecycle_path, &lifecycle)?;
    Ok(())
}

async fn write_reference_resp_suite(
    open_loop_path: &Path,
    external_path: &Path,
) -> Result<(), String> {
    require_reference_resp_gate()?;
    let repo_root = repository_root()?;
    let inputs = RespReferenceRunInputs::load(&repo_root).map_err(|error| error.to_string())?;
    let context = load_reference_context(&repo_root, Some(&inputs.prerequisites))
        .map_err(|error| error.to_string())?;
    let ports = RespReferencePorts::select_available().map_err(|error| error.to_string())?;
    let launch = RespDaemonLaunch::for_repeat(&repo_root, 0, ports);
    let daemon = start_reference_daemon(&context, &launch)
        .await
        .map_err(|error| error.to_string())?;
    let external_contract = RedisBenchmarkContract::load(&repo_root.join(RESP_EXTERNAL_SCENARIO))
        .map_err(|error| error.to_string())?;
    let provenance = ExternalToolProvenanceRegistry::load(
        &repo_root.join(REDIS_BENCHMARK_PROVENANCE_REGISTRY_PATH),
    )
    .map_err(|error| error.to_string())?;
    let evidence = run_resp_reference_suite(
        context,
        daemon,
        external_contract,
        provenance,
        inputs.external_tool_prebuild,
    )
    .await
    .map_err(|error| error.to_string())?;
    let open_loop_bytes = evidence
        .open_loop
        .to_pretty_json()
        .map_err(|error| error.to_string())?;
    let external_bytes = pretty_json_bytes(external_path, &evidence.external)?;
    let lifecycle_path = open_loop_path.with_file_name("node-resp-daemon-lifecycle.json");
    let lifecycle_bytes = pretty_json_bytes(&lifecycle_path, &evidence.daemon)?;
    let receipt = RespReferenceSuiteReceipt::seal(
        &evidence,
        &open_loop_bytes,
        &external_bytes,
        &lifecycle_bytes,
    )
    .map_err(|error| error.to_string())?;
    let receipt_path = open_loop_path.with_file_name("node-resp-suite-receipt.json");
    let receipt_bytes = pretty_json_bytes(&receipt_path, &receipt)?;
    write_bytes(open_loop_path, open_loop_bytes)?;
    write_bytes(external_path, external_bytes)?;
    write_bytes(&lifecycle_path, lifecycle_bytes)?;
    // Written last: absence of this cross-artifact receipt makes a partial run non-evidence.
    write_bytes(&receipt_path, receipt_bytes)?;
    Ok(())
}

fn require_reference_resp_gate() -> Result<(), String> {
    if std::env::var("HYDRACACHE_RUN_PERF_RESP").as_deref() != Ok("1") {
        return Err(
            "reference-v1 RESP evidence requires HYDRACACHE_RUN_PERF_RESP=1 and never falls back to fixture smoke"
                .to_owned(),
        );
    }
    Ok(())
}

fn repository_root() -> Result<PathBuf, String> {
    std::env::current_dir()
        .map_err(|error| format!("unable to resolve repository root: {error}"))?
        .canonicalize()
        .map_err(|error| format!("unable to canonicalize repository root: {error}"))
}

fn load_reference_context_for_run(
    repo_root: &Path,
) -> Result<hydracache_loadgen::tiers::resp_reference::ValidatedRespReferenceContext, String> {
    let inputs = RespReferenceRunInputs::load(repo_root).map_err(|error| error.to_string())?;
    load_reference_context(repo_root, Some(&inputs.prerequisites))
        .map_err(|error| error.to_string())
}

fn write_pretty_json(path: &Path, value: &impl serde::Serialize) -> Result<(), String> {
    write_bytes(path, pretty_json_bytes(path, value)?)
}

fn pretty_json_bytes(path: &Path, value: &impl serde::Serialize) -> Result<Vec<u8>, String> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("unable to serialize {}: {error}", path.display()))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path)
        .map_err(|error| format!("unable to read {} for SHA-256: {error}", path.display()))?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn write_bytes(path: &Path, bytes: Vec<u8>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("unable to create {}: {error}", parent.display()))?;
    }
    std::fs::write(path, bytes)
        .map_err(|error| format!("unable to write {}: {error}", path.display()))
}

fn print_help() {
    println!(
        "HydraCache release-0.67 development load generator\n\nUSAGE:\n    hydracache-loadgen tier local --profile <PROFILE> --report <PATH>\n    hydracache-loadgen tier client-surface --profile <PROFILE> --report <PATH>\n    hydracache-loadgen tier node-resp --profile <PROFILE> --report <PATH>\n    hydracache-loadgen tier control-plane --nodes <3|5|7> --target-roles leader,follower --profile reference-v1 --report <PATH>\n    hydracache-loadgen tier grid-model --profile <PROFILE> --report <PATH>\n    hydracache-loadgen suite core --profile <PROFILE> --output-dir <DIR>\n    hydracache-loadgen suite resp --profile <PROFILE> --output-dir <DIR>\n    hydracache-loadgen suite control-plane --profile reference-v1 --output-dir <DIR>\n    hydracache-loadgen compare redis --profile reference-v1 --report target/test-evidence/0.67/compare-redis.json\n    hydracache-loadgen brownout control-plane-leader --profile reference-v1 --report <PATH>\n    hydracache-loadgen brownout resp-endpoint-kill --profile reference-v1 --report <PATH>\n    hydracache-loadgen brownout grid-model-replica --profile reference-v1 --report <PATH>\n    hydracache-loadgen overload local --profile reference-v1 --report <PATH>\n    hydracache-loadgen overload client-surface --profile reference-v1 --report <PATH>\n    hydracache-loadgen overload node-resp --profile reference-v1 --report <PATH>\n\nSmoke output is explicitly plumbing-only. The client-surface tier is an in-process Router; RESP smoke uses a product-facade loopback fixture, not a daemon. W4A, W5, W6, and mandatory W8 have no promotable fixture-capacity mode and use only receipt-bound predecessors under their exact surface gates. W4B remains an explicitly in-process library/model artifact. reference-v1 fails closed until the W7 profile and receipt-bound prebuild context are present."
    );
}
