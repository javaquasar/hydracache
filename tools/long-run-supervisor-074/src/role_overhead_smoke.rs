use crate::host_receipt::collect_installed_provisioning_identity;
use crate::systemd_unit::{
    cpuset_mask, inspect_supervisor_unit, inspect_unit_optional, start_transient_unit,
    stop_unit_and_wait, ExecCommand, TransientUnitSpec, UnitProperty,
};
use crate::{sha256_hex, ProcessIdentity};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

const INSTALLED_BINARY: &str = "/opt/hydracache-perf/bin/hydracache-long-run-supervisor-074";
const PROVISIONING_RECEIPT: &str = "/var/lib/hydracache-performance/provisioning-receipt-074.json";
const ACTIVE_CAMPAIGN: &str = "/var/lib/hydracache-performance/campaigns/active-campaign";
const OUTPUT_DIRECTORY: &str = "/run/hydracache-perf/role-overhead-smoke-v1";
const SUPERVISOR_UNIT: &str = "hydracache-performance-supervisor-074.service";
const SCHEMA_VERSION: u32 = 1;
const SEED: u64 = 740_074;
const PAIRS: u32 = 5;
const OPERATIONS: u64 = 100_000_000;
const WARMUP_OPERATIONS: u64 = 1_000_000;
const CHECKPOINT_BYTES: usize = 4_096;
const FIXED_DELAY: Duration = Duration::from_millis(300);
const ATTEMPT_TIMEOUT: Duration = Duration::from_secs(10);
const WORKLOAD_DEFINITION: &[u8] =
    b"hydracache-0.74-non-product-role-overhead-xorshift64-v1;operations=100000000;warmup=1000000;delay_ms=300";
const PAYLOAD_DEFINITION: &[u8] =
    b"hydracache-0.74-non-product-role-overhead-checkpoint-v1;bytes=4096";

#[derive(Debug, Serialize)]
pub struct AttemptSet {
    schema_version: u32,
    release: &'static str,
    evidence_class: &'static str,
    promotable: bool,
    seed: u64,
    order: &'static str,
    attempts: Vec<Attempt>,
}

#[derive(Debug, Serialize)]
struct Attempt {
    schema_version: u32,
    role: &'static str,
    variant: &'static str,
    pair_index: u32,
    position: u32,
    identity: AttemptIdentity,
    metrics: AttemptMetrics,
    guards: AttemptGuards,
}

#[derive(Debug, Clone, Serialize)]
struct AttemptIdentity {
    source_commit: String,
    binary_sha256: String,
    workload_sha256: String,
    payload_sha256: String,
    host_receipt_sha256: String,
    seed: u64,
    operations: u64,
    warmup_operations: u64,
    cpuset: String,
}

#[derive(Debug, Serialize)]
struct AttemptMetrics {
    elapsed_ns: u64,
    role_cpu_ns: u64,
    role_rss_peak_bytes: u64,
    role_io_bytes: u64,
    supervisor_cpu_ns: u64,
    supervisor_rss_peak_bytes: u64,
    supervisor_io_bytes: u64,
    checkpoint_write_bytes: u64,
    completed_operations: u64,
}

