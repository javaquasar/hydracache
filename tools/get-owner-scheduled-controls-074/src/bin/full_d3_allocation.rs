//! Finite-stage allocation worker, never a timing or cohort admission authority.
#[path = "../../../resp-scratch-screen-074/src/memory.rs"]
mod memory;

#[cfg(not(test))]
#[global_allocator]
static ALLOCATOR: memory::Allocator = memory::Allocator;

use get_owner_scheduled_controls_074::{
    security::{MaterialSeal, MtlsFixture},
    timing::{requested, Input, Operation, Surface},
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{io::Read, path::Path, process::ExitCode, time::Duration};

impl requested::Scope for memory::Scope {
    type Output = memory::Measurement;
    fn finish(self) -> Self::Output {
        memory::Scope::finish(self)
    }
}

enum Arguments<'a> {
    Validate(&'a str),
    Run {
        config: &'a str,
        binary: &'a str,
        source: &'a str,
        material: Option<(&'a str, &'a str)>,
    },
}
fn digest(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn arguments(args: &[String]) -> Result<Arguments<'_>, String> {
    match args {
        [mode, config] if mode == "--validate" => Ok(Arguments::Validate(config)),
        [mode, config, binary, source, path, pin] if mode == "--run" && digest(binary,64) && digest(source,40) => {
            let material = match (path.as_str(), pin.as_str()) {
                ("-", "-") => None,
                (_, value) if !path.is_empty() && path != "-" && digest(value,64) => Some((path.as_str(),value)),
                _ => return Err("invalid explicit allocation material pin".to_owned()),
            };
            Ok(Arguments::Run { config, binary, source, material })
        }
        _ => Err("expected --validate CONFIG or --run CONFIG BINARY_SHA256 SOURCE_SHA MATERIAL_PATH MATERIAL_SHA256 (plain: - -); no retry/defaults".to_owned()),
    }
}

fn stage_input(path: &Path) -> Result<Input, String> {
    let mut raw = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(65537)
        .read_to_end(&mut raw)
        .map_err(|e| e.to_string())?;
    if raw.len() > 65536 {
        return Err("allocation config exceeds 64 KiB".to_owned());
    }
    let input: Input = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
    validate_stage(&input)?;
    Ok(input)
}
fn validate_stage(input: &Input) -> Result<(), String> {
    input.validate()?;
    let schedule = &input.schedule;
    if !matches!(input.operation, Operation::Get)
        || input.keyspace != 16
        || input.slots != 8
        || input.warmup_calls != 64
        || input.minimum_usable_cpu_ns != 1_000_000_000
        || input.minimum_usable_measurement_wall_ns != 1_000_000_000
        || schedule.operations != 128
        || schedule.offered_rate_per_second != 10000
        || schedule.maximum_queued != 128
        || schedule.operation_timeout_ns != 5_000_000_000
        || schedule.drain_timeout_ns != 5_000_000_000
        || schedule.slo_ns != 5_000_000_000
        || schedule.highest_trackable_ns != 10_000_000_000
    {
        return Err("allocation-stage workload differs from sealed contract".to_owned());
    }
    let valid = match input.surface {
        Surface::Resp2 | Surface::Resp3 | Surface::Resp2Mtls | Surface::Resp3Mtls => matches!(
            (input.payload_bytes, input.pipeline_depth),
            (4096, 50) | (1048576, 10)
        ),
        _ => input.payload_bytes == 4096 && input.pipeline_depth == 0,
    };
    if !valid {
        return Err("unsupported allocation-stage cell".to_owned());
    }
    Ok(())
}
fn binary_hash(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut buffer = [0_u8; 65536];
    let mut hash = Sha256::new();
    loop {
        let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn under_deadline<F: std::future::Future>(
    runtime: &tokio::runtime::Runtime,
    future: F,
) -> Result<F::Output, String> {
    runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(60), future)
            .await
            .map_err(|_| "allocation deadline; partial/no shutdown proof; no retry".to_owned())
    })
}

#[derive(Serialize)]
struct Envelope<'a> {
    schema_version: u32,
    profile_id: &'static str,
    source_commit_from_coordinator: &'a str,
    source_git_identity_verified_by_binary: bool,
    binary_sha256: String,
    compiled_root_lock_sha256: String,
    compiled_observer_lock_sha256: String,
    allocation_diagnostics_feature: bool,
    fixed_material: Option<MaterialSeal>,
    report: Option<requested::Report<memory::Measurement>>,
    error: Option<String>,
}

