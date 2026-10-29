//! Executes the independently implemented Java and Rust local IMap services on one trace.

use sha2::{Digest, Sha256};
use std::error::Error;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use hydracache_cluster_testkit::value_plane_concurrent_075::{
    execute_concurrent_rust, generated_schedule, parse_histories, shrink_schedule,
    validate_histories, ConcurrentHistory, ConcurrentSchedule, MAX_SCHEDULE_BYTES,
};
use hydracache_cluster_testkit::value_plane_differential_075::{
    execute_rust, first_difference, generated_trace, shrink_trace, ApiTrace, DifferentialError,
    TraceDifference, MAX_TRACE_BYTES, MAX_TRACE_STEPS,
};

const MAX_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
const MAX_STDERR_BYTES: usize = 64 * 1024;
const PROCESS_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DifferentialReport {
    pub seed: u64,
    pub steps: usize,
    pub comparisons: usize,
    pub canary_detected: bool,
    pub concurrent_rounds: usize,
    pub concurrent_canary_detected: bool,
    pub extended_steps: usize,
    pub extended_process_canaries: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConcurrentReport {
    pub seed: u64,
    pub rounds: usize,
    pub canary_detected: bool,
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let (seed, steps) = parse_args(&args)?;
    let root = crate::doc_check::find_repo_root()?;
    prepare_java_at_root(&root)?;
    let report = check_actual_at_root(&root, seed, steps)?;
    println!("imap-api-differential 0.75: OK (seed={}, {} v1 observations, {} legal concurrent histories per language, {} common v2 steps; sequential/concurrent and {} extended process canaries detected)", report.seed, report.comparisons, report.concurrent_rounds, report.extended_steps, report.extended_process_canaries);
    Ok(())
}

pub fn prepare_java_at_root(root: &Path) -> Result<(), Box<dyn Error>> {
    let maven = if cfg!(windows) { "mvn.cmd" } else { "mvn" };
    let status = Command::new(maven)
        .args(["-B", "-f", "tests/java-imap-benchmark/pom.xml", "test"])
        .current_dir(root)
        .status()?;
    if !status.success() {
        return Err("Java semantic build failed before differential execution".into());
    }
    Ok(())
}

/// The shared correctness gate builds Java first; this helper never silently substitutes a fake.
pub fn check_actual_at_root(
    root: &Path,
    seed: u64,
    random_steps: usize,
) -> Result<DifferentialReport, Box<dyn Error>> {
    let trace = generated_trace(seed, random_steps)?;
    let rust = execute_rust(&trace)?;
    let java = execute_java(root, &trace, false)?;
    if let Some(difference) = first_difference(&trace, &rust, &java) {
        let minimized = shrink_trace(&trace, 16, |candidate| {
            let rust = execute_rust(candidate)?;
            let java = execute_java(root, candidate, false)
                .map_err(|error| DifferentialError(error.to_string()))?;
            Ok(first_difference(candidate, &rust, &java).is_some())
        })?;
        let path = retain_failure_trace(root, &minimized)?;
        let preview = minimized
            .encode()
            .lines()
            .take(17)
            .map(str::to_owned)
            .collect::<Vec<_>>()
            .join("\n");
        let preview = bounded_preview(&preview, 8_192);
        return Err(format!("Rust/Java IMap mismatch {}\nshrunk trace: {} steps; retained at {}\npreview (at most 16 operations):\n{}", render_difference(&difference), minimized.steps.len(), path.display(), preview).into());
    }
    let canary = execute_java(root, &trace, true)?;
    if first_difference(&trace, &rust, &canary).is_none() {
        return Err("differential canary failed: Java dropped-set defect escaped detection".into());
    }
    let concurrent = check_concurrent_at_root(root, seed, 16)?;
    let extended = crate::imap_extended_differential_075::check_actual(root, seed as i64)?;
    Ok(DifferentialReport {
        seed,
        steps: trace.steps.len(),
        comparisons: rust.len(),
        canary_detected: true,
        concurrent_rounds: concurrent.rounds,
        concurrent_canary_detected: concurrent.canary_detected,
        extended_steps: extended.steps,
        extended_process_canaries: extended.process_canaries,
    })
}

/// Different overlap winners are permitted, but every independent history must be legal.
pub fn check_concurrent_at_root(
    root: &Path,
    seed: u64,
    random_rounds: usize,
) -> Result<ConcurrentReport, Box<dyn Error>> {
    let schedule = generated_schedule(seed, random_rounds)?;
    let rust = execute_concurrent_rust(&schedule)?;
    let java = execute_java_concurrent(root, &schedule, false)?;
    let failure = validate_histories(&schedule, &rust)
        .err()
        .map(|error| format!("Rust: {error}"))
        .or_else(|| {
            validate_histories(&schedule, &java)
                .err()
                .map(|error| format!("Java: {error}"))
        });
    if let Some(failure) = failure {
        let minimized = shrink_schedule(&schedule, 16, |candidate| {
            let rust = execute_concurrent_rust(candidate)?;
            let java = execute_java_concurrent(root, candidate, false)
                .map_err(|error| DifferentialError(error.to_string()))?;
            Ok(validate_histories(candidate, &rust).is_err()
                || validate_histories(candidate, &java).is_err())
        })?;
        let path = retain_failure_schedule(root, &minimized)?;
        return Err(format!("illegal concurrent IMap history ({failure}); seed={seed}; minimized {} rounds retained at {}\n{}", minimized.rounds.len(), path.display(), bounded_preview(&minimized.encode(), 4096)).into());
    }
    let broken_java = execute_java_concurrent(root, &schedule, true)?;
    if validate_histories(&schedule, &broken_java).is_ok() {
        return Err("concurrent canary failed: Java unconditional-compare defect escaped legal-history checker".into());
    }
    Ok(ConcurrentReport {
        seed,
        rounds: schedule.rounds.len(),
        canary_detected: true,
    })
}

pub fn execute_java_concurrent(
    root: &Path,
    schedule: &ConcurrentSchedule,
    defect: bool,
) -> Result<Vec<ConcurrentHistory>, Box<dyn Error>> {
    let input = schedule.encode();
    ConcurrentSchedule::parse(&input)?;
    if input.len() > MAX_SCHEDULE_BYTES {
        return Err("Java concurrent input exceeds bound".into());
    }
    let output = execute_java_peer(
        root,
        "IMapConcurrentRunner",
        input,
        defect.then_some("--canary-unconditional-compare"),
    )?;
    Ok(parse_histories(&output, schedule.rounds.len())?)
}

pub fn execute_java(
    root: &Path,
    trace: &ApiTrace,
    defect: bool,
) -> Result<Vec<String>, Box<dyn Error>> {
    let input = trace.encode();
    if input.len() > MAX_TRACE_BYTES || trace.steps.len() > MAX_TRACE_STEPS {
        return Err("Java differential input exceeds bound".into());
    }
    let output = execute_java_peer(
        root,
        "IMapTraceRunner",
        input,
        defect.then_some("--canary-drop-set"),
    )?;
    parse_observations(&output, trace.steps.len())
}

fn execute_java_peer(
    root: &Path,
    runner: &str,
    input: String,
    defect: Option<&str>,
) -> Result<Vec<u8>, Box<dyn Error>> {
    execute_java_bounded(
        root,
        runner,
        input,
        &defect.into_iter().collect::<Vec<_>>(),
        MAX_OUTPUT_BYTES,
    )
}

pub(crate) fn execute_java_bounded(
    root: &Path,
    runner: &str,
    input: String,
    arguments: &[&str],
    maximum: usize,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let classpath = root.join("tests/java-imap-benchmark/target/classes");
    if !classpath
        .join(format!("io/hydracache/imap/semantic/{runner}.class"))
        .is_file()
    {
        return Err(
            format!("compiled {runner} is missing; run the Java semantic build first").into(),
        );
    }
    let java = std::env::var_os("JAVA_HOME")
        .map(|path| {
            std::path::PathBuf::from(path)
                .join("bin")
                .join(if cfg!(windows) { "java.exe" } else { "java" })
        })
        .filter(|path| path.is_file())
        .unwrap_or_else(|| "java".into());
    let mut command = Command::new(java);
    command
        .arg("-Xmx128m")
        .arg("-cp")
        .arg(classpath)
        .arg(format!("io.hydracache.imap.semantic.{runner}"));
    command.args(arguments).current_dir(root);
    execute_process_bounded(command, input, maximum)
}

pub(crate) fn execute_process_bounded(
    mut command: Command,
    input: String,
    maximum: usize,
) -> Result<Vec<u8>, Box<dyn Error>> {
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn()?;
    let stdout = child.stdout.take().ok_or("missing Java stdout")?;
    let stderr = child.stderr.take().ok_or("missing Java stderr")?;
    let out_thread = thread::spawn(move || bounded_read(stdout, maximum));
    let err_thread = thread::spawn(move || bounded_read(stderr, MAX_STDERR_BYTES));
    let mut stdin = child.stdin.take().ok_or("missing Java stdin")?;
    let in_thread = thread::spawn(move || stdin.write_all(input.as_bytes()));
    let started = Instant::now();
    let (status, timed_out) = loop {
        if let Some(status) = child.try_wait()? {
            break (status, false);
        }
        if started.elapsed() >= PROCESS_TIMEOUT {
            child.kill()?;
            break (child.wait()?, true);
        }
        thread::sleep(Duration::from_millis(5));
    };
    let output = out_thread.join().map_err(|_| "Java output reader panicked");
    let errors = err_thread.join().map_err(|_| "Java stderr reader panicked");
    let input = in_thread.join().map_err(|_| "Java input writer panicked");
    if timed_out {
        return Err("Java differential process exceeded 15-second bound".into());
    }
    let output = output??;
    let errors = errors??;
    input??;
    if !status.success() {
        return Err(format!(
            "Java differential process failed ({status}): {}",
            String::from_utf8_lossy(&errors)
        )
        .into());
    }
    Ok(output)
}

fn bounded_read(source: impl Read, maximum: usize) -> Result<Vec<u8>, std::io::Error> {
    let mut output = Vec::new();
    source.take(maximum as u64 + 1).read_to_end(&mut output)?;
    if output.len() > maximum {
        return Err(std::io::Error::other(
            "Java differential output exceeds bound",
        ));
    }
    Ok(output)
}

pub fn parse_observations(output: &[u8], expected: usize) -> Result<Vec<String>, Box<dyn Error>> {
    if output.len() > MAX_OUTPUT_BYTES || !output.is_ascii() {
        return Err("invalid or oversized Java differential output".into());
    }
    let text = std::str::from_utf8(output)?;
    let lines = text.lines().map(str::to_owned).collect::<Vec<_>>();
    if lines.len() != expected
        || lines
            .iter()
            .any(|line| line.is_empty() || line.len() > 64 * 1024 || line.contains('\t'))
    {
        return Err("Java differential returned missing, extra or oversized observations".into());
    }
    Ok(lines)
}

fn render_difference(difference: &TraceDifference) -> String {
    format!(
        "at step {} ({})\nRust: {}\nJava: {}",
        difference.step,
        difference.operation,
        bounded_preview(&difference.rust, 2_048),
        bounded_preview(&difference.java, 2_048)
    )
}

fn bounded_preview(value: &str, maximum: usize) -> String {
    let mut result = value.chars().take(maximum).collect::<String>();
    if value.chars().count() > maximum {
        result.push_str("<truncated>");
    }
    result
}

pub fn retain_failure_trace(
    root: &Path,
    trace: &ApiTrace,
) -> Result<std::path::PathBuf, Box<dyn Error>> {
    let source = trace.encode();
    ApiTrace::parse(&source)?;
    retain_source(root, &source, &format!("seed-{}", trace.seed))
}

pub fn retain_failure_schedule(
    root: &Path,
    schedule: &ConcurrentSchedule,
) -> Result<std::path::PathBuf, Box<dyn Error>> {
    let source = schedule.encode();
    ConcurrentSchedule::parse(&source)?;
    retain_source(root, &source, &format!("schedule-seed-{}", schedule.seed))
}

pub(crate) fn retain_source(
    root: &Path,
    source: &str,
    prefix: &str,
) -> Result<std::path::PathBuf, Box<dyn Error>> {
    let digest = Sha256::digest(source.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let directory = root.join("target/imap-075/differential-failures");
    std::fs::create_dir_all(&directory)?;
    let path = directory.join(format!("{prefix}-{digest}.tsv"));
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(mut file) => {
            file.write_all(source.as_bytes())?;
            file.sync_all()?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if std::fs::read(&path)? != source.as_bytes() {
                return Err(
                    "existing differential failure artifact differs; refusing overwrite".into(),
                );
            }
        }
        Err(error) => return Err(error.into()),
    }
    Ok(path)
}

pub fn parse_args(args: &[String]) -> Result<(u64, usize), Box<dyn Error>> {
    let mut release = None;
    let mut seed = 117;
    let mut steps = 128;
    let mut seen = std::collections::BTreeSet::new();
    let mut fields = args.chunks_exact(2);
    for pair in &mut fields {
        if !seen.insert(pair[0].as_str()) {
            return Err("duplicate differential option".into());
        }
        match pair[0].as_str() {
            "--release" => release = Some(pair[1].as_str()),
            "--seed" => seed = pair[1].parse()?,
            "--steps" => steps = pair[1].parse()?,
            _ => return Err("unknown differential option".into()),
        }
    }
    if !fields.remainder().is_empty() || release != Some("0.75") {
        return Err("imap-api-differential requires --release 0.75 and valued options".into());
    }
    generated_trace(seed, steps)?;
    Ok((seed, steps))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn subprocess_readers_bound_output_before_unbounded_retention() {
        assert_eq!(bounded_read("small".as_bytes(), 5).unwrap(), b"small");
        assert!(bounded_read("oversized".as_bytes(), 4).is_err());
        assert_eq!(bounded_preview("same", 4), "same");
        assert_eq!(bounded_preview("longer", 4), "long<truncated>");
    }
}
