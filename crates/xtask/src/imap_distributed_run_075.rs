//! Continuous real-daemon diagnostics for the admitted HC/2 subset of W10i.

use std::error::Error;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const QUALIFICATION_SECONDS: u64 = 6 * 60 * 60;
const CONFIRMATION_SECONDS: u64 = 24 * 60 * 60;
const MAX_LEDGER_BYTES: u64 = 512 * 1024 * 1024;
const MAX_LEDGER_ROW: u64 = 1024;
const MAX_INITIAL_GAP_MS: u64 = 1_000;
const MAX_INTER_ROUND_GAP_MS: u64 = 20_000;
const SCENARIO: &str = "distributed_hc2_routes_public_requests_and_recovers_after_owner_kill";

type RunResult<T> = Result<T, Box<dyn Error>>;

#[derive(Clone, Copy)]
pub enum Mode {
    Qualification,
    Confirmation,
}

#[derive(Debug)]
struct Options {
    seconds: u64,
    output: PathBuf,
    prior: Option<PathBuf>,
}

fn parse(args: Vec<String>, mode: Mode) -> RunResult<Options> {
    let mut diagnostic = false;
    let mut release = None;
    let mut duration = None;
    let mut output = None;
    let mut prior = None;
    let mut args = args.into_iter();
    while let Some(option) = args.next() {
        let slot = match option.as_str() {
            "--diagnostic" if !diagnostic => {
                diagnostic = true;
                continue;
            }
            "--release" => &mut release,
            "--duration" => &mut duration,
            "--output" => &mut output,
            "--qualification" => &mut prior,
            _ => return Err(format!("unsupported or duplicate W10i option {option}").into()),
        };
        if slot.is_some() {
            return Err(format!("duplicate W10i option {option}").into());
        }
        *slot = Some(
            args.next()
                .filter(|value| !value.starts_with("--"))
                .ok_or_else(|| format!("missing value for {option}"))?,
        );
    }
    if release.as_deref() != Some("0.75") {
        return Err("W10i requires --release 0.75".into());
    }
    let seconds = match (mode, duration.as_deref()) {
        (Mode::Qualification, Some("6h")) => QUALIFICATION_SECONDS,
        (Mode::Confirmation, Some("24h")) => CONFIRMATION_SECONDS,
        (Mode::Qualification, Some(duration)) => duration
            .strip_suffix('s')
            .and_then(|value| value.parse().ok())
            .filter(|seconds| (10..=60).contains(seconds))
            .ok_or("qualification requires 6h or a 10-60s diagnostic")?,
        _ => return Err("qualification requires 6h and confirmation requires 24h".into()),
    };
    if !diagnostic {
        return Err("W10i production admission requires packaged mixed HC/2 workloads, distributed TTL and listeners, live drain and replica proof, and accepted OD-2/OD-3 bounds; the available runs require --diagnostic".into());
    }
    if matches!(mode, Mode::Confirmation) != prior.is_some() {
        return Err(
            "only confirmation requires --qualification <completed-six-hour-directory>".into(),
        );
    }
    Ok(Options {
        seconds,
        output: output
            .ok_or("W10i requires a fresh --output directory")?
            .into(),
        prior: prior.map(Into::into),
    })
}

