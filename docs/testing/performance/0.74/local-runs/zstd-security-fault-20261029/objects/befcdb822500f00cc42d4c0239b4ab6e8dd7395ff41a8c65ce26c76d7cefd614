use serde::Serialize;
use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::hint::black_box;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier};
use std::time::Instant;

const PROFILE_ID: &str = "w9a-atomic-ordering-screen-074-v1";

#[derive(Debug)]
struct Options {
    source_commit: String,
    operations: u64,
    concurrency: usize,
    pairs: usize,
    output: Option<PathBuf>,
}

impl Options {
    fn parse() -> Result<Self, Box<dyn Error>> {
        let mut values = BTreeMap::new();
        let mut args = std::env::args().skip(1);
        while let Some(name) = args.next() {
            let value = args
                .next()
                .ok_or_else(|| format!("missing value after {name}"))?;
            values.insert(name, value);
        }
        let options = Self {
            source_commit: take_required(&mut values, "--source-commit")?,
            operations: take(&mut values, "--operations", "10000000").parse()?,
            concurrency: take(&mut values, "--concurrency", "1").parse()?,
            pairs: take(&mut values, "--pairs", "7").parse()?,
            output: values.remove("--output").map(PathBuf::from),
        };
        if !values.is_empty() {
            return Err(format!("unknown arguments: {:?}", values.keys()).into());
        }
        if options.source_commit.len() != 40
            || !options
                .source_commit
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("source-commit must be a full Git SHA".into());
        }
        if options.operations == 0
            || options.concurrency == 0
            || options.pairs != 7
            || !options
                .operations
                .is_multiple_of(options.concurrency as u64)
        {
            return Err(
                "operations must divide across non-zero concurrency and pairs must be 7".into(),
            );
        }
        Ok(options)
    }
}

fn take(values: &mut BTreeMap<String, String>, name: &str, default: &str) -> String {
    values.remove(name).unwrap_or_else(|| default.to_owned())
}

fn take_required(
    values: &mut BTreeMap<String, String>,
    name: &str,
) -> Result<String, Box<dyn Error>> {
    values
        .remove(name)
        .ok_or_else(|| format!("{name} is required").into())
}

#[derive(Debug, Clone, Copy, Serialize)]
struct Sample {
    pair: usize,
    order_in_pair: usize,
    ordering: &'static str,
    elapsed_nanoseconds_per_operation: f64,
    cpu_nanoseconds_per_operation: f64,
    final_value: u64,
}

#[derive(Debug, Serialize)]
struct Summary {
    median_seq_cst_cpu_nanoseconds_per_operation: f64,
    median_relaxed_cpu_nanoseconds_per_operation: f64,
    median_cpu_improvement_percent: f64,
    relaxed_faster_pairs: usize,
    minimum_cpu_improvement_percent: f64,
    screen_passed: bool,
}

