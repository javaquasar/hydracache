//! Launch and independently verify the packaged Rust and Java W10i diagnostic.

use std::ffi::OsString;
use std::ops::Range;

use hydracache_cluster_testkit::w10i_packaged_ledger::{
    planned_rounds, Coverage, Progress, Replay, Windows,
};

use super::*;

const SCENARIO: &str =
    "imap_sdk_cohort::w10i_packaged_sdks_read_during_interrupted_copy_and_after_cutover";
const MAX_INPUT_BYTES: u64 = 512 * 1024 * 1024;
const MAX_LOG_BYTES: u64 = 8 * 1024 * 1024;
const MAX_REPORT_BYTES: u64 = 16 * 1024;
const SDKS: [&str; 2] = ["rust", "java"];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Profile {
    java: PathBuf,
    classpath: Vec<PathBuf>,
    rust_consumer: PathBuf,
    rust_package: PathBuf,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SdkIdentity {
    java_sha256: String,
    classpath_sha256: Vec<String>,
    rust_consumer_sha256: String,
    rust_package_sha256: String,
}

impl Profile {
    fn load(path: &Path) -> RunResult<Self> {
        let mut profile: Self = read_json(path)?;
        if !(1..=64).contains(&profile.classpath.len()) {
            return Err("W10i requires an ordered Java classpath of 1-64 files".into());
        }
        let parent = path
            .canonicalize()?
            .parent()
            .ok_or("invalid W10i SDK profile location")?
            .to_owned();
        for input in [
            &mut profile.java,
            &mut profile.rust_consumer,
            &mut profile.rust_package,
        ]
        .into_iter()
        .chain(profile.classpath.iter_mut())
        {
            *input = parent.join(&*input).canonicalize()?;
            if !input.is_file() {
                return Err("W10i SDK profile inputs must be regular files".into());
            }
        }
        Ok(profile)
    }

    fn identity(&self) -> RunResult<SdkIdentity> {
        let digest = |path: &Path| digest_file(path, MAX_INPUT_BYTES);
        Ok(SdkIdentity {
            java_sha256: digest(&self.java)?,
            classpath_sha256: self
                .classpath
                .iter()
                .map(|path| digest(path))
                .collect::<RunResult<_>>()?,
            rust_consumer_sha256: digest(&self.rust_consumer)?,
            rust_package_sha256: digest(&self.rust_package)?,
        })
    }

    fn classpath(&self) -> RunResult<OsString> {
        Ok(std::env::join_paths(&self.classpath)?)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Worker {
    schema_version: u16,
    production_admitted: bool,
    source_sha: String,
    seconds: u64,
    traffic_elapsed_ms: u64,
    daemon_sha256: String,
    runner_sha256: String,
    rust_package_sha256: String,
    control_sha256: String,
    workload_sha256: String,
    drain_sha256: String,
    killed_pid: u32,
    rejoined_pid: u32,
    rejoined_at_ms: u64,
    partition: u32,
    before: ReplicaCheckpoint,
    promoted: ReplicaCheckpoint,
    outage: Range<u64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DrainReport {
    drain: Drain,
    held_readback: Range<u64>,
    returned_readback: Range<u64>,
    held_verified: u64,
    returned_verified: u64,
    counts: Counts,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SdkProgress {
    sdk: String,
    progress: Progress,
    ledger_sha256: String,
    plan_sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Checkpoint {
    label: String,
    verified: [CheckpointRead; 2],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckpointRead {
    sdk: String,
    keys: u32,
    last_completed_ms: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Workload {
    schema_version: u16,
    sdk: [SdkProgress; 2],
    checkpoints: [Checkpoint; 3],
    watermarks: [[u64; 2]; 4],
    windows: Windows,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Published {
    schema_version: u16,
    production_admitted: bool,
    source_sha: String,
    seconds: u64,
    sdk: SdkIdentity,
    worker_sha256: String,
    cargo_log_sha256: String,
    qualification_sha256: Option<String>,
}

pub(super) fn run(opts: &Options, root: &Path, source: &str, path: &Path) -> RunResult<()> {
    let profile = Profile::load(path)?;
    let sdk = profile.identity()?;
    let classpath = profile.classpath()?;
    let prior = opts
        .prior
        .as_ref()
        .map(|path| read_prior(path, source, &sdk))
        .transpose()?;
    fs::create_dir(&opts.output)?;
    let directory = opts.output.canonicalize()?;
    let mut command = worker_command(root, &directory, source, opts, SCENARIO);
    command
        .env("HYDRACACHE_RUN_W10I_SDK_COHORT", "1")
        .env("HYDRACACHE_W10G_JAVA", &profile.java)
        .env("HYDRACACHE_W10G_JAVA_CLASSPATH", classpath)
        .env("HYDRACACHE_W10I_RUST_CONSUMER", &profile.rust_consumer)
        .env(
            "HYDRACACHE_W10I_RUST_PACKAGE_SHA256",
            &sdk.rust_package_sha256,
        );
    if let Some((_, previous)) = &prior {
        command
            .env(
                "HYDRACACHE_W10I_EXPECTED_DAEMON_SHA256",
                &previous.daemon_sha256,
            )
            .env(
                "HYDRACACHE_W10I_EXPECTED_RUNNER_SHA256",
                &previous.runner_sha256,
            );
    }
    run_worker(command, &directory)?;
    if crate::imap_ci_075::source_state(root)? != source || profile.identity()? != sdk {
        return Err("W10i source or SDK inputs changed during the diagnostic".into());
    }
    let worker = verify(&directory, source, opts.seconds, &sdk)?;
    let qualification_sha256 = prior
        .map(|(digest, previous)| {
            if (previous.daemon_sha256, previous.runner_sha256)
                != (worker.daemon_sha256, worker.runner_sha256)
            {
                return Err(
                    "W10i packaged confirmation requires the qualified daemon and runner binaries",
                );
            }
            Ok(digest)
        })
        .transpose()?;
    let published = Published {
        schema_version: 1,
        production_admitted: false,
        source_sha: source.to_owned(),
        seconds: opts.seconds,
        sdk,
        worker_sha256: digest_file(&directory.join("sdk-worker.json"), MAX_REPORT_BYTES)?,
        cargo_log_sha256: digest_file(&directory.join("cargo.log"), MAX_LOG_BYTES)?,
        qualification_sha256,
    };
    let staging = directory.join("report.json.tmp");
    let mut file = new_file(&staging)?;
    serde_json::to_writer_pretty(&mut file, &published)?;
    file.sync_all()?;
    drop(file);
    let report = directory.join("report.json");
    fs::rename(staging, &report)?;
    println!("W10i packaged diagnostic complete: {}", report.display());
    Ok(())
}

fn read_prior(directory: &Path, source: &str, sdk: &SdkIdentity) -> RunResult<(String, Worker)> {
    let path = directory.join("report.json");
    let previous: Published = read_json(&path)?;
    if previous.schema_version != 1
        || previous.production_admitted
        || previous.seconds != QUALIFICATION_SECONDS
        || previous.source_sha != source
        || &previous.sdk != sdk
        || previous.qualification_sha256.is_some()
        || previous.cargo_log_sha256 != digest_file(&directory.join("cargo.log"), MAX_LOG_BYTES)?
        || previous.worker_sha256
            != digest_file(&directory.join("sdk-worker.json"), MAX_REPORT_BYTES)?
    {
        return Err("invalid W10i packaged six-hour parent or SDK identity".into());
    }
    let worker = verify(directory, source, QUALIFICATION_SECONDS, sdk)?;
    Ok((digest_file(&path, MAX_REPORT_BYTES)?, worker))
}

fn verify(directory: &Path, source: &str, seconds: u64, sdk: &SdkIdentity) -> RunResult<Worker> {
    let worker: Worker = read_json(&directory.join("sdk-worker.json"))?;
    verify_identity(&worker, source, seconds, sdk)?;
    verify_promotion(&worker)?;

    let run = directory.join("sdk-run");
    let drain_path = run.join("sdk-drain.json");
    let workload_path = run.join("packaged-workload.json");
    let control_path = run.join("control-ledger.jsonl");
    if worker.drain_sha256 != digest_file(&drain_path, MAX_REPORT_BYTES)?
        || worker.workload_sha256 != digest_file(&workload_path, MAX_REPORT_BYTES)?
        || worker.control_sha256 != digest_file(&control_path, MAX_LEDGER_BYTES)?
    {
        return Err("W10i packaged evidence digest mismatch".into());
    }
    let report: DrainReport = read_json(&drain_path)?;
    let fault = Fault {
        killed_pid: worker.killed_pid,
        killed_at_ms: worker.outage.start,
        recovered_at_ms: worker.outage.end,
        rejoined_pid: worker.rejoined_pid,
        rejoined_at_ms: worker.rejoined_at_ms,
        audit: None,
    };
    let expected_ms = seconds * 1_000;
    verify_drain(&report.drain, &fault, expected_ms)?;
    verify_replica_reconciliation(&report.drain)?;
    let copy = report
        .drain
        .interrupted_copy
        .as_ref()
        .ok_or("missing W10i packaged interrupted copy")?;
    let windows = Windows {
        outage: worker.outage.clone(),
        held: copy.hit_at_ms..copy.resumed_at_ms,
        returned: report.drain.returned_at_ms..expected_ms,
    };
    if !within(&report.held_readback, &windows.held)
        || !within(&report.returned_readback, &windows.returned)
    {
        return Err("W10i cross-SDK readback missed a required window".into());
    }
    let mut complete_mutations = [0_u64; 3];
    let (counts, _) = scan_ledger(
        BufReader::new(File::open(control_path)?),
        expected_ms,
        Some(&report.drain),
        |row| {
            if let Some(end) = row.verified_at_ms.filter(|_| row.delete_ack) {
                for (count, window) in complete_mutations.iter_mut().zip([
                    &windows.outage,
                    &windows.held,
                    &windows.returned,
                ]) {
                    if window.contains(&row.elapsed_ms) && window.contains(&end) {
                        *count += 3;
                    }
                }
            }
        },
    )?;
    if counts != report.counts
        || complete_mutations.contains(&0)
        || complete_mutations[1..] != [report.held_verified, report.returned_verified]
    {
        return Err("W10i packaged control counts do not match the ledger".into());
    }
    verify_workload(&run, read_json(&workload_path)?, &windows, seconds)?;
    Ok(worker)
}

fn verify_identity(
    worker: &Worker,
    source: &str,
    seconds: u64,
    sdk: &SdkIdentity,
) -> RunResult<()> {
    let expected_ms = seconds * 1_000;
    if worker.schema_version != 1
        || worker.production_admitted
        || worker.source_sha != source
        || worker.seconds != seconds
        || worker.rust_package_sha256 != sdk.rust_package_sha256
        || !valid_hash(&worker.daemon_sha256)
        || !valid_hash(&worker.runner_sha256)
        || !(expected_ms..=expected_ms + 10_000).contains(&worker.traffic_elapsed_ms)
        || worker.killed_pid == 0
        || worker.rejoined_pid == 0
        || worker.killed_pid == worker.rejoined_pid
        || !within(&worker.outage, &(expected_ms / 3..worker.rejoined_at_ms))
        || worker.outage.end >= worker.rejoined_at_ms
        || worker.rejoined_at_ms >= expected_ms
    {
        return Err("invalid W10i packaged worker identity, duration or fault sequence".into());
    }
    Ok(())
}

fn verify_promotion(worker: &Worker) -> RunResult<()> {
    if worker.partition >= 271 {
        return Err("invalid W10i packaged promotion partition".into());
    }
    for (cut, slots) in [(&worker.before, [0, 1]), (&worker.promoted, [1, 2])] {
        if !same_replica_image(&cut.digests[0], &cut.digests[1])
            || cut.digests[0].member.node == cut.digests[1].member.node
            || cut.placement[0].epoch != cut.placement[1].epoch
            || cut
                .placement
                .iter()
                .zip(&cut.digests)
                .zip(slots)
                .enumerate()
                .any(|(index, ((copy, digest), slot))| {
                    copy.member_slot != slot
                        || copy.certified_version != 1
                        || copy.primary != (index == 0)
                        || copy.backup == copy.primary
                        || !valid_replica_digest(digest, copy, worker.partition)
                })
        {
            return Err("invalid W10i packaged replica checkpoint".into());
        }
    }
    let [old, survivor] = &worker.before.digests;
    let [new, backup] = &worker.promoted.digests;
    if old.topology_epoch >= new.topology_epoch
        || !same_replica_image(old, new)
        || survivor.member != new.member
        || old.member.node == backup.member.node
    {
        return Err("W10i packaged promotion did not preserve the original replica lineage".into());
    }
    Ok(())
}

fn within(inner: &Range<u64>, outer: &Range<u64>) -> bool {
    outer.start <= inner.start && inner.start < inner.end && inner.end <= outer.end
}

fn verify_workload(run: &Path, report: Workload, windows: &Windows, seconds: u64) -> RunResult<()> {
    if report.schema_version != 2
        || &report.windows != windows
        || report.watermarks[0].contains(&0)
        || report.watermarks.windows(2).any(|pair| {
            pair[0]
                .iter()
                .zip(pair[1])
                .any(|(previous, next)| *previous >= next)
        })
    {
        return Err("invalid packaged workload windows or subscriber resume chain".into());
    }
    for (checkpoint, label) in
        report
            .checkpoints
            .iter()
            .zip(["promoted", "copy-held", "after-drain"])
    {
        if checkpoint.label != label
            || checkpoint.verified.iter().zip(SDKS).any(|(read, sdk)| {
                read.sdk != sdk
                    || !matches!(read.keys, 2 | 4)
                    || read.last_completed_ms >= windows.returned.end
            })
        {
            return Err("missing packaged reconciliation checkpoint".into());
        }
    }
    let limit = planned_rounds(seconds).ok_or("invalid packaged duration")?;
    for (summary, sdk) in report.sdk.iter().zip(SDKS) {
        let journal = run.join(format!("{sdk}-load.tsv"));
        let plan = run.join(format!("{sdk}-load.plan"));
        if summary.sdk != sdk
            || summary.ledger_sha256 != digest_file(&journal, MAX_INPUT_BYTES)?
            || summary.plan_sha256 != digest_file(&plan, MAX_INPUT_BYTES)?
        {
            return Err("W10i packaged plan or journal digest mismatch".into());
        }
        let reader = |path: &Path| File::open(path).map(BufReader::new);
        let mut coverage = Coverage::default();
        for round in Replay::new(reader(&journal)?, reader(&plan)?, true, limit, sdk) {
            coverage.observe(&round?, windows)?;
        }
        if coverage.finish(windows)? != summary.progress {
            return Err("W10i packaged summary disagrees with its journal".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn profile_binds_file_content_and_classpath_order() {
        let scratch = tempfile::tempdir().unwrap();
        let root = scratch.path();
        for name in ["java", "first.jar", "second.jar", "rust", "sdk.crate"] {
            fs::write(root.join(name), name).unwrap();
        }
        let path = root.join("sdk.json");
        let mut input = json!({
            "java": "java", "classpath": ["first.jar", "second.jar"],
            "rust_consumer": "rust", "rust_package": "sdk.crate"
        });
        fs::write(&path, input.to_string()).unwrap();
        let profile = Profile::load(&path).unwrap();
        let original = profile.identity().unwrap();
        assert_eq!(
            std::env::split_paths(&profile.classpath().unwrap()).count(),
            2
        );
        input["classpath"] = json!(["second.jar", "first.jar"]);
        fs::write(&path, input.to_string()).unwrap();
        assert_ne!(Profile::load(&path).unwrap().identity().unwrap(), original);
        fs::write(root.join("rust"), "new").unwrap();
        assert_ne!(profile.identity().unwrap(), original);
        input["classpath"] = json!(["."]);
        fs::write(&path, input.to_string()).unwrap();
        assert!(Profile::load(&path).is_err());
        input["classpath"] = json!([]);
        fs::write(&path, input.to_string()).unwrap();
        assert!(Profile::load(&path).is_err());
    }

    #[test]
    fn packaged_reports_bind_the_schedule_journals_and_subscriber_chain() {
        let scratch = tempfile::tempdir().unwrap();
        let root = scratch.path();
        let windows = Windows {
            outage: 1_000..2_000,
            held: 2_000..3_000,
            returned: 3_000..4_000,
        };
        let mut sdk = Vec::new();
        for name in SDKS {
            let mut plan = Vec::new();
            let mut journal = Vec::new();
            for (index, start) in [100, 1_000, 2_000, 3_000, 3_900].into_iter().enumerate() {
                use std::io::Write as _;
                let key = |n| {
                    format!("w10i-load/{name}/{n}")
                        .bytes()
                        .map(|byte| format!("{byte:02x}"))
                        .collect::<String>()
                };
                let [a, b] = [key(index * 2), key(index * 2 + 1)];
                writeln!(plan, "{a}\t{b}\t4\t6").unwrap();
                writeln!(journal, "submit\t{index}\t{start}\t{a}\t{b}").unwrap();
                writeln!(journal, "outcome\t{index}\t{}\ta:4:2:1\tr:4:2:1\ta:4:2:2\tv:4:2:2\ta:6:2:1\ta:6:2:2\t0:6:2:2", start + 50).unwrap();
            }
            let plan_path = root.join(format!("{name}-load.plan"));
            let journal_path = root.join(format!("{name}-load.tsv"));
            fs::write(&plan_path, plan).unwrap();
            fs::write(&journal_path, journal).unwrap();
            sdk.push(json!({
                "sdk": name, "ledger_sha256": digest_file(&journal_path, MAX_INPUT_BYTES).unwrap(),
                "plan_sha256": digest_file(&plan_path, MAX_INPUT_BYTES).unwrap(),
                "progress": {
                    "counts": { "confirmed":30, "rejected":5, "not_applied":0, "unknown":0 },
                    "rounds":5, "keys":10, "offline_rounds":1, "held_rounds":1, "post_return_rounds":2,
                }
            }));
        }
        let checkpoints = ["promoted", "copy-held", "after-drain"].map(|label| json!({
            "label": label, "verified": SDKS.map(|name| json!({"sdk": name, "keys": 4, "last_completed_ms": 900}))
        }));
        let report = json!({
            "schema_version": 2, "sdk": sdk, "checkpoints": checkpoints, "windows": windows,
            "watermarks": [[1,1],[2,2],[3,3],[4,4]],
        });
        let verify =
            |report| verify_workload(root, serde_json::from_value(report).unwrap(), &windows, 4);
        verify(report.clone()).unwrap();
        for (path, bad) in [
            ("/watermarks/2/1", json!(2)),
            ("/windows/held/start", json!(2_001)),
            ("/checkpoints/0/label", json!("absent")),
            ("/sdk/0/progress/keys", json!(0)),
        ] {
            let mut altered = report.clone();
            *altered.pointer_mut(path).unwrap() = bad;
            assert!(verify(altered).is_err(), "{path}");
        }
        fs::write(root.join("java-load.tsv"), "missing outcome").unwrap();
        assert!(verify(report).is_err());
    }

    #[test]
    fn short_run_and_changed_inputs_cannot_become_a_confirmation_parent() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("report.json");
        let mut published = Published {
            schema_version: 1,
            production_admitted: false,
            source_sha: "source".into(),
            seconds: 60,
            sdk: SdkIdentity {
                java_sha256: "java".into(),
                classpath_sha256: vec!["jar".into()],
                rust_consumer_sha256: "binary".into(),
                rust_package_sha256: "package".into(),
            },
            worker_sha256: "worker".into(),
            cargo_log_sha256: "log".into(),
            qualification_sha256: None,
        };
        fs::write(&path, serde_json::to_vec(&published).unwrap()).unwrap();
        let result = read_prior(root.path(), "source", &published.sdk);
        assert!(matches!(result, Err(ref err) if err.to_string().contains("six-hour parent")));
        published.seconds = QUALIFICATION_SECONDS;
        fs::write(&path, serde_json::to_vec(&published).unwrap()).unwrap();
        assert!(read_prior(root.path(), "different", &published.sdk).is_err());
        let mut changed = published.sdk;
        changed.classpath_sha256[0] = "different".into();
        assert!(read_prior(root.path(), "source", &changed).is_err());
    }
}