#[derive(Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Counts {
    rounds: u64,
    acknowledgements: u64,
    verified: u64,
    unknown: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    schema_version: u16,
    production_admitted: bool,
    source_sha: String,
    seconds: u64,
    elapsed_ms: u64,
    daemon_sha256: String,
    runner_sha256: String,
    ledger_sha256: String,
    fault: Fault,
    counts: Counts,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Fault {
    killed_pid: u32,
    killed_at_ms: u64,
    recovered_at_ms: u64,
    rejoined_pid: u32,
    rejoined_at_ms: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Published {
    worker: Receipt,
    log_sha256: String,
    qualification_sha256: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Round {
    sequence: u64,
    elapsed_ms: u64,
    put_value_sha256: String,
    cas_value_sha256: String,
    put_ack: bool,
    cas_ack: bool,
    delete_ack: bool,
    unknown: u64,
}

pub fn run(args: Vec<String>, mode: Mode) -> RunResult<()> {
    let opts = parse(args, mode)?;
    let root = crate::doc_check::find_repo_root()?;
    let source = crate::imap_ci_075::source_state(&root)?;
    let prior = opts
        .prior
        .as_ref()
        .map(|directory| read_prior(directory, &source))
        .transpose()?;
    fs::create_dir(&opts.output)?;
    let directory = opts.output.canonicalize()?;
    let mut cargo = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    if let Some((_, previous)) = &prior {
        cargo
            .env(
                "HYDRACACHE_W10I_EXPECTED_DAEMON_SHA256",
                &previous.worker.daemon_sha256,
            )
            .env(
                "HYDRACACHE_W10I_EXPECTED_RUNNER_SHA256",
                &previous.worker.runner_sha256,
            );
    }
    let log = new_file(&directory.join("cargo.log"))?;
    let status = cargo
        .current_dir(&root)
        .args([
            "test",
            "--locked",
            "--offline",
            "--release",
            "-p",
            "hydracache-server",
            "--test",
            "hc2_daemon_process",
            "--",
            "--exact",
            SCENARIO,
            "--nocapture",
        ])
        .env("HYDRACACHE_RUN_DAEMON_PROCESS_E2E", "1")
        .env("HYDRACACHE_W10I_RUN_DIR", &directory)
        .env("HYDRACACHE_W10I_SOURCE_SHA", &source)
        .env("HYDRACACHE_W10I_DURATION_SECONDS", opts.seconds.to_string())
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log))
        .status()?;
    if !status.success() {
        return Err(format!(
            "W10i diagnostic failed; see {}",
            directory.join("cargo.log").display()
        )
        .into());
    }
    if crate::imap_ci_075::source_state(&root)? != source {
        return Err("W10i source changed during the diagnostic".into());
    }
    let worker: Receipt = read_json(&directory.join("worker.json"))?;
    verify(&worker, &directory, &source, opts.seconds)?;
    let qualification_sha256 = prior
        .map(|(digest, previous)| {
            match_candidate(&previous.worker, &worker)?;
            Ok::<_, Box<dyn Error>>(digest)
        })
        .transpose()?;
    let published = Published {
        worker,
        log_sha256: digest_file(&directory.join("cargo.log"), 8 * 1024 * 1024)?,
        qualification_sha256,
    };
    let staging = directory.join("report.json.tmp");
    let mut report = new_file(&staging)?;
    serde_json::to_writer_pretty(&mut report, &published)?;
    report.sync_all()?;
    drop(report);
    fs::rename(staging, directory.join("report.json"))?;
    println!(
        "W10i diagnostic complete: {}",
        directory.join("report.json").display()
    );
    Ok(())
}

fn read_prior(directory: &Path, source: &str) -> RunResult<(String, Published)> {
    let path = directory.join("report.json");
    let report: Published = read_json(&path)?;
    if report.qualification_sha256.is_some()
        || report.log_sha256 != digest_file(&directory.join("cargo.log"), 8 * 1024 * 1024)?
    {
        return Err("invalid W10i qualification parent evidence".into());
    }
    verify(&report.worker, directory, source, QUALIFICATION_SECONDS)?;
    Ok((digest_file(&path, 16 * 1024)?, report))
}

fn match_candidate(previous: &Receipt, current: &Receipt) -> RunResult<()> {
    if previous.daemon_sha256 != current.daemon_sha256
        || previous.runner_sha256 != current.runner_sha256
        || previous.source_sha != current.source_sha
    {
        return Err("W10i confirmation requires exactly the qualified source and binaries".into());
    }
    Ok(())
}

fn verify(receipt: &Receipt, directory: &Path, source: &str, seconds: u64) -> RunResult<()> {
    let expected_ms = seconds * 1_000;
    let fault = &receipt.fault;
    if receipt.schema_version != 1
        || receipt.production_admitted
        || receipt.source_sha != source
        || receipt.seconds != seconds
        || !(expected_ms..=expected_ms + 60_000).contains(&receipt.elapsed_ms)
        || !valid_hash(&receipt.daemon_sha256)
        || !valid_hash(&receipt.runner_sha256)
        || fault.killed_pid == 0
        || fault.rejoined_pid == 0
        || fault.killed_pid == fault.rejoined_pid
        || fault.killed_at_ms < expected_ms / 3
        || fault.killed_at_ms >= fault.recovered_at_ms
        || fault.recovered_at_ms >= fault.rejoined_at_ms
        || fault.rejoined_at_ms >= expected_ms
    {
        return Err("invalid W10i worker identity, duration or fault sequence".into());
    }
    let path = directory.join("ledger.jsonl");
    if receipt.ledger_sha256 != digest_file(&path, MAX_LEDGER_BYTES)? {
        return Err("W10i ledger digest does not match the worker receipt".into());
    }
    if verify_ledger(BufReader::new(File::open(path)?), expected_ms)? != receipt.counts {
        return Err("W10i ledger counts do not match the worker receipt".into());
    }
    Ok(())
}

