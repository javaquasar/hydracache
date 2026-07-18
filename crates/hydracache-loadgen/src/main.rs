use std::path::{Path, PathBuf};

use hydracache_loadgen::cli;
use hydracache_loadgen::cli::LoadgenCommand;
use hydracache_loadgen::resp_external::{
    ExternalToolProvenanceRegistry, RedisBenchmarkContract,
    REDIS_BENCHMARK_PROVENANCE_REGISTRY_PATH,
};
use hydracache_loadgen::tiers::client_surface::write_client_surface_report;
use hydracache_loadgen::tiers::local::write_local_report;
use hydracache_loadgen::tiers::resp::{
    resp_reference_report_on, run_resp_reference_suite, write_resp_report, RespReferenceRunInputs,
    RespReferenceSuiteReceipt,
};
use hydracache_loadgen::tiers::resp_reference::{
    load_reference_context, start_reference_daemon, RespDaemonLaunch, RespReferencePorts,
};

const RESP_EXTERNAL_SCENARIO: &str =
    "docs/testing/perf-scenarios/0.67/resp-external-redis-benchmark-v1.toml";

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
            write_local_report(command.profile(), &path)
                .await
                .map_err(|error| error.to_string())?;
            eprintln!(
                "hydracache-loadgen: wrote plumbing-only local smoke report to {}",
                path.display()
            );
        }
        LoadgenCommand::TierClientSurface { .. } => {
            let path = command.client_surface_report_path().ok_or_else(|| {
                "client-surface command lost its canonical report path".to_owned()
            })?;
            write_client_surface_report(command.profile(), &path)
                .await
                .map_err(|error| error.to_string())?;
        }
        LoadgenCommand::SuiteCore { .. } => {
            let local_path = command
                .local_report_path()
                .ok_or_else(|| "core suite lost its local report path".to_owned())?;
            write_local_report(command.profile(), &local_path)
                .await
                .map_err(|error| error.to_string())?;
            let client_surface_path = command
                .client_surface_report_path()
                .ok_or_else(|| "core suite lost its client-surface report path".to_owned())?;
            write_client_surface_report(command.profile(), &client_surface_path)
                .await
                .map_err(|error| error.to_string())?;
            eprintln!(
                "hydracache-loadgen: wrote plumbing-only core suite reports to {} and {}",
                local_path.display(),
                client_surface_path.display()
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
        LoadgenCommand::SuiteResp { .. } => {
            let path = command
                .resp_open_loop_report_path()
                .ok_or_else(|| "RESP suite lost its canonical open-loop report path".to_owned())?;
            if command.profile() == "reference-v1" {
                let external_path = command.resp_external_report_path().ok_or_else(|| {
                    "RESP suite lost its canonical external-tool report path".to_owned()
                })?;
                write_reference_resp_suite(&path, &external_path).await?;
            } else {
                write_resp_report(command.profile(), &path)
                    .await
                    .map_err(|error| error.to_string())?;
                eprintln!(
                    "hydracache-loadgen: supplemental redis-benchmark evidence skipped loudly for fixture smoke; it requires the selected receipt-bound daemon endpoint"
                );
            }
        }
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

fn write_pretty_json(path: &Path, value: &impl serde::Serialize) -> Result<(), String> {
    write_bytes(path, pretty_json_bytes(path, value)?)
}

fn pretty_json_bytes(path: &Path, value: &impl serde::Serialize) -> Result<Vec<u8>, String> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("unable to serialize {}: {error}", path.display()))?;
    bytes.push(b'\n');
    Ok(bytes)
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
        "HydraCache release-0.67 development load generator\n\nUSAGE:\n    hydracache-loadgen tier local --profile <PROFILE> --report <PATH>\n    hydracache-loadgen tier client-surface --profile <PROFILE> --report <PATH>\n    hydracache-loadgen tier node-resp --profile <PROFILE> --report <PATH>\n    hydracache-loadgen suite core --profile <PROFILE> --output-dir <DIR>\n    hydracache-loadgen suite resp --profile <PROFILE> --output-dir <DIR>\n\nSmoke output is explicitly plumbing-only. The client-surface tier is an in-process Router; RESP smoke uses a product-facade loopback fixture, not a daemon. reference-v1 fails closed until the W7 profile and receipt-bound prebuild context are present."
    );
}
