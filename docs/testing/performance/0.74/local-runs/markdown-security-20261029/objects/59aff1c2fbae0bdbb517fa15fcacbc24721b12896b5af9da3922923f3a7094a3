use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

const SCHEMA_VERSION: u32 = 1;
const PAIRS: u32 = 5;
const OPERATIONS: u64 = 100_000_000;
const WARMUP_OPERATIONS: u64 = 1_000_000;
const CHECKPOINT_BYTES: usize = 4_096;
const FIXED_DELAY: Duration = Duration::from_millis(300);

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

/// Execute only the bounded, deterministic, non-product fixture. The external
/// unprivileged collector owns scheduling, guard observation and evidence.
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

fn read_bounded(path: &Path, maximum: u64) -> Result<Vec<u8>, String> {
    let metadata = path.symlink_metadata().map_err(|error| error.to_string())?;
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

fn read_status_value(path: &Path, name: &str) -> Result<String, String> {
    let bytes = read_bounded(path, 128 * 1024)?;
    let document = std::str::from_utf8(&bytes).map_err(|error| error.to_string())?;
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
    if value.is_empty() || value.len() > 256 || value.bytes().any(|byte| byte.is_ascii_whitespace())
    {
        return Err("allowed CPU list is invalid".to_owned());
    }
    let mut first = None;
    let mut previous_end = None;
    for range in value.split(',') {
        let (start, end) = match range.split_once('-') {
            Some((start, end)) if !end.contains('-') => (start, end),
            None => (range, range),
            _ => return Err("allowed CPU list is invalid".to_owned()),
        };
        let start_value = start
            .parse::<u32>()
            .map_err(|_| "allowed CPU list is invalid".to_owned())?;
        let end_value = end
            .parse::<u32>()
            .map_err(|_| "allowed CPU list is invalid".to_owned())?;
        if start != start_value.to_string()
            || end != end_value.to_string()
            || start_value > end_value
            || end_value >= 4096
            || previous_end.is_some_and(|previous| start_value <= previous)
        {
            return Err("allowed CPU list is not canonical".to_owned());
        }
        first.get_or_insert(start_value);
        previous_end = Some(end_value);
    }
    first
        .map(|cpu| cpu.to_string())
        .ok_or_else(|| "allowed CPU list is empty".to_owned())
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
    let bytes = read_bounded(path, 64 * 1024)?;
    let document = std::str::from_utf8(&bytes).map_err(|error| error.to_string())?;
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

#[cfg(test)]
mod tests {
    use super::{expected_variant, first_allowed_cpu, PAIRS};

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
    fn first_allowed_cpu_accepts_only_bounded_canonical_first_cpu() {
        assert_eq!(first_allowed_cpu("2-7,10").unwrap(), "2");
        assert_eq!(first_allowed_cpu("0").unwrap(), "0");
        for invalid in ["", "02", "-1", "2-", "4096", "cpu0"] {
            assert!(first_allowed_cpu(invalid).is_err(), "accepted {invalid}");
        }
    }
}