fn verify_ledger(mut input: impl BufRead, expected_ms: u64) -> RunResult<Counts> {
    let mut counts = Counts::default();
    let mut last = 0;
    let mut line = Vec::new();
    loop {
        line.clear();
        if (&mut input)
            .take(MAX_LEDGER_ROW)
            .read_until(b'\n', &mut line)?
            == 0
        {
            break;
        }
        if !line.ends_with(b"\n") {
            return Err("truncated or oversized W10i ledger row".into());
        }
        let row: Round = serde_json::from_slice(&line)?;
        let limit = if counts.rounds == 0 {
            MAX_INITIAL_GAP_MS
        } else {
            MAX_INTER_ROUND_GAP_MS
        };
        if row.sequence != counts.rounds
            || row.elapsed_ms < last
            || row.elapsed_ms >= expected_ms
            || row.elapsed_ms - last > limit
            || (row.cas_ack && !row.put_ack)
            || (row.delete_ack && !row.cas_ack)
            || row.unknown != u64::from(!row.delete_ack)
            || row.put_value_sha256
                != crate::imap_qualification_075::digest(
                    format!("control-put-{}", row.sequence).as_bytes(),
                )
            || row.cas_value_sha256
                != crate::imap_qualification_075::digest(
                    format!("control-cas-{}", row.sequence).as_bytes(),
                )
        {
            return Err(
                "W10i ledger sequence, continuity, operation order or digest is invalid".into(),
            );
        }
        last = row.elapsed_ms;
        counts.rounds += 1;
        let ack = u64::from(row.put_ack) + u64::from(row.cas_ack) + u64::from(row.delete_ack);
        counts.acknowledgements += ack;
        counts.verified += ack;
        counts.unknown += row.unknown;
    }
    if counts.acknowledgements == 0
        || expected_ms - last > MAX_INTER_ROUND_GAP_MS.min(expected_ms / 2)
    {
        return Err("W10i ledger lacks verified progress through the scheduled end".into());
    }
    Ok(counts)
}

