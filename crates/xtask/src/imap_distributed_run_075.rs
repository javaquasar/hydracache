//! Continuous real-daemon diagnostics for the admitted HC/2 subset of W10i.

use std::error::Error;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use hydracache::{PartitionReplicaIdentity, PartitionValuePrefix};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const QUALIFICATION_SECONDS: u64 = 6 * 60 * 60;
const CONFIRMATION_SECONDS: u64 = 24 * 60 * 60;
const MAX_LEDGER_BYTES: u64 = 512 * 1024 * 1024;
const MAX_LEDGER_ROW: u64 = 1024;
const MAX_INITIAL_GAP_MS: u64 = 1_000;
const MAX_INTER_ROUND_GAP_MS: u64 = 20_000;
const MIN_DRAIN_DIAGNOSTIC_SECONDS: u64 = 45;
const MIN_POST_RETURN_MS: u64 = 5_000;
const DRAIN_PROBE_VALUE: &[u8] = b"w10i-live-drain-probe";
static INTERRUPTED_COPY_VALUE: [u8; 96 * 1024] = [0x5a; 96 * 1024];
const MIN_COPY_HOLD_MS: u64 = 1_500;
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
    live_drain: bool,
    interrupt_copy: bool,
}

fn parse(args: Vec<String>, mode: Mode) -> RunResult<Options> {
    let mut diagnostic = false;
    let mut live_drain = false;
    let mut interrupt_copy = false;
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
            "--live-drain" if !live_drain => {
                live_drain = true;
                continue;
            }
            "--interrupt-copy" if !interrupt_copy => {
                interrupt_copy = true;
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
    let live_drain = live_drain || interrupt_copy;
    if live_drain && seconds < MIN_DRAIN_DIAGNOSTIC_SECONDS {
        return Err(
            "W10i --live-drain requires at least 45 seconds for both faults and recovery".into(),
        );
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
        live_drain,
        interrupt_copy,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    drain: Option<Drain>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Fault {
    killed_pid: u32,
    killed_at_ms: u64,
    recovered_at_ms: u64,
    rejoined_pid: u32,
    rejoined_at_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    audit: Option<OwnerKillAudit>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OwnerKillAudit {
    partition: u32,
    before: ReplicaCheckpoint,
    promoted: ReplicaCheckpoint,
    recovered: ReplicaCheckpoint,
    rejoined: ReplicaCheckpoint,
    tombstoned: ReplicaCheckpoint,
    returned_member: PartitionReplicaIdentity,
    returned_epoch: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ReplicaCheckpoint {
    placement: [CopyObservation; 2],
    digests: [ReplicaDigest; 2],
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Drain {
    departed_pid: u32,
    returned_pid: u32,
    started_at_ms: u64,
    completed_at_ms: u64,
    audited_at_ms: u64,
    returned_at_ms: u64,
    partition: u32,
    certified_version: u64,
    value_sha256: String,
    before: [CopyObservation; 2],
    after: [CopyObservation; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    replica_digests: Option<ReplicaReconciliation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    interrupted_copy: Option<InterruptedCopy>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ReplicaReconciliation {
    before: [ReplicaDigest; 2],
    after: [ReplicaDigest; 2],
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ReplicaDigest {
    schema_version: u16,
    member: PartitionReplicaIdentity,
    topology_epoch: u64,
    primary: bool,
    prefix: PartitionValuePrefix,
    image_sha256: [u8; 32],
    rows: usize,
    retained_outcomes: usize,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct InterruptedCopy {
    hit_at_ms: u64,
    observed_at_ms: u64,
    resumed_at_ms: u64,
    blocked_chunk: u32,
    forbidden_restart_chunk: u32,
    held_placement: [CopyObservation; 3],
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CopyObservation {
    member_slot: u8,
    epoch: u64,
    certified_version: u64,
    primary: bool,
    backup: bool,
}

#[derive(Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DrainProgress {
    verified_during_drain: u64,
    verified_after_return: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    verified_while_copy_interrupted: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Published {
    worker: Receipt,
    log_sha256: String,
    qualification_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    drain_progress: Option<DrainProgress>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Round {
    sequence: u64,
    elapsed_ms: u64,
    #[serde(default)]
    verified_at_ms: Option<u64>,
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
        .map(|directory| read_prior(directory, &source, opts.live_drain, opts.interrupt_copy))
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
        .env(
            "HYDRACACHE_W10I_LIVE_DRAIN",
            if opts.live_drain { "1" } else { "0" },
        )
        .env(
            "HYDRACACHE_W10I_INTERRUPT_COPY",
            if opts.interrupt_copy { "1" } else { "0" },
        )
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
    let drain_progress = verify(
        &worker,
        &directory,
        &source,
        opts.seconds,
        opts.live_drain,
        opts.interrupt_copy,
    )?;
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
        drain_progress,
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

fn read_prior(
    directory: &Path,
    source: &str,
    live_drain: bool,
    interrupt_copy: bool,
) -> RunResult<(String, Published)> {
    let path = directory.join("report.json");
    let report: Published = read_json(&path)?;
    if report.qualification_sha256.is_some()
        || report.log_sha256 != digest_file(&directory.join("cargo.log"), 8 * 1024 * 1024)?
    {
        return Err("invalid W10i qualification parent evidence".into());
    }
    if verify(
        &report.worker,
        directory,
        source,
        QUALIFICATION_SECONDS,
        live_drain,
        interrupt_copy,
    )? != report.drain_progress
    {
        return Err("W10i qualification parent's drain progress does not match its ledger".into());
    }
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

fn verify(
    receipt: &Receipt,
    directory: &Path,
    source: &str,
    seconds: u64,
    live_drain: bool,
    interrupt_copy: bool,
) -> RunResult<Option<DrainProgress>> {
    let expected_ms = seconds * 1_000;
    let fault = &receipt.fault;
    let schema = 6 + u16::from(live_drain) + u16::from(interrupt_copy);
    if receipt.schema_version != schema
        || receipt.drain.is_some() != live_drain
        || receipt
            .drain
            .as_ref()
            .is_some_and(|drain| drain.interrupted_copy.is_some())
            != interrupt_copy
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
    let audit = fault
        .audit
        .as_ref()
        .ok_or("missing W10i owner-kill replica audit")?;
    verify_owner_kill_audit(audit)?;
    if let Some(drain) = &receipt.drain {
        verify_drain(drain, fault, expected_ms)?;
        verify_replica_reconciliation(drain)?;
    }
    let path = directory.join("ledger.jsonl");
    if receipt.ledger_sha256 != digest_file(&path, MAX_LEDGER_BYTES)? {
        return Err("W10i ledger digest does not match the worker receipt".into());
    }
    let (counts, progress) = verify_ledger(
        BufReader::new(File::open(path)?),
        expected_ms,
        receipt.drain.as_ref(),
    )?;
    if counts != receipt.counts {
        return Err("W10i ledger counts do not match the worker receipt".into());
    }
    Ok(progress)
}

fn verify_owner_kill_audit(audit: &OwnerKillAudit) -> RunResult<()> {
    for (checkpoint, slots, version) in [
        (&audit.before, [0, 1], 2),
        (&audit.promoted, [1, 2], 2),
        (&audit.recovered, [1, 2], 3),
        (&audit.rejoined, [1, 2], 3),
        (&audit.tombstoned, [1, 2], 4),
    ] {
        let [primary, backup] = &checkpoint.digests;
        if audit.partition >= 271
            || primary.member.node == backup.member.node
            || primary.topology_epoch != backup.topology_epoch
            || !same_replica_image(primary, backup)
            || checkpoint
                .placement
                .iter()
                .zip(&checkpoint.digests)
                .zip(slots)
                .enumerate()
                .any(|(index, ((copy, digest), slot))| {
                    copy.member_slot != slot
                        || copy.certified_version != version
                        || copy.primary != (index == 0)
                        || copy.backup == copy.primary
                        || !valid_replica_digest(digest, copy, audit.partition)
                        || (digest.rows, digest.retained_outcomes) != (1, 1)
                })
        {
            return Err("invalid W10i owner-kill replica checkpoint".into());
        }
    }
    let [old, old_backup] = &audit.before.digests;
    let [promoted, new_backup] = &audit.promoted.digests;
    let recovered = &audit.recovered.digests[0];
    let rejoined = &audit.rejoined.digests[0];
    let tombstoned = &audit.tombstoned.digests[0];
    if old_backup.member != promoted.member
        || old.member.node == new_backup.member.node
        || old.topology_epoch >= promoted.topology_epoch
        || !same_replica_image(old, promoted)
        || audit.returned_member.node != old.member.node
        || audit.returned_member.generation <= old.member.generation
        || promoted.topology_epoch > recovered.topology_epoch
        || recovered.topology_epoch >= audit.returned_epoch
        || audit.returned_epoch > rejoined.topology_epoch
        || rejoined.topology_epoch > tombstoned.topology_epoch
        || !same_replica_image(recovered, rejoined)
        || !advances_certified_image(promoted, recovered)
        || !advances_certified_image(rejoined, tombstoned)
        || [&audit.recovered, &audit.rejoined, &audit.tombstoned]
            .iter()
            .any(|cut| {
                cut.digests[0].member != promoted.member
                    || cut.digests[1].member != new_backup.member
            })
    {
        return Err(
            "W10i owner-kill replica audits do not prove preservation, lineage and rejoin".into(),
        );
    }
    Ok(())
}

fn advances_certified_image(previous: &ReplicaDigest, next: &ReplicaDigest) -> bool {
    next.prefix.previous == previous.prefix.digest
        && next.prefix.epoch.value() >= previous.topology_epoch
        && next.image_sha256 != previous.image_sha256
}

fn valid_replica_digest(digest: &ReplicaDigest, copy: &CopyObservation, partition: u32) -> bool {
    digest.schema_version == 1
        && !digest.member.node.as_str().is_empty()
        && digest.member.generation.value() != 0
        && digest.topology_epoch != 0
        && digest.topology_epoch == copy.epoch
        && digest.primary == copy.primary
        && digest.prefix.partition.value() == partition
        && digest.prefix.version == copy.certified_version
        && (1..=digest.topology_epoch).contains(&digest.prefix.epoch.value())
        && digest.image_sha256 != [0; 32]
        && (1..=4096).contains(&digest.rows)
        && (1..=128).contains(&digest.retained_outcomes)
}

fn same_replica_image(a: &ReplicaDigest, b: &ReplicaDigest) -> bool {
    (a.prefix, a.image_sha256, a.rows, a.retained_outcomes)
        == (b.prefix, b.image_sha256, b.rows, b.retained_outcomes)
}

fn verify_drain(drain: &Drain, fault: &Fault, expected_ms: u64) -> RunResult<()> {
    let [old_primary, old_backup] = &drain.before;
    let [survivor_one, survivor_two] = &drain.after;
    let interrupted = drain.interrupted_copy.is_some();
    let value = if interrupted {
        INTERRUPTED_COPY_VALUE.as_slice()
    } else {
        DRAIN_PROBE_VALUE
    };
    let valid_copy = |copy: &CopyObservation| {
        copy.certified_version == drain.certified_version && copy.primary != copy.backup
    };
    if drain.departed_pid != fault.rejoined_pid
        || drain.returned_pid == 0
        || drain.returned_pid == drain.departed_pid
        || drain.returned_pid == fault.killed_pid
        || drain.started_at_ms < expected_ms / 2
        || drain.started_at_ms <= fault.rejoined_at_ms
        || drain.started_at_ms >= drain.completed_at_ms
        || drain.completed_at_ms > drain.audited_at_ms
        || drain.audited_at_ms >= drain.returned_at_ms
        || drain.returned_at_ms > expected_ms.saturating_sub(MIN_POST_RETURN_MS)
        || drain.partition >= 271
        || drain.certified_version == 0
        || drain.value_sha256 != crate::imap_qualification_075::digest(value)
        || !drain.before.iter().chain(&drain.after).all(valid_copy)
        || old_primary.member_slot != 0
        || !old_primary.primary
        || old_primary.epoch == 0
        || old_backup.member_slot != if interrupted { 1 } else { 2 }
        || !old_backup.backup
        || old_backup.epoch != old_primary.epoch
        || survivor_one.member_slot != 1
        || survivor_two.member_slot != 2
        || survivor_one.epoch != survivor_two.epoch
        || survivor_one.epoch <= old_primary.epoch
        || survivor_one.primary == survivor_two.primary
    {
        return Err("invalid W10i live drain sequence or certified placement".into());
    }
    if let Some(copy) = &drain.interrupted_copy {
        verify_interrupted_copy(copy, drain)?;
    }
    Ok(())
}

fn verify_replica_reconciliation(drain: &Drain) -> RunResult<()> {
    let audit = drain
        .replica_digests
        .as_ref()
        .ok_or("missing W10i replica reconciliation")?;
    let expected = &audit.before[0];
    let mut by_slot: [Option<&PartitionReplicaIdentity>; 3] = [None; 3];
    for (digest, placement) in audit
        .before
        .iter()
        .chain(&audit.after)
        .zip(drain.before.iter().chain(&drain.after))
    {
        let slot = usize::from(placement.member_slot);
        if !valid_replica_digest(digest, placement, drain.partition)
            || !same_replica_image(digest, expected)
            || by_slot.iter().enumerate().any(|(index, member)| {
                member.is_some_and(|member| {
                    if index == slot {
                        member != &digest.member
                    } else {
                        member.node == digest.member.node
                    }
                })
            })
        {
            return Err("W10i replica digests disagree with certified data or placement".into());
        }
        *by_slot
            .get_mut(slot)
            .ok_or("invalid W10i replica member slot")? = Some(&digest.member);
    }
    Ok(())
}

fn verify_interrupted_copy(copy: &InterruptedCopy, drain: &Drain) -> RunResult<()> {
    if copy.blocked_chunk != 1
        || copy.forbidden_restart_chunk != 0
        || copy.hit_at_ms < drain.started_at_ms
        || copy.observed_at_ms.saturating_sub(copy.hit_at_ms) < MIN_COPY_HOLD_MS
        || copy.observed_at_ms > copy.resumed_at_ms
        || copy.resumed_at_ms >= drain.completed_at_ms
        || !drain.after[1].primary
        || drain.after[0].epoch <= copy.held_placement[0].epoch
        || !copy
            .held_placement
            .iter()
            .enumerate()
            .all(|(slot, observation)| {
                usize::from(observation.member_slot) == slot
                    && observation.certified_version == drain.certified_version
                    && Some(observation.epoch) == drain.before[0].epoch.checked_add(1)
                    && observation.primary == (slot == 0)
                    && observation.backup == (slot == 1)
            })
    {
        return Err("invalid W10i interrupted snapshot copy or premature ownership change".into());
    }
    Ok(())
}

fn verify_ledger(
    mut input: impl BufRead,
    expected_ms: u64,
    drain: Option<&Drain>,
) -> RunResult<(Counts, Option<DrainProgress>)> {
    let mut counts = Counts::default();
    let mut progress = drain.map(|drain| DrainProgress {
        verified_while_copy_interrupted: drain.interrupted_copy.as_ref().map(|_| 0),
        ..Default::default()
    });
    let mut last = 0;
    let mut last_verified = 0;
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
        if let Some((drain, progress)) = drain.zip(progress.as_mut()) {
            let completed = row
                .verified_at_ms
                .ok_or("W10i live drain ledger lacks a round completion time")?;
            if row.elapsed_ms < last_verified
                || completed < row.elapsed_ms
                || completed > expected_ms + 60_000
            {
                return Err("W10i live drain ledger has invalid round completion times".into());
            }
            last_verified = completed;
            if row.elapsed_ms >= drain.started_at_ms && completed <= drain.completed_at_ms {
                progress.verified_during_drain += ack;
            }
            if row.elapsed_ms >= drain.returned_at_ms && completed <= expected_ms {
                progress.verified_after_return += ack;
            }
            if let Some((copy, verified)) = drain
                .interrupted_copy
                .as_ref()
                .zip(progress.verified_while_copy_interrupted.as_mut())
            {
                if row.elapsed_ms >= copy.hit_at_ms && completed <= copy.observed_at_ms {
                    *verified += ack;
                }
            }
        } else if row.verified_at_ms.is_some() {
            return Err("unexpected W10i live drain timing in the original ledger profile".into());
        }
        counts.acknowledgements += ack;
        counts.verified += ack;
        counts.unknown += row.unknown;
    }
    if counts.acknowledgements == 0
        || expected_ms - last > MAX_INTER_ROUND_GAP_MS.min(expected_ms / 2)
    {
        return Err("W10i ledger lacks verified progress through the scheduled end".into());
    }
    if progress
        .as_ref()
        .is_some_and(|p| p.verified_during_drain == 0 || p.verified_after_return == 0)
    {
        return Err(
            "W10i ledger must prove verified traffic during drain and after the member returned"
                .into(),
        );
    }
    if progress
        .as_ref()
        .is_some_and(|p| p.verified_while_copy_interrupted == Some(0))
    {
        return Err(
            "W10i ledger lacks verified traffic while the snapshot copy was interrupted".into(),
        );
    }
    Ok((counts, progress))
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
        let copy = args(&[
            "--release",
            "0.75",
            "--duration",
            "60s",
            "--diagnostic",
            "--interrupt-copy",
            "--output",
            "out",
        ]);
        let options = parse(copy, Mode::Qualification).unwrap();
        assert!(options.interrupt_copy && options.live_drain);
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
        for (duration, accepted) in [("40s", false), ("45s", true), ("60s", true), ("6h", true)] {
            let request = args(&[
                "--release",
                "0.75",
                "--duration",
                duration,
                "--diagnostic",
                "--live-drain",
                "--output",
                "out",
            ]);
            assert_eq!(
                parse(request, Mode::Qualification).is_ok_and(|options| options.live_drain),
                accepted
            );
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
            verify_ledger(ledger.as_slice(), 10_000, None).unwrap().0,
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
            verify_ledger(rows(&[row(0, 0), partial.clone()]).as_slice(), 10_000, None)
                .unwrap()
                .0
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
                verify_ledger(rows(&[row(0, 0), tampered]).as_slice(), 10_000, None).is_err(),
                "{field}"
            );
        }
        assert!(verify_ledger(&ledger[..ledger.len() - 1], 10_000, None).is_err());
        assert!(
            verify_ledger(rows(&[row(0, 0), row(1, 49_999)]).as_slice(), 50_000, None).is_err()
        );
    }

    #[test]
    fn evidence_binds_source_duration_child_identities_and_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ledger.jsonl");
        fs::write(&path, rows(&[row(0, 0), row(1, 9_900)])).unwrap();
        let mut worker = Receipt {
            schema_version: 6,
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
                audit: Some(owner_kill_fixture()),
            },
            drain: None,
            counts: Counts {
                rounds: 2,
                acknowledgements: 6,
                verified: 6,
                unknown: 0,
            },
        };
        let previous = serde_json::from_value(serde_json::to_value(&worker).unwrap()).unwrap();
        verify(&worker, dir.path(), &worker.source_sha, 10, false, false).unwrap();
        let audit = worker.fault.audit.take();
        assert!(verify(&worker, dir.path(), &worker.source_sha, 10, false, false).is_err());
        worker.fault.audit = audit;
        match_candidate(&previous, &worker).unwrap();
        worker.runner_sha256 = "d".repeat(64);
        assert!(match_candidate(&previous, &worker).is_err());
        assert!(verify(&worker, dir.path(), "another-source", 10, false, false).is_err());
        assert!(verify(&worker, dir.path(), &worker.source_sha, 60, false, false).is_err());
        worker.fault.rejoined_pid = worker.fault.killed_pid;
        assert!(verify(&worker, dir.path(), &worker.source_sha, 10, false, false).is_err());
        worker.fault.rejoined_pid = 43;
        worker.fault.rejoined_at_ms = 10_001;
        assert!(verify(&worker, dir.path(), &worker.source_sha, 10, false, false).is_err());
        worker.fault.rejoined_at_ms = 5_000;
        worker.production_admitted = true;
        assert!(verify(&worker, dir.path(), &worker.source_sha, 10, false, false).is_err());
        worker.production_admitted = false;
        fs::write(&path, rows(&[row(0, 0), row(1, 9_950)])).unwrap();
        assert!(verify(&worker, dir.path(), &worker.source_sha, 10, false, false).is_err());
    }

    fn owner_kill_fixture() -> OwnerKillAudit {
        let member = |slot: u8, generation| PartitionReplicaIdentity {
            node: hydracache::ClusterNodeId::new(format!("member-{slot}")),
            generation: hydracache::ClusterGeneration::new(generation),
        };
        let partition = 11;
        let cut = |slots: [u8; 2], epoch: u64, version: u8, cert_epoch| ReplicaCheckpoint {
            placement: std::array::from_fn(|index| CopyObservation {
                member_slot: slots[index],
                epoch,
                certified_version: u64::from(version),
                primary: index == 0,
                backup: index == 1,
            }),
            digests: std::array::from_fn(|index| ReplicaDigest {
                schema_version: 1,
                member: member(slots[index], 1),
                topology_epoch: epoch,
                primary: index == 0,
                prefix: PartitionValuePrefix {
                    partition: hydracache::PartitionId::new(partition),
                    epoch: hydracache::ClusterEpoch::new(cert_epoch),
                    version: u64::from(version),
                    previous: [version - 1; 32],
                    digest: [version; 32],
                },
                image_sha256: [version + 10; 32],
                rows: 1,
                retained_outcomes: 1,
            }),
        };
        OwnerKillAudit {
            partition,
            before: cut([0, 1], 4, 2, 4),
            promoted: cut([1, 2], 6, 2, 4),
            recovered: cut([1, 2], 6, 3, 6),
            rejoined: cut([1, 2], 7, 3, 6),
            tombstoned: cut([1, 2], 7, 4, 7),
            returned_member: member(0, 2),
            returned_epoch: 7,
        }
    }

    #[test]
    fn owner_kill_rejects_matching_pairs_with_broken_lineage_or_process_identity() {
        verify_owner_kill_audit(&owner_kill_fixture()).unwrap();
        let changes: [fn(&mut OwnerKillAudit); 8] = [
            |audit| audit.promoted.digests[1].image_sha256[0] ^= 1,
            |audit| {
                audit
                    .promoted
                    .digests
                    .iter_mut()
                    .for_each(|proof| proof.image_sha256[0] ^= 1)
            },
            |audit| {
                audit
                    .recovered
                    .digests
                    .iter_mut()
                    .for_each(|proof| proof.prefix.previous[0] ^= 1)
            },
            |audit| {
                audit
                    .tombstoned
                    .digests
                    .iter_mut()
                    .for_each(|proof| proof.prefix.previous[0] ^= 1)
            },
            |audit| {
                audit
                    .rejoined
                    .digests
                    .iter_mut()
                    .for_each(|proof| proof.retained_outcomes = 0)
            },
            |audit| audit.returned_member.generation = audit.before.digests[0].member.generation,
            |audit| audit.returned_epoch = audit.recovered.digests[0].topology_epoch,
            |audit| {
                audit.promoted.digests[0].member.generation = hydracache::ClusterGeneration::new(2)
            },
        ];
        for change in changes {
            let mut audit = owner_kill_fixture();
            change(&mut audit);
            assert!(verify_owner_kill_audit(&audit).is_err());
        }
    }

    #[test]
    fn live_drain_requires_certified_placement_and_verified_traffic_in_both_windows() {
        let copy = |member_slot, epoch, primary| CopyObservation {
            member_slot,
            epoch,
            certified_version: 5,
            primary,
            backup: !primary,
        };
        let mut drain = Drain {
            departed_pid: 43,
            returned_pid: 44,
            started_at_ms: 30_000,
            completed_at_ms: 34_000,
            audited_at_ms: 35_000,
            returned_at_ms: 45_000,
            partition: 11,
            certified_version: 5,
            value_sha256: crate::imap_qualification_075::digest(DRAIN_PROBE_VALUE),
            before: [copy(0, 7, true), copy(2, 7, false)],
            after: [copy(1, 8, true), copy(2, 8, false)],
            replica_digests: None,
            interrupted_copy: None,
        };
        let fault = Fault {
            killed_pid: 42,
            killed_at_ms: 20_000,
            recovered_at_ms: 24_000,
            rejoined_pid: 43,
            rejoined_at_ms: 25_000,
            audit: None,
        };
        verify_drain(&drain, &fault, 60_000).unwrap();
        assert!(verify_replica_reconciliation(&drain).is_err());
        let partition = drain.partition;
        let digest = |copy: &CopyObservation| ReplicaDigest {
            schema_version: 1,
            member: PartitionReplicaIdentity {
                node: hydracache::ClusterNodeId::new(format!("member-{}", copy.member_slot)),
                generation: hydracache::ClusterGeneration::new(2),
            },
            topology_epoch: copy.epoch,
            primary: copy.primary,
            prefix: PartitionValuePrefix {
                partition: hydracache::PartitionId::new(partition),
                epoch: hydracache::ClusterEpoch::new(7),
                version: copy.certified_version,
                previous: [2; 32],
                digest: [3; 32],
            },
            image_sha256: [4; 32],
            rows: 1,
            retained_outcomes: 1,
        };
        let reconciliation = || ReplicaReconciliation {
            before: [&drain.before[0], &drain.before[1]].map(digest),
            after: [&drain.after[0], &drain.after[1]].map(digest),
        };
        let mut reconciled = reconciliation();
        drain.replica_digests = Some(reconciliation());
        verify_replica_reconciliation(&drain).unwrap();
        let original = serde_json::to_value(&reconciled).unwrap();
        let tamper: [fn(&mut ReplicaDigest); 7] = [
            |proof| proof.image_sha256[0] ^= 1,
            |proof| proof.prefix.previous[0] ^= 1,
            |proof| proof.topology_epoch -= 1,
            |proof| proof.primary = !proof.primary,
            |proof| proof.rows += 1,
            |proof| proof.member.generation = hydracache::ClusterGeneration::new(3),
            |proof| proof.member.node = hydracache::ClusterNodeId::new("member-1"),
        ];
        for change in tamper {
            change(&mut reconciled.after[1]);
            drain.replica_digests = Some(reconciled);
            assert!(verify_replica_reconciliation(&drain).is_err());
            reconciled = serde_json::from_value(original.clone()).unwrap();
        }
        drain.after[1].certified_version -= 1;
        assert!(verify_drain(&drain, &fault, 60_000).is_err());
        drain.after[1].certified_version += 1;
        drain.after[1].epoch -= 1;
        assert!(verify_drain(&drain, &fault, 60_000).is_err());
        drain.after[1].epoch += 1;
        drain.returned_pid = 42;
        assert!(verify_drain(&drain, &fault, 60_000).is_err());
        drain.returned_pid = 44;

        let mut ledger: Vec<_> = [0, 15_000, 30_500, 41_000, 58_000]
            .into_iter()
            .enumerate()
            .map(|(seq, time)| {
                let mut entry = row(seq as u64, time);
                entry["verified_at_ms"] = json!(time + 100);
                entry
            })
            .collect();
        let check = |ledger: &[Value]| verify_ledger(rows(ledger).as_slice(), 60_000, Some(&drain));
        let (_, progress) = check(&ledger).unwrap();
        assert_eq!(
            progress.unwrap(),
            DrainProgress {
                verified_during_drain: 3,
                verified_after_return: 3,
                verified_while_copy_interrupted: None,
            }
        );
        ledger[2]["verified_at_ms"] = json!(34_001);
        assert!(check(&ledger).is_err());
        ledger[2]["verified_at_ms"] = json!(30_600);
        ledger[4]["verified_at_ms"] = json!(60_001);
        assert!(check(&ledger).is_err());
        ledger[4]["verified_at_ms"] = Value::Null;
        assert!(check(&ledger).is_err());
        assert!(verify_ledger(rows(&ledger).as_slice(), 60_000, None).is_err());
    }

    #[test]
    fn interrupted_copy_requires_old_placement_and_progress_before_resume() {
        let copy = |member_slot, epoch, primary, backup| CopyObservation {
            member_slot,
            epoch,
            certified_version: 5,
            primary,
            backup,
        };
        let mut drain = Drain {
            departed_pid: 43,
            returned_pid: 44,
            started_at_ms: 30_000,
            completed_at_ms: 35_000,
            audited_at_ms: 36_000,
            returned_at_ms: 44_000,
            partition: 11,
            certified_version: 5,
            value_sha256: crate::imap_qualification_075::digest(&INTERRUPTED_COPY_VALUE),
            before: [copy(0, 7, true, false), copy(1, 7, false, true)],
            after: [copy(1, 11, false, true), copy(2, 11, true, false)],
            replica_digests: None,
            interrupted_copy: Some(InterruptedCopy {
                hit_at_ms: 30_100,
                observed_at_ms: 32_000,
                resumed_at_ms: 32_000,
                blocked_chunk: 1,
                forbidden_restart_chunk: 0,
                held_placement: [
                    copy(0, 8, true, false),
                    copy(1, 8, false, true),
                    copy(2, 8, false, false),
                ],
            }),
        };
        let fault = Fault {
            killed_pid: 42,
            killed_at_ms: 20_000,
            recovered_at_ms: 24_000,
            rejoined_pid: 43,
            rejoined_at_ms: 25_000,
            audit: None,
        };
        verify_drain(&drain, &fault, 60_000).unwrap();
        drain.interrupted_copy.as_mut().unwrap().held_placement[2].primary = true;
        assert!(verify_drain(&drain, &fault, 60_000).is_err());
        drain.interrupted_copy.as_mut().unwrap().held_placement[2].primary = false;
        drain
            .interrupted_copy
            .as_mut()
            .unwrap()
            .forbidden_restart_chunk = 1;
        assert!(verify_drain(&drain, &fault, 60_000).is_err());
        drain
            .interrupted_copy
            .as_mut()
            .unwrap()
            .forbidden_restart_chunk = 0;
        drain.interrupted_copy.as_mut().unwrap().observed_at_ms = 31_000;
        assert!(verify_drain(&drain, &fault, 60_000).is_err());
        drain.interrupted_copy.as_mut().unwrap().observed_at_ms = 32_000;

        let mut ledger: Vec<_> = [0, 15_000, 30_500, 45_000, 59_000]
            .into_iter()
            .enumerate()
            .map(|(seq, time)| {
                let mut entry = row(seq as u64, time);
                entry["verified_at_ms"] = json!(time + 100);
                entry
            })
            .collect();
        let check = |ledger: &[Value]| verify_ledger(rows(ledger).as_slice(), 60_000, Some(&drain));
        assert_eq!(
            check(&ledger)
                .unwrap()
                .1
                .unwrap()
                .verified_while_copy_interrupted,
            Some(3)
        );
        ledger[2]["verified_at_ms"] = json!(32_001);
        assert!(check(&ledger)
            .unwrap_err()
            .to_string()
            .contains("while the snapshot copy was interrupted"));
    }
}