#[derive(Debug, Serialize)]
struct AttemptGuards {
    affinity_applied: bool,
    priority_applied: bool,
    host_identity_stable: bool,
    role_identity_stable: bool,
    supervisor_identity_stable: bool,
    campaign_claim_valid: bool,
    unexpected_errors_absent: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FixtureReceipt {
    schema_version: u32,
    role: String,
    variant: String,
    pair_index: u32,
    position: u32,
    cpuset: String,
    nice: i32,
    elapsed_ns: u64,
    cpu_ns: u64,
    rss_peak_bytes: u64,
    io_bytes: u64,
    checkpoint_write_bytes: u64,
    completed_operations: u64,
}

#[derive(Debug)]
struct SupervisorSample {
    identity: ProcessIdentity,
    cpu_ns: u64,
    rss_peak_bytes: u64,
    io_bytes: u64,
}

pub fn run_role_overhead_smoke() -> Result<AttemptSet, String> {
    require_root_and_installed_binary()?;
    require_no_active_campaign()?;
    prepare_output_directory()?;

    let binary_bytes = read_bounded(Path::new(INSTALLED_BINARY), 128 * 1024 * 1024)?;
    let binary_sha256 = sha256_hex(&binary_bytes);
    let installed = collect_installed_provisioning_identity(&binary_sha256)
        .map_err(|error| error.to_string())?;
    let receipt_bytes = read_bounded(Path::new(PROVISIONING_RECEIPT), 1024 * 1024)?;
    let boot_id = read_trimmed(Path::new("/proc/sys/kernel/random/boot_id"), 128)?;
    let cpuset = first_allowed_cpu(&read_status_value(
        Path::new("/proc/self/status"),
        "Cpus_allowed_list",
    )?)?;
    let identity = AttemptIdentity {
        source_commit: installed.source_commit,
        binary_sha256,
        workload_sha256: sha256_hex(WORKLOAD_DEFINITION),
        payload_sha256: sha256_hex(PAYLOAD_DEFINITION),
        host_receipt_sha256: sha256_hex(&receipt_bytes),
        seed: SEED,
        operations: OPERATIONS,
        warmup_operations: WARMUP_OPERATIONS,
        cpuset,
    };

    let mut attempts = Vec::with_capacity((PAIRS * 4) as usize);
    for role in ["i74", "c74"] {
        for pair_index in 1..=PAIRS {
            let variants = if pair_index % 2 == 1 {
                ["control", "instrumented"]
            } else {
                ["instrumented", "control"]
            };
            for (index, variant) in variants.into_iter().enumerate() {
                attempts.push(run_attempt(
                    role,
                    variant,
                    pair_index,
                    index as u32 + 1,
                    &identity,
                    &boot_id,
                )?);
            }
        }
    }
    require_no_active_campaign()?;
    if read_trimmed(Path::new("/proc/sys/kernel/random/boot_id"), 128)? != boot_id {
        return Err("host boot identity changed during role-overhead rehearsal".to_owned());
    }
    Ok(AttemptSet {
        schema_version: SCHEMA_VERSION,
        release: "0.74",
        evidence_class: "non-product-role-overhead-rehearsal",
        promotable: false,
        seed: SEED,
        order: "abba-counterbalanced-v1",
        attempts,
    })
}

#[allow(clippy::too_many_arguments)]
fn run_attempt(
    role: &'static str,
    variant: &'static str,
    pair_index: u32,
    position: u32,
    identity: &AttemptIdentity,
    boot_id: &str,
) -> Result<Attempt, String> {
    require_no_active_campaign()?;
    let unit_name =
        format!("hydracache-performance-074-role-overhead-{role}-p{pair_index}-{position}.service");
    if inspect_unit_optional(&unit_name)
        .map_err(|error| error.to_string())?
        .is_some()
    {
        return Err(format!(
            "fixed role-overhead unit already exists: {unit_name}"
        ));
    }
    let checkpoint_path = output_path(role, pair_index, position, "checkpoint")?;
    let receipt_path = output_path(role, pair_index, position, "json")?;
    prepare_output_file(&checkpoint_path)?;
    prepare_output_file(&receipt_path)?;

    let supervisor_before = sample_supervisor()?;
    let spec = attempt_spec(
        &unit_name,
        role,
        variant,
        pair_index,
        position,
        &identity.cpuset,
        (&checkpoint_path, &receipt_path),
    )?;
    start_transient_unit(&spec).map_err(|error| error.to_string())?;
    let observed = wait_for_terminal(&unit_name);
    let cleanup = stop_unit_and_wait(&unit_name, 5).map_err(|error| error.to_string());
    observed?;
    cleanup?;

    let fixture: FixtureReceipt = serde_json::from_slice(&read_bounded(&receipt_path, 64 * 1024)?)
        .map_err(|error| format!("fixture receipt is invalid: {error}"))?;
    let checkpoint_size = safe_regular_size(&checkpoint_path, CHECKPOINT_BYTES as u64)?;
    let supervisor_after = sample_supervisor()?;
    let current_boot = read_trimmed(Path::new("/proc/sys/kernel/random/boot_id"), 128)?;
    let role_identity_stable = fixture.schema_version == SCHEMA_VERSION
        && fixture.role == role
        && fixture.variant == variant
        && fixture.pair_index == pair_index
        && fixture.position == position
        && fixture.completed_operations == OPERATIONS
        && fixture.checkpoint_write_bytes == checkpoint_size;
    let expected_checkpoint = if variant == "instrumented" {
        CHECKPOINT_BYTES as u64
    } else {
        0
    };
    if !role_identity_stable
        || checkpoint_size != expected_checkpoint
        || fixture.io_bytes < checkpoint_size
        || fixture.elapsed_ns == 0
        || fixture.cpu_ns == 0
        || fixture.rss_peak_bytes == 0
    {
        return Err("role-overhead fixture receipt or checkpoint differs".to_owned());
    }
    let supervisor_identity_stable = supervisor_before.identity == supervisor_after.identity;
    if !supervisor_identity_stable {
        return Err("supervisor identity changed during role-overhead attempt".to_owned());
    }
    require_no_active_campaign()?;
    Ok(Attempt {
        schema_version: SCHEMA_VERSION,
        role,
        variant,
        pair_index,
        position,
        identity: identity.clone(),
        metrics: AttemptMetrics {
            elapsed_ns: fixture.elapsed_ns,
            role_cpu_ns: fixture.cpu_ns,
            role_rss_peak_bytes: fixture.rss_peak_bytes,
            role_io_bytes: fixture.io_bytes,
            supervisor_cpu_ns: supervisor_after
                .cpu_ns
                .saturating_sub(supervisor_before.cpu_ns),
            supervisor_rss_peak_bytes: supervisor_after.rss_peak_bytes,
            supervisor_io_bytes: supervisor_after
                .io_bytes
                .saturating_sub(supervisor_before.io_bytes),
            checkpoint_write_bytes: fixture.checkpoint_write_bytes,
            completed_operations: fixture.completed_operations,
        },
        guards: AttemptGuards {
            affinity_applied: fixture.cpuset == identity.cpuset,
            priority_applied: fixture.nice == 0,
            host_identity_stable: current_boot == boot_id,
            role_identity_stable,
            supervisor_identity_stable,
            campaign_claim_valid: true,
            unexpected_errors_absent: true,
        },
    })
}

pub fn run_role_overhead_fixture(args: &[String]) -> Result<(), String> {
    if unsafe { libc::geteuid() } == 0 || args.len() != 5 {
        return Err("role-overhead fixture invocation is invalid".to_owned());
    }
    let role = args[0].as_str();
    let variant = args[1].as_str();
    let pair_index = parse_bounded(&args[2], 1, PAIRS)?;
    let position = parse_bounded(&args[3], 1, 2)?;
    let cpuset = &args[4];
    if !matches!(role, "i74" | "c74")
        || !matches!(variant, "control" | "instrumented")
        || variant != expected_variant(pair_index, position)
        || first_allowed_cpu(cpuset)? != cpuset.as_str()
    {
        return Err("role-overhead fixture schedule is invalid".to_owned());
    }
    if read_status_value(Path::new("/proc/self/status"), "Cpus_allowed_list")? != *cpuset {
        return Err("role-overhead fixture affinity differs".to_owned());
    }
    let nice = unsafe { libc::getpriority(libc::PRIO_PROCESS, 0) };
    if nice != 0 {
        return Err("role-overhead fixture priority differs".to_owned());
    }

    run_operations(WARMUP_OPERATIONS);
    let io_before = proc_io_characters(Path::new("/proc/self/io"))?;
    let cpu_before = process_cpu_time_ns()?;
    let started = Instant::now();
    run_operations(OPERATIONS);
    let mut checkpoint_write_bytes = 0_u64;
    if variant == "instrumented" {
        let bytes = vec![b'h'; CHECKPOINT_BYTES];
        let mut stdout = std::io::stdout().lock();
        stdout
            .write_all(&bytes)
            .map_err(|error| error.to_string())?;
        stdout.flush().map_err(|error| error.to_string())?;
        if unsafe { libc::fsync(libc::STDOUT_FILENO) } != 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        checkpoint_write_bytes = CHECKPOINT_BYTES as u64;
    }
    thread::sleep(FIXED_DELAY);
    let elapsed_ns = u64::try_from(started.elapsed().as_nanos())
        .map_err(|_| "fixture elapsed time overflowed".to_owned())?;
    let cpu_ns = process_cpu_time_ns()?.saturating_sub(cpu_before);
    let io_bytes = proc_io_characters(Path::new("/proc/self/io"))?.saturating_sub(io_before);
    let rss_peak_bytes = process_peak_rss_bytes(Path::new("/proc/self/status"))?;
    let receipt = FixtureReceipt {
        schema_version: SCHEMA_VERSION,
        role: role.to_owned(),
        variant: variant.to_owned(),
        pair_index,
        position,
        cpuset: cpuset.clone(),
        nice,
        elapsed_ns,
        cpu_ns,
        rss_peak_bytes,
        io_bytes,
        checkpoint_write_bytes,
        completed_operations: OPERATIONS,
    };
    serde_json::to_writer(std::io::stderr().lock(), &receipt).map_err(|error| error.to_string())?;
    eprintln!();
    Ok(())
}

fn attempt_spec(
    unit_name: &str,
    role: &str,
    variant: &str,
    pair_index: u32,
    position: u32,
    cpuset: &str,
    output_paths: (&Path, &Path),
) -> Result<TransientUnitSpec, String> {
    let (checkpoint_path, receipt_path) = output_paths;
    let checkpoint = checkpoint_path
        .to_str()
        .ok_or_else(|| "checkpoint path is not UTF-8".to_owned())?;
    let receipt = receipt_path
        .to_str()
        .ok_or_else(|| "receipt path is not UTF-8".to_owned())?;
    Ok(TransientUnitSpec {
        unit_name: unit_name.to_owned(),
        properties: vec![
            (
                "Description",
                UnitProperty::Text("HydraCache 0.74 non-product role-overhead fixture".to_owned()),
            ),
            ("User", UnitProperty::Text("hydracache-perf".to_owned())),
            ("Group", UnitProperty::Text("hydracache-perf".to_owned())),
            ("Type", UnitProperty::Text("exec".to_owned())),
            ("RemainAfterExit", UnitProperty::Boolean(true)),
            ("Restart", UnitProperty::Text("no".to_owned())),
            ("KillMode", UnitProperty::Text("control-group".to_owned())),
            ("Slice", UnitProperty::Text("system.slice".to_owned())),
            ("IOAccounting", UnitProperty::Boolean(true)),
            (
                "CPUAffinity",
                UnitProperty::Bytes(cpuset_mask(cpuset).map_err(|error| error.to_string())?),
            ),
            ("Nice", UnitProperty::Signed(0)),
            ("RuntimeMaxUSec", UnitProperty::Unsigned(10_000_000)),
            ("TimeoutStopUSec", UnitProperty::Unsigned(2_000_000)),
            ("MemoryMax", UnitProperty::Unsigned(67_108_864)),
            ("LimitNOFILE", UnitProperty::Unsigned(64)),
            ("TasksMax", UnitProperty::Unsigned(4)),
            ("NoNewPrivileges", UnitProperty::Boolean(true)),
            ("PrivateTmp", UnitProperty::Boolean(true)),
            ("PrivateDevices", UnitProperty::Boolean(true)),
            ("ProtectSystem", UnitProperty::Text("strict".to_owned())),
            ("ProtectHome", UnitProperty::Text("yes".to_owned())),
            ("ProtectKernelTunables", UnitProperty::Boolean(true)),
            ("ProtectKernelModules", UnitProperty::Boolean(true)),
            ("ProtectControlGroups", UnitProperty::Boolean(true)),
            ("RestrictSUIDSGID", UnitProperty::Boolean(true)),
            ("LockPersonality", UnitProperty::Boolean(true)),
            (
                "Environment",
                UnitProperty::Strings(vec![
                    "LANG=C.UTF-8".to_owned(),
                    "LC_ALL=C.UTF-8".to_owned(),
                    "PATH=/usr/bin:/bin".to_owned(),
                    "TZ=UTC".to_owned(),
                ]),
            ),
            (
                "StandardOutputFileToAppend",
                UnitProperty::Text(checkpoint.to_owned()),
            ),
            (
                "StandardErrorFileToAppend",
                UnitProperty::Text(receipt.to_owned()),
            ),
            (
                "ExecStart",
                UnitProperty::Commands(vec![ExecCommand {
                    path: INSTALLED_BINARY.to_owned(),
                    argv: vec![
                        INSTALLED_BINARY.to_owned(),
                        "role-overhead-fixture".to_owned(),
                        role.to_owned(),
                        variant.to_owned(),
                        pair_index.to_string(),
                        position.to_string(),
                        cpuset.to_owned(),
                    ],
                    ignore_failure: false,
                }]),
            ),
        ],
    })
}

fn wait_for_terminal(unit_name: &str) -> Result<(), String> {
    let deadline = Instant::now() + ATTEMPT_TIMEOUT;
    loop {
        let snapshot = inspect_unit_optional(unit_name)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "role-overhead unit disappeared".to_owned())?;
        if snapshot.active_state == "active"
            && snapshot.sub_state == "exited"
            && snapshot.main_pid == 0
            && snapshot.result == "success"
        {
            return Ok(());
        }
        if snapshot.active_state == "failed" || Instant::now() >= deadline {
            return Err(format!("role-overhead unit did not complete: {snapshot:?}"));
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn sample_supervisor() -> Result<SupervisorSample, String> {
    let unit = inspect_supervisor_unit().map_err(|error| error.to_string())?;
    if unit.unit_name != SUPERVISOR_UNIT
        || unit.active_state != "active"
        || unit.sub_state != "running"
        || unit.main_pid == 0
        || unit.control_group != format!("/system.slice/{SUPERVISOR_UNIT}")
        || unit.result != "success"
    {
        return Err("fixed supervisor service is not stably active".to_owned());
    }
    let process = crate::process_identity::inspect_process(unit.main_pid)
        .map_err(|error| error.to_string())?;
    let identity = crate::process_identity::identity_from_snapshot(process, SUPERVISOR_UNIT)
        .map_err(|error| error.to_string())?;
    let cgroup = Path::new("/sys/fs/cgroup").join(unit.control_group.trim_start_matches('/'));
    Ok(SupervisorSample {
        cpu_ns: cgroup_cpu_ns(&cgroup.join("cpu.stat"))?,
        rss_peak_bytes: process_peak_rss_bytes(&PathBuf::from(format!(
            "/proc/{}/status",
            unit.main_pid
        )))?,
        io_bytes: cgroup_io_bytes(&cgroup.join("io.stat"))?,
        identity,
    })
}

fn require_root_and_installed_binary() -> Result<(), String> {
    if unsafe { libc::geteuid() } != 0 {
        return Err("role-overhead coordinator requires root".to_owned());
    }
    let current = fs::canonicalize(std::env::current_exe().map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())?;
    let installed = fs::canonicalize(INSTALLED_BINARY).map_err(|error| error.to_string())?;
    if current != installed {
        return Err("role-overhead coordinator requires the installed binary".to_owned());
    }
    Ok(())
}

fn require_no_active_campaign() -> Result<(), String> {
    match fs::symlink_metadata(ACTIVE_CAMPAIGN) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Ok(_) => Err("host has an active campaign claim".to_owned()),
        Err(error) => Err(error.to_string()),
    }
}

fn prepare_output_directory() -> Result<(), String> {
    let path = Path::new(OUTPUT_DIRECTORY);
    match fs::symlink_metadata(path) {
        Ok(metadata)
            if metadata.is_dir()
                && !metadata.file_type().is_symlink()
                && metadata.uid() == 0
                && metadata.gid() == 0
                && metadata.mode() & 0o7777 == 0o700 =>
        {
            Ok(())
        }
        Ok(_) => Err("role-overhead output directory metadata differs".to_owned()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(path).map_err(|error| error.to_string())?;
            fs::set_permissions(path, std::os::unix::fs::PermissionsExt::from_mode(0o700))
                .map_err(|error| error.to_string())
        }
        Err(error) => Err(error.to_string()),
    }
}

fn output_path(role: &str, pair: u32, position: u32, suffix: &str) -> Result<PathBuf, String> {
    if !matches!(role, "i74" | "c74")
        || !(1..=PAIRS).contains(&pair)
        || !(1..=2).contains(&position)
        || !matches!(suffix, "checkpoint" | "json")
    {
        return Err("role-overhead output identity is invalid".to_owned());
    }
    Ok(Path::new(OUTPUT_DIRECTORY).join(format!("{role}-p{pair}-{position}.{suffix}")))
}

fn prepare_output_file(path: &Path) -> Result<(), String> {
    if path.parent() != Some(Path::new(OUTPUT_DIRECTORY)) {
        return Err("role-overhead output path escaped".to_owned());
    }
    match fs::symlink_metadata(path) {
        Ok(metadata)
            if metadata.is_file()
                && !metadata.file_type().is_symlink()
                && metadata.uid() == 0
                && metadata.gid() == 0 => {}
        Ok(_) => return Err("role-overhead output file metadata differs".to_owned()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.to_string()),
    }
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
        .and_then(|file| file.sync_all())
        .map_err(|error| error.to_string())
}

fn safe_regular_size(path: &Path, maximum: u64) -> Result<u64, String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.uid() != 0
        || metadata.gid() != 0
        || metadata.len() > maximum
    {
        return Err("role-overhead output file is unsafe".to_owned());
    }
    Ok(metadata.len())
}

fn read_bounded(path: &Path, maximum: u64) -> Result<Vec<u8>, String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > maximum {
        return Err(format!("bounded regular file required: {}", path.display()));
    }
    let mut bytes = Vec::with_capacity((metadata.len().min(maximum)) as usize);
    File::open(path)
        .and_then(|file| file.take(maximum + 1).read_to_end(&mut bytes))
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > maximum || (metadata.len() != 0 && bytes.len() as u64 != metadata.len())
    {
        return Err("file changed while it was read".to_owned());
    }
    Ok(bytes)
}