fn new_file(path: &Path) -> io::Result<File> {
    OpenOptions::new().write(true).create_new(true).open(path)
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> RunResult<T> {
    let mut data = Vec::new();
    File::open(path)?
        .take(16 * 1024 + 1)
        .read_to_end(&mut data)?;
    if data.len() > 16 * 1024 {
        return Err("W10i receipt exceeds its bound".into());
    }
    Ok(serde_json::from_slice(&data)?)
}

fn digest_file(path: &Path, limit: u64) -> RunResult<String> {
    let file = File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err("W10i evidence must be a regular file".into());
    }
    let mut hash = Sha256::new();
    let mut input = file.take(limit + 1);
    let mut buffer = [0; 16 * 1024];
    loop {
        let n = input.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    if input.limit() == 0 {
        return Err("W10i evidence exceeds its bound".into());
    }
    Ok(hash
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn duration_and_production_gates_are_fail_closed() {
        let run = args(&[
            "--release",
            "0.75",
            "--duration",
            "6h",
            "--diagnostic",
            "--output",
            "out",
        ]);
        assert_eq!(
            parse(run.clone(), Mode::Qualification).unwrap().seconds,
            QUALIFICATION_SECONDS
        );
        assert!(parse(run.clone(), Mode::Confirmation).is_err());
        let without_opt_in = run
            .into_iter()
            .filter(|value| value != "--diagnostic")
            .collect();
        assert!(parse(without_opt_in, Mode::Qualification)
            .unwrap_err()
            .to_string()
            .contains("OD-2/OD-3"));
        let confirm = args(&[
            "--release",
            "0.75",
            "--duration",
            "24h",
            "--diagnostic",
            "--output",
            "out",
        ]);
        assert!(parse(confirm.clone(), Mode::Confirmation).is_err());
        let confirm = confirm
            .into_iter()
            .chain(args(&["--qualification", "previous"]))
            .collect();
        assert_eq!(
            parse(confirm, Mode::Confirmation).unwrap().seconds,
            CONFIRMATION_SECONDS
        );
        for duration in ["2s", "61s", "24h"] {
            let smoke = args(&[
                "--release",
                "0.75",
                "--duration",
                duration,
                "--diagnostic",
                "--output",
                "out",
            ]);
            assert!(parse(smoke, Mode::Qualification).is_err());
        }
    }

    fn row(sequence: u64, elapsed_ms: u64) -> Value {
        json!({
            "sequence": sequence,
            "elapsed_ms": elapsed_ms,
            "put_value_sha256": crate::imap_qualification_075::digest(format!("control-put-{sequence}").as_bytes()),
            "cas_value_sha256": crate::imap_qualification_075::digest(format!("control-cas-{sequence}").as_bytes()),
            "put_ack": true, "cas_ack": true, "delete_ack": true, "unknown": 0
        })
    }

    fn rows(values: &[Value]) -> Vec<u8> {
        values
            .iter()
            .flat_map(|value| format!("{value}\n").into_bytes())
            .collect()
    }

    #[test]
    fn ledger_requires_sequence_end_window_matching_digests_and_order() {
        let ledger = rows(&[row(0, 0), row(1, 9_900)]);
        assert_eq!(
            verify_ledger(ledger.as_slice(), 10_000).unwrap(),
            Counts {
                rounds: 2,
                acknowledgements: 6,
                verified: 6,
                unknown: 0,
            }
        );
        let mut partial = row(1, 9_900);
        partial["cas_ack"] = json!(false);
        partial["delete_ack"] = json!(false);
        partial["unknown"] = json!(1);
        assert_eq!(
            verify_ledger(rows(&[row(0, 0), partial.clone()]).as_slice(), 10_000)
                .unwrap()
                .unknown,
            1
        );
        for (field, bad) in [
            ("sequence", json!(2)),
            ("elapsed_ms", json!(1)),
            ("cas_value_sha256", json!("wrong")),
            ("delete_ack", json!(true)),
            ("unknown", json!(0)),
        ] {
            let mut tampered = partial.clone();
            tampered[field] = bad;
            assert!(
                verify_ledger(rows(&[row(0, 0), tampered]).as_slice(), 10_000).is_err(),
                "{field}"
            );
        }
        assert!(verify_ledger(&ledger[..ledger.len() - 1], 10_000).is_err());
        assert!(verify_ledger(rows(&[row(0, 0), row(1, 49_999)]).as_slice(), 50_000).is_err());
    }

    #[test]
    fn evidence_binds_source_duration_child_identities_and_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ledger.jsonl");
        fs::write(&path, rows(&[row(0, 0), row(1, 9_900)])).unwrap();
        let mut worker = Receipt {
            schema_version: 1,
            production_admitted: false,
            source_sha: "a".repeat(40),
            seconds: 10,
            elapsed_ms: 10_002,
            daemon_sha256: "b".repeat(64),
            runner_sha256: "c".repeat(64),
            ledger_sha256: digest_file(&path, MAX_LEDGER_BYTES).unwrap(),
            fault: Fault {
                killed_pid: 42,
                killed_at_ms: 3_334,
                recovered_at_ms: 4_000,
                rejoined_pid: 43,
                rejoined_at_ms: 5_000,
            },
            counts: Counts {
                rounds: 2,
                acknowledgements: 6,
                verified: 6,
                unknown: 0,
            },
        };
        let previous = serde_json::from_value(serde_json::to_value(&worker).unwrap()).unwrap();
        verify(&worker, dir.path(), &worker.source_sha, 10).unwrap();
        match_candidate(&previous, &worker).unwrap();
        worker.runner_sha256 = "d".repeat(64);
        assert!(match_candidate(&previous, &worker).is_err());
        assert!(verify(&worker, dir.path(), "another-source", 10).is_err());
        assert!(verify(&worker, dir.path(), &worker.source_sha, 60).is_err());
        worker.fault.rejoined_pid = worker.fault.killed_pid;
        assert!(verify(&worker, dir.path(), &worker.source_sha, 10).is_err());
        worker.fault.rejoined_pid = 43;
        worker.fault.rejoined_at_ms = 10_001;
        assert!(verify(&worker, dir.path(), &worker.source_sha, 10).is_err());
        worker.fault.rejoined_at_ms = 5_000;
        worker.production_admitted = true;
        assert!(verify(&worker, dir.path(), &worker.source_sha, 10).is_err());
        worker.production_admitted = false;
        fs::write(&path, rows(&[row(0, 0), row(1, 9_950)])).unwrap();
        assert!(verify(&worker, dir.path(), &worker.source_sha, 10).is_err());
    }
}