fn execute(args: &[String]) -> Result<bool, String> {
    let (config, binary, source, material) = match arguments(args)? {
        Arguments::Validate(path) => {
            let input = stage_input(Path::new(path))?;
            println!(
                "{}",
                serde_json::json!({"profile_id":requested::PROFILE,"valid":true,
                "workload_sha256":input.workload_sha256(),"fixture_started":false,"admission_allowed":false})
            );
            return Ok(true);
        }
        Arguments::Run {
            config,
            binary,
            source,
            material,
        } => (config, binary, source, material),
    };
    if !cfg!(feature = "allocation-diagnostics") {
        return Err("counting feature missing".to_owned());
    }
    let input = stage_input(Path::new(config))?;
    let actual = binary_hash(&std::env::current_exe().map_err(|e| e.to_string())?)?;
    if actual != binary {
        return Err("allocation binary seal mismatch before fixture".to_owned());
    }
    if input.surface.uses_mtls() != material.is_some() {
        return Err("allocation material/surface mismatch before fixture".to_owned());
    }
    let fixed = material
        .map(|(path, pin)| MtlsFixture::read_pinned(Path::new(path), pin))
        .transpose()?;
    let mut envelope = Envelope {
        schema_version: 1,
        profile_id: requested::PROFILE,
        source_commit_from_coordinator: source,
        source_git_identity_verified_by_binary: false,
        binary_sha256: actual,
        compiled_root_lock_sha256: format!(
            "{:x}",
            Sha256::digest(include_bytes!("../../../../Cargo.lock"))
        ),
        compiled_observer_lock_sha256: format!(
            "{:x}",
            Sha256::digest(include_bytes!("../../Cargo.lock"))
        ),
        allocation_diagnostics_feature: cfg!(feature = "allocation-diagnostics"),
        fixed_material: fixed.as_ref().map(|(_, seal)| seal.clone()),
        report: None,
        error: None,
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    match under_deadline(
        &runtime,
        requested::run(
            input,
            fixed.as_ref().map(|(material, _)| material),
            memory::Scope::start,
        ),
    ) {
        Ok(report) => envelope.report = Some(report),
        Err(error) => envelope.error = Some(error),
    }
    let passed = envelope.error.is_none()
        && envelope.report.as_ref().is_some_and(|r| {
            r.error.is_none()
                && r.final_dataset_verified
                && r.shutdown_verified
                && r.requested_layouts.is_some()
        });
    println!(
        "{}",
        serde_json::to_string(&envelope).map_err(|e| e.to_string())?
    );
    Ok(passed)
}
fn main() -> ExitCode {
    match execute(&std::env::args().skip(1).collect::<Vec<_>>()) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use get_owner_scheduled_controls_074::native::Dataset;
    fn input() -> Input {
        serde_json::from_value(serde_json::json!({
            "schema_version":1,"profile_id":"unprofiled-timing-controls-074-v1",
            "surface":"resp2","operation":"get","seed":740074,"keyspace":16,
            "payload_bytes":4096,"dataset_sha256":Dataset::new(16,4096).unwrap().digest(),
            "slots":8,"pipeline_depth":50,"warmup_calls":64,
            "minimum_usable_cpu_ns":1000000000_u64,"minimum_usable_measurement_wall_ns":1000000000_u64,
            "schedule":{"operations":128,"offered_rate_per_second":10000,"concurrency":8,
                "maximum_queued":128,"operation_timeout_ns":5000000000_u64,"drain_timeout_ns":5000000000_u64,
                "slo_ns":5000000000_u64,"highest_trackable_ns":10000000000_u64}
        })).unwrap()
    }
    #[test]
    fn exact_stage_cells_and_identity_bounds_without_workloads() {
        let base = input();
        validate_stage(&base).unwrap();
        for field in ["slots", "warmup_calls", "keyspace", "minimum_usable_cpu_ns"] {
            let mut value = serde_json::to_value(&base).unwrap();
            value[field] = serde_json::json!(0);
            assert!(validate_stage(&serde_json::from_value(value).unwrap()).is_err());
        }
        for surface in ["embedded", "direct", "hc1", "hc2-mtls"] {
            let mut value = serde_json::to_value(&base).unwrap();
            value["surface"] = serde_json::json!(surface);
            value["pipeline_depth"] = serde_json::json!(0);
            validate_stage(&serde_json::from_value(value).unwrap()).unwrap();
        }
        for surface in ["resp2", "resp3", "resp2-mtls", "resp3-mtls"] {
            for (payload, pipeline) in [(4096, 50), (1048576, 10)] {
                let mut value = serde_json::to_value(&base).unwrap();
                value["surface"] = serde_json::json!(surface);
                value["payload_bytes"] = serde_json::json!(payload);
                value["pipeline_depth"] = serde_json::json!(pipeline);
                value["dataset_sha256"] =
                    serde_json::json!(Dataset::new(16, payload).unwrap().digest());
                validate_stage(&serde_json::from_value(value).unwrap()).unwrap();
            }
        }
        for field in [
            "maximum_queued",
            "operation_timeout_ns",
            "drain_timeout_ns",
            "slo_ns",
            "highest_trackable_ns",
            "offered_rate_per_second",
        ] {
            let mut value = serde_json::to_value(&base).unwrap();
            value["schedule"][field] = serde_json::json!(1);
            assert!(validate_stage(&serde_json::from_value(value).unwrap()).is_err());
        }
        let mut drift = base;
        drift.schedule.operations += 1;
        assert!(validate_stage(&drift).is_err());
    }
    #[test]
    fn argument_grammar_requires_exact_pins_and_no_fallback_or_retry() {
        let prefix = vec![
            "--run".to_owned(),
            "config.json".to_owned(),
            "a".repeat(64),
            "b".repeat(40),
        ];
        let mut plain = prefix.clone();
        plain.extend(["-".to_owned(), "-".to_owned()]);
        assert!(arguments(&plain).is_ok());
        let mut secure = prefix;
        secure.extend(["private.json".to_owned(), "c".repeat(64)]);
        assert!(arguments(&secure).is_ok());
        secure[5] = "C".repeat(64);
        assert!(arguments(&secure).is_err());
        plain.push("retry".to_owned());
        assert!(arguments(&plain).is_err());
    }

    #[test]
    fn deadline_is_created_inside_the_runtime_without_starting_a_fixture() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        assert_eq!(under_deadline(&runtime, async { 42 }).unwrap(), 42);
    }
}