#[derive(Debug, Serialize)]
struct Receipt {
    schema_version: u32,
    release: &'static str,
    profile_id: &'static str,
    promotable: bool,
    source_commit: String,
    operations_per_sample: u64,
    concurrency: usize,
    pairs: usize,
    order: &'static str,
    samples: Vec<Sample>,
    summary: Summary,
    claim_boundary: &'static str,
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = Options::parse()?;
    let mut samples = Vec::with_capacity(options.pairs * 2);
    run_sample(
        Ordering::SeqCst,
        options.operations / 100,
        options.concurrency,
    )?;
    run_sample(
        Ordering::Relaxed,
        options.operations / 100,
        options.concurrency,
    )?;
    for pair in 1..=options.pairs {
        let orderings = if pair % 2 == 1 {
            [("SeqCst", Ordering::SeqCst), ("Relaxed", Ordering::Relaxed)]
        } else {
            [("Relaxed", Ordering::Relaxed), ("SeqCst", Ordering::SeqCst)]
        };
        for (order_in_pair, (name, ordering)) in orderings.into_iter().enumerate() {
            let (elapsed, cpu, final_value) =
                run_sample(ordering, options.operations, options.concurrency)?;
            samples.push(Sample {
                pair,
                order_in_pair: order_in_pair + 1,
                ordering: name,
                elapsed_nanoseconds_per_operation: elapsed * 1_000_000_000.0
                    / options.operations as f64,
                cpu_nanoseconds_per_operation: cpu * 1_000_000_000.0 / options.operations as f64,
                final_value,
            });
        }
    }
    let seq = samples
        .iter()
        .filter(|sample| sample.ordering == "SeqCst")
        .map(|sample| sample.cpu_nanoseconds_per_operation)
        .collect::<Vec<_>>();
    let relaxed = samples
        .iter()
        .filter(|sample| sample.ordering == "Relaxed")
        .map(|sample| sample.cpu_nanoseconds_per_operation)
        .collect::<Vec<_>>();
    let seq_median = median(seq.clone());
    let relaxed_median = median(relaxed.clone());
    let improvement = (seq_median - relaxed_median) * 100.0 / seq_median;
    let relaxed_faster_pairs = seq
        .iter()
        .zip(&relaxed)
        .filter(|(baseline, candidate)| candidate < baseline)
        .count();
    let receipt = Receipt {
        schema_version: 1,
        release: "0.74",
        profile_id: PROFILE_ID,
        promotable: false,
        source_commit: options.source_commit,
        operations_per_sample: options.operations,
        concurrency: options.concurrency,
        pairs: options.pairs,
        order: "counterbalanced-by-pair",
        samples,
        summary: Summary {
            median_seq_cst_cpu_nanoseconds_per_operation: seq_median,
            median_relaxed_cpu_nanoseconds_per_operation: relaxed_median,
            median_cpu_improvement_percent: improvement,
            relaxed_faster_pairs,
            minimum_cpu_improvement_percent: 2.0,
            screen_passed: improvement >= 2.0 && relaxed_faster_pairs >= 6,
        },
        claim_boundary:
            "isolated atomic screening only; cannot authorize a product change or throughput claim",
    };
    let encoded = serde_json::to_vec_pretty(&receipt)?;
    if let Some(path) = options.output {
        if path.exists() {
            return Err(format!("output already exists: {}", path.display()).into());
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, [&encoded[..], b"\n"].concat())?;
    } else {
        println!("{}", String::from_utf8(encoded)?);
    }
    Ok(())
}

fn run_sample(
    ordering: Ordering,
    operations: u64,
    concurrency: usize,
) -> Result<(f64, f64, u64), Box<dyn Error>> {
    let counter = Arc::new(AtomicU64::new(0));
    let barrier = Arc::new(Barrier::new(concurrency + 1));
    let per_thread = operations / concurrency as u64;
    let mut workers = Vec::with_capacity(concurrency);
    for _ in 0..concurrency {
        let counter = Arc::clone(&counter);
        let barrier = Arc::clone(&barrier);
        workers.push(std::thread::spawn(move || {
            barrier.wait();
            for _ in 0..per_thread {
                black_box(counter.fetch_add(1, ordering));
            }
        }));
    }
    barrier.wait();
    let cpu_before = process_cpu_seconds()?;
    let started = Instant::now();
    for worker in workers {
        worker.join().map_err(|_| "atomic worker panicked")?;
    }
    let elapsed = started.elapsed().as_secs_f64();
    let cpu = (process_cpu_seconds()? - cpu_before).max(0.0);
    let final_value = counter.load(Ordering::Relaxed);
    if final_value != operations {
        return Err(format!("counter mismatch: expected {operations}, got {final_value}").into());
    }
    Ok((elapsed, cpu, final_value))
}

fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

#[cfg(windows)]
fn process_cpu_seconds() -> Result<f64, Box<dyn Error>> {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};

    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    // SAFETY: all FILETIME pointers refer to writable values for the current process.
    if unsafe {
        GetProcessTimes(
            GetCurrentProcess(),
            &mut creation,
            &mut exit,
            &mut kernel,
            &mut user,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    let ticks =
        |value: FILETIME| ((value.dwHighDateTime as u64) << 32) | value.dwLowDateTime as u64;
    Ok((ticks(kernel) + ticks(user)) as f64 / 10_000_000.0)
}

#[cfg(unix)]
fn process_cpu_seconds() -> Result<f64, Box<dyn Error>> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    // SAFETY: getrusage initializes the supplied structure on success.
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: the successful call above initialized `usage`.
    let usage = unsafe { usage.assume_init() };
    let seconds = |time: libc::timeval| time.tv_sec as f64 + time.tv_usec as f64 / 1_000_000.0;
    Ok(seconds(usage.ru_utime) + seconds(usage.ru_stime))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_orderings_preserve_atomic_increment_count() {
        for ordering in [Ordering::SeqCst, Ordering::Relaxed] {
            let (_, _, final_value) = run_sample(ordering, 4_000, 4).unwrap();
            assert_eq!(final_value, 4_000);
        }
    }

    #[test]
    fn median_uses_the_middle_sorted_value() {
        assert_eq!(median(vec![3.0, 1.0, 2.0]), 2.0);
    }
}
