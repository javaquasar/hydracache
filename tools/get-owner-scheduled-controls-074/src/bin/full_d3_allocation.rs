//! Finite-stage allocation worker, never a timing or cohort admission authority.
#[path = "../../../resp-scratch-screen-074/src/memory.rs"]
mod memory;

#[cfg(not(test))]
#[global_allocator]
static ALLOCATOR: memory::Allocator = memory::Allocator;

use get_owner_scheduled_controls_074::{
    security::{MaterialSeal, MtlsFixture},
    timing::{requested, Input, Operation, Surface, TimedObservation},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{io::Read, path::Path, process::ExitCode, time::Duration};

impl requested::Scope for memory::Scope {
    type Output = memory::Measurement;
    fn finish(self) -> Self::Output {
        memory::Scope::finish(self)
    }
}

const PROFILE: &str = "get-owner-full-d3-allocation-074-v2";
// The sealed historical cohort rejected a mandatory gate. Keep metadata/replay
// tools, but never silently measure this source's now-disabled candidate again.
const CANDIDATE_RETIRED: bool = true;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StageInput {
    schema_version: u32,
    profile_id: String,
    workload: Input,
    resp_connections: usize,
}
impl StageInput {
    fn workload_sha256(&self) -> String {
        format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(self).expect("finite stage input"))
        )
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

fn stage_input(path: &Path) -> Result<StageInput, String> {
    let mut raw = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(65537)
        .read_to_end(&mut raw)
        .map_err(|e| e.to_string())?;
    if raw.len() > 65536 {
        return Err("allocation config exceeds 64 KiB".to_owned());
    }
    let input: StageInput = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
    validate_stage(&input)?;
    Ok(input)
}
fn validate_stage(stage: &StageInput) -> Result<(), String> {
    if stage.schema_version != 2 || stage.profile_id != PROFILE {
        return Err("unsupported allocation placement schema/profile".to_owned());
    }
    let input = &stage.workload;
    input.validate()?;
    let schedule = &input.schedule;
    let resp = matches!(
        input.surface,
        Surface::Resp2 | Surface::Resp3 | Surface::Resp2Mtls | Surface::Resp3Mtls
    );
    let (slots, operations, rate, queued, connections) = if resp {
        (128, 1000, 1_000_000, 1024, 1)
    } else {
        (8, 128, 10000, 128, 0)
    };
    if !matches!(input.operation, Operation::Get)
        || input.keyspace != 16
        || input.slots != slots
        || stage.resp_connections != connections
        || input.warmup_calls != 64
        || input.minimum_usable_cpu_ns != 1_000_000_000
        || input.minimum_usable_measurement_wall_ns != 1_000_000_000
        || schedule.operations != operations
        || schedule.offered_rate_per_second != rate
        || schedule.maximum_queued != queued
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

fn interval_high_water(intervals: impl IntoIterator<Item = (u64, u64)>) -> Result<usize, String> {
    let mut events = Vec::new();
    for (write, reply) in intervals {
        if reply < write {
            return Err("wire response precedes write".to_owned());
        }
        if reply > write {
            events.push((write, 1_i64));
            events.push((reply, -1_i64));
        }
    }
    events.sort_unstable(); // Ends before starts at equal timestamps: no invented overlap.
    let (mut active, mut peak) = (0_i64, 0_i64);
    for (_, delta) in events {
        active += delta;
        if active < 0 {
            return Err("wire interval underflow".to_owned());
        }
        peak = peak.max(active);
    }
    if active != 0 {
        return Err("wire interval not closed".to_owned());
    }
    Ok(peak as usize)
}
fn wire_high_water<R: Serialize>(report: &requested::Report<R>) -> Result<Option<usize>, String> {
    let Some(TimedObservation::Resp(resp)) = &report.observation else {
        return Ok(None);
    };
    if resp.physical_connections != 1 {
        return Err("v2 wire proof requires one physical connection".to_owned());
    }
    let intervals = resp
        .wire_samples
        .iter()
        .map(|w| {
            Ok((
                w.write_completed_ns.ok_or("missing write timestamp")?,
                w.response_complete_ns.ok_or("missing response timestamp")?,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    interval_high_water(intervals).map(Some)
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
    stage_input: StageInput,
    stage_workload_sha256: String,
    actual_wire_inflight_high_water: Option<usize>,
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
                serde_json::json!({"profile_id":PROFILE,"valid":true,
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
    if CANDIDATE_RETIRED {
        return Err("GET-owner is terminally rejected for 0.74; no fresh numerical execution; replay the sealed historical packet".to_owned());
    }
    if !cfg!(feature = "allocation-diagnostics") {
        return Err("counting feature missing".to_owned());
    }
    let input = stage_input(Path::new(config))?;
    let actual = binary_hash(&std::env::current_exe().map_err(|e| e.to_string())?)?;
    if actual != binary {
        return Err("allocation binary seal mismatch before fixture".to_owned());
    }
    if input.workload.surface.uses_mtls() != material.is_some() {
        return Err("allocation material/surface mismatch before fixture".to_owned());
    }
    let fixed = material
        .map(|(path, pin)| MtlsFixture::read_pinned(Path::new(path), pin))
        .transpose()?;
    let workload = input.workload.clone();
    let connections = input.resp_connections;
    let stage_hash = input.workload_sha256();
    let mut envelope = Envelope {
        schema_version: 2,
        profile_id: PROFILE,
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
        stage_input: input,
        stage_workload_sha256: stage_hash,
        actual_wire_inflight_high_water: None,
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
        requested::run_with_resp_connections(
            workload,
            fixed.as_ref().map(|(material, _)| material),
            connections,
            memory::Scope::start,
        ),
    ) {
        Ok(report) => envelope.report = Some(report),
        Err(error) => envelope.error = Some(error),
    }
    if let Some(report) = &envelope.report {
        match wire_high_water(report) {
            Ok(peak) => {
                envelope.actual_wire_inflight_high_water = peak;
                if connections != 0 && peak != Some(envelope.stage_input.workload.pipeline_depth) {
                    envelope.error = Some(
                        "configured pipeline depth not reached; invalid stage; no retry".to_owned(),
                    );
                }
            }
            Err(error) => envelope.error = Some(error),
        }
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
    fn input() -> StageInput {
        let workload = serde_json::from_value(serde_json::json!({
            "schema_version":1,"profile_id":"unprofiled-timing-controls-074-v1",
            "surface":"resp2","operation":"get","seed":740074,"keyspace":16,
            "payload_bytes":4096,"dataset_sha256":Dataset::new(16,4096).unwrap().digest(),
            "slots":128,"pipeline_depth":50,"warmup_calls":64,
            "minimum_usable_cpu_ns":1000000000_u64,"minimum_usable_measurement_wall_ns":1000000000_u64,
            "schedule":{"operations":1000,"offered_rate_per_second":1000000,"concurrency":128,
                "maximum_queued":1024,"operation_timeout_ns":5000000000_u64,"drain_timeout_ns":5000000000_u64,
                "slo_ns":5000000000_u64,"highest_trackable_ns":10000000000_u64}
        })).unwrap();
        StageInput {
            schema_version: 2,
            profile_id: PROFILE.to_owned(),
            workload,
            resp_connections: 1,
        }
    }
    #[test]
    fn exact_stage_cells_and_identity_bounds_without_workloads() {
        let base = input();
        validate_stage(&base).unwrap();
        for field in ["slots", "warmup_calls", "keyspace", "minimum_usable_cpu_ns"] {
            let mut value = serde_json::to_value(&base).unwrap();
            value["workload"][field] = serde_json::json!(0);
            assert!(validate_stage(&serde_json::from_value(value).unwrap()).is_err());
        }
        for surface in ["embedded", "direct", "hc1", "hc2-mtls"] {
            let mut value = serde_json::to_value(&base).unwrap();
            value["resp_connections"] = serde_json::json!(0);
            let workload = &mut value["workload"];
            workload["surface"] = serde_json::json!(surface);
            workload["pipeline_depth"] = serde_json::json!(0);
            workload["slots"] = serde_json::json!(8);
            workload["schedule"]["concurrency"] = serde_json::json!(8);
            workload["schedule"]["operations"] = serde_json::json!(128);
            workload["schedule"]["offered_rate_per_second"] = serde_json::json!(10000);
            workload["schedule"]["maximum_queued"] = serde_json::json!(128);
            validate_stage(&serde_json::from_value(value).unwrap()).unwrap();
        }
        for surface in ["resp2", "resp3", "resp2-mtls", "resp3-mtls"] {
            for (payload, pipeline) in [(4096, 50), (1048576, 10)] {
                let mut value = serde_json::to_value(&base).unwrap();
                value["workload"]["surface"] = serde_json::json!(surface);
                value["workload"]["payload_bytes"] = serde_json::json!(payload);
                value["workload"]["pipeline_depth"] = serde_json::json!(pipeline);
                value["workload"]["dataset_sha256"] =
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
            value["workload"]["schedule"][field] = serde_json::json!(1);
            assert!(validate_stage(&serde_json::from_value(value).unwrap()).is_err());
        }
        let mut drift = base;
        drift.workload.schedule.operations += 1;
        assert!(validate_stage(&drift).is_err());
    }
    #[test]
    fn physical_placement_is_required_strict_and_part_of_identity() {
        let base = input();
        let mut changed = base.clone();
        changed.resp_connections = 8;
        assert_ne!(base.workload_sha256(), changed.workload_sha256());
        assert!(validate_stage(&changed).is_err());
        let mut value = serde_json::to_value(&base).unwrap();
        value.as_object_mut().unwrap().remove("resp_connections");
        assert!(serde_json::from_value::<StageInput>(value).is_err());
        let mut value = serde_json::to_value(&base).unwrap();
        value["retry"] = serde_json::json!(true);
        assert!(serde_json::from_value::<StageInput>(value).is_err());
    }
    #[test]
    fn wire_intervals_do_not_invent_overlap_at_equal_timestamps() {
        assert_eq!(
            interval_high_water([(1, 3), (2, 4), (4, 5), (6, 6)]).unwrap(),
            2
        );
        assert_eq!(interval_high_water([(1, 2), (2, 3)]).unwrap(), 1);
        assert_eq!(interval_high_water([]).unwrap(), 0);
        assert!(interval_high_water([(2, 1)]).is_err());
    }
    #[tokio::test]
    #[ignore = "explicit GNU release semantic preflight; not a fast unit or performance measurement"]
    async fn semantic_wire_control_reaches_each_required_depth_without_counting() {
        struct SemanticOnly;
        impl requested::Scope for SemanticOnly {
            type Output = ();
            fn finish(self) {}
        }
        // This test binary has no counting allocator. Never admit performance
        // from these semantic receipts or silently repair a failed first cell.
        let material = MtlsFixture::new().unwrap();
        for surface in [
            Surface::Resp2,
            Surface::Resp3,
            Surface::Resp2Mtls,
            Surface::Resp3Mtls,
        ] {
            for (payload, depth) in [(4096, 50), (1048576, 10)] {
                let mut stage = input();
                stage.workload.surface = surface;
                stage.workload.payload_bytes = payload;
                stage.workload.pipeline_depth = depth;
                stage.workload.dataset_sha256 = Dataset::new(16, payload).unwrap().digest();
                validate_stage(&stage).unwrap();
                let report = tokio::time::timeout(
                    Duration::from_secs(60),
                    requested::run_with_resp_connections(
                        stage.workload,
                        surface.uses_mtls().then_some(&material),
                        1,
                        || SemanticOnly,
                    ),
                )
                .await
                .expect("semantic cell deadline; no retry");
                let peak = wire_high_water(&report).unwrap();
                if let Some(observation) = &report.observation {
                    let o = observation.operations();
                    println!("semantic-only offered={} successes={} errors={} timeouts={} queue_timeouts={} admission_rejections={} target_rejections={} incomplete={} scheduled_overflows={} service_overflows={}",o.offered,o.successes,o.errors,o.timeouts,o.queue_timeouts,o.admission_rejections,o.target_rejections,o.incomplete,o.scheduled_response_latency.overflow_count,o.service_response_latency.overflow_count);
                }
                println!("semantic-only surface={surface:?} payload={payload} configured={depth} observed={peak:?} error={:?} shutdown={}",report.error,report.shutdown_verified);
                assert!(report.error.is_none(), "{:?}", report.error);
                assert!(report.final_dataset_verified && report.shutdown_verified);
                assert_eq!(peak, Some(depth));
            }
        }
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
    fn retired_candidate_refuses_before_opening_config_or_starting_fixture() {
        let args = vec![
            "--run".to_owned(),
            "absent-config-must-not-be-opened".to_owned(),
            "a".repeat(64),
            "b".repeat(40),
            "-".to_owned(),
            "-".to_owned(),
        ];
        assert!(execute(&args).unwrap_err().contains("terminally rejected"));
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