fn read_trimmed(path: &Path, maximum: u64) -> Result<String, String> {
    let bytes = read_bounded(path, maximum)?;
    let value = std::str::from_utf8(&bytes)
        .map_err(|error| error.to_string())?
        .trim()
        .to_owned();
    if value.is_empty() {
        return Err(format!("empty document: {}", path.display()));
    }
    Ok(value)
}

fn read_status_value(path: &Path, name: &str) -> Result<String, String> {
    let document = read_trimmed(path, 128 * 1024)?;
    let prefix = format!("{name}:");
    let values = document
        .lines()
        .filter_map(|line| line.strip_prefix(&prefix).map(str::trim))
        .collect::<Vec<_>>();
    if values.len() != 1 || values[0].is_empty() {
        return Err(format!("status field is absent or ambiguous: {name}"));
    }
    Ok(values[0].to_owned())
}

fn first_allowed_cpu(value: &str) -> Result<String, String> {
    cpuset_mask(value).map_err(|error| error.to_string())?;
    let first = value
        .split(',')
        .next()
        .and_then(|range| range.split('-').next())
        .ok_or_else(|| "allowed CPU list is empty".to_owned())?;
    let cpu = first
        .parse::<u32>()
        .map_err(|_| "allowed CPU list is invalid".to_owned())?;
    if first != cpu.to_string() || cpu >= 4096 {
        return Err("allowed CPU list is not canonical".to_owned());
    }
    Ok(cpu.to_string())
}

fn parse_bounded(value: &str, minimum: u32, maximum: u32) -> Result<u32, String> {
    let parsed = value
        .parse::<u32>()
        .map_err(|_| "bounded integer is invalid".to_owned())?;
    if value != parsed.to_string() || !(minimum..=maximum).contains(&parsed) {
        return Err("bounded integer is outside the admitted range".to_owned());
    }
    Ok(parsed)
}

fn expected_variant(pair: u32, position: u32) -> &'static str {
    match (pair % 2, position) {
        (1, 1) | (0, 2) => "control",
        _ => "instrumented",
    }
}

fn run_operations(operations: u64) {
    let mut value = 0x9e37_79b9_7f4a_7c15_u64;
    for index in 0..operations {
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        value = value.wrapping_add(index);
    }
    std::hint::black_box(value);
}

fn process_cpu_time_ns() -> Result<u64, String> {
    let mut value = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    if unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut value) } != 0
        || value.tv_sec < 0
        || value.tv_nsec < 0
    {
        return Err(std::io::Error::last_os_error().to_string());
    }
    (value.tv_sec as u64)
        .checked_mul(1_000_000_000)
        .and_then(|seconds| seconds.checked_add(value.tv_nsec as u64))
        .ok_or_else(|| "process CPU time overflowed".to_owned())
}

fn process_peak_rss_bytes(path: &Path) -> Result<u64, String> {
    let value = read_status_value(path, "VmHWM")?;
    let mut fields = value.split_ascii_whitespace();
    let kib = fields
        .next()
        .ok_or_else(|| "VmHWM is empty".to_owned())?
        .parse::<u64>()
        .map_err(|_| "VmHWM is invalid".to_owned())?;
    if fields.next() != Some("kB") || fields.next().is_some() || kib == 0 {
        return Err("VmHWM unit is invalid".to_owned());
    }
    kib.checked_mul(1024)
        .ok_or_else(|| "VmHWM overflowed".to_owned())
}

fn proc_io_characters(path: &Path) -> Result<u64, String> {
    let document = read_trimmed(path, 64 * 1024)?;
    let mut value = None;
    for line in document.lines() {
        if let Some(raw) = line.strip_prefix("wchar:") {
            if value.is_some() {
                return Err("process I/O wchar is ambiguous".to_owned());
            }
            value = Some(
                raw.trim()
                    .parse::<u64>()
                    .map_err(|_| "process I/O wchar is invalid".to_owned())?,
            );
        }
    }
    value.ok_or_else(|| "process I/O wchar is absent".to_owned())
}

fn cgroup_cpu_ns(path: &Path) -> Result<u64, String> {
    let document = read_trimmed(path, 64 * 1024)?;
    let mut usage = None;
    for line in document.lines() {
        let fields = line.split_ascii_whitespace().collect::<Vec<_>>();
        if fields.first() == Some(&"usage_usec") {
            if fields.len() != 2 || usage.is_some() {
                return Err("cgroup CPU usage is ambiguous".to_owned());
            }
            usage = Some(
                fields[1]
                    .parse::<u64>()
                    .map_err(|_| "cgroup CPU usage is invalid".to_owned())?,
            );
        }
    }
    usage
        .ok_or_else(|| "cgroup CPU usage is absent".to_owned())?
        .checked_mul(1_000)
        .ok_or_else(|| "cgroup CPU usage overflowed".to_owned())
}

fn cgroup_io_bytes(path: &Path) -> Result<u64, String> {
    let document = match fs::read_to_string(path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error.to_string()),
    };
    let mut total = 0_u64;
    for line in document.lines() {
        for field in line.split_ascii_whitespace().skip(1) {
            let Some((name, raw)) = field.split_once('=') else {
                return Err("cgroup I/O field is invalid".to_owned());
            };
            if matches!(name, "rbytes" | "wbytes") {
                total = total
                    .checked_add(
                        raw.parse::<u64>()
                            .map_err(|_| "cgroup I/O value is invalid".to_owned())?,
                    )
                    .ok_or_else(|| "cgroup I/O total overflowed".to_owned())?;
            }
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::{attempt_spec, expected_variant, first_allowed_cpu, output_path, PAIRS};

    #[test]
    fn schedule_is_exactly_counterbalanced_for_five_pairs() {
        let schedule = (1..=PAIRS)
            .flat_map(|pair| (1..=2).map(move |position| expected_variant(pair, position)))
            .collect::<Vec<_>>();
        assert_eq!(
            schedule,
            vec![
                "control",
                "instrumented",
                "instrumented",
                "control",
                "control",
                "instrumented",
                "instrumented",
                "control",
                "control",
                "instrumented",
            ]
        );
    }

    #[test]
    fn first_allowed_cpu_accepts_canonical_kernel_lists_only() {
        assert_eq!(first_allowed_cpu("2-7,10").unwrap(), "2");
        assert_eq!(first_allowed_cpu("0").unwrap(), "0");
        for invalid in ["", "02", "-1", "2-", "4096", "cpu0"] {
            assert!(first_allowed_cpu(invalid).is_err(), "accepted {invalid}");
        }
    }

    #[test]
    fn attempt_spec_is_fixed_bounded_hardened_and_non_product() {
        let checkpoint = output_path("i74", 1, 1, "checkpoint").unwrap();
        let receipt = output_path("i74", 1, 1, "json").unwrap();
        let spec = attempt_spec(
            "hydracache-performance-074-role-overhead-i74-p1-1.service",
            "i74",
            "control",
            1,
            1,
            "2",
            (&checkpoint, &receipt),
        )
        .unwrap();
        let encoded = format!("{spec:?}");
        for required in [
            "role-overhead-fixture",
            "CPUAffinity",
            "Nice",
            "IOAccounting",
            "NoNewPrivileges",
            "ProtectSystem",
            "PrivateDevices",
        ] {
            assert!(encoded.contains(required), "missing {required}");
        }
        for forbidden in ["/bin/sh", "campaign-lifecycle-fixture", "hydra-bench"] {
            assert!(!encoded.contains(forbidden), "forbidden {forbidden}");
        }
    }
}
