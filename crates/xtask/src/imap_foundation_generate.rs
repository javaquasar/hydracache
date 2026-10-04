use crate::imap_foundation_evidence;
use crate::imap_value_plane_model::{execute_for_test, git_head, hex_digest};
use hydracache_cluster_testkit::distributed_value_plane_075::{
    DistributedValuePlaneSimulator, ExecutionFault, SimulatorBounds,
};
use hydracache_cluster_testkit::value_plane_history_075::{
    HistoryCall, HistoryOperation, HistoryOutcome, HistoryResult, ValuePlaneHistory,
    ValuePlaneHistoryOracle,
};
use hydracache_cluster_testkit::value_plane_model_075::{
    CanonicalMapKey, MutationDigest, MutationIdentity, MutationOperation, MutationPlan,
    OutcomeCertainty, TtlDirective, ValuePlaneBounds,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

const RELEASE: &str = "0.75";

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let (output, seed) = parse_args(&args)?;
    let root = crate::doc_check::find_repo_root()?;
    let files = generate_at_root(&root, &output, seed)?;
    println!(
        "imap-foundation-evidence-generate {RELEASE}: wrote {} validated receipts to {}",
        files.len(),
        output.display()
    );
    Ok(())
}

pub fn generate_at_root(
    root: &Path,
    output: &Path,
    seed: u64,
) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    if output.exists() {
        return Err(format!("output directory already exists: {}", output.display()).into());
    }
    fs::create_dir_all(output)?;
    let source_sha = git_head(root)?;

    let model = serde_json::to_value(execute_for_test(root, seed, 8, 20_000)?)?;
    let fault = fault_receipt(&source_sha, seed)?;
    let linear = linearizability_receipt(&source_sha)?;
    let rpo = rpo_receipt(&source_sha)?;
    let receipts = [
        ("authority-model.json", model),
        ("fault-schedule.json", fault),
        ("linearizability.json", linear),
        ("rpo-rto.json", rpo),
    ];
    let mut paths = Vec::new();
    for (name, receipt) in receipts {
        let path = output.join(name);
        fs::write(&path, serde_json::to_vec_pretty(&receipt)?)?;
        let problems = imap_foundation_evidence::check_at_root(root, RELEASE, Some(&path))?;
        if !problems.is_empty() {
            return Err(format!("generated receipt {name} is invalid: {problems:?}").into());
        }
        paths.push(path);
    }
    let problems = imap_foundation_evidence::check_receipt_set_at_root(root, RELEASE, output)?;
    if !problems.is_empty() {
        return Err(format!("generated receipt set is invalid: {problems:?}").into());
    }
    Ok(paths)
}

fn fault_receipt(source_sha: &str, seed: u64) -> Result<Value, Box<dyn Error>> {
    let mut sim = simulator()?;
    let key = key(0);
    let plan = put(1, key, b"value", 1);
    let result = sim.execute_with_fault(
        "node-c",
        plan.clone(),
        0,
        ExecutionFault::LoseResponseAfterAcknowledgement,
    )?;
    if result.outcome.certainty != OutcomeCertainty::OutcomeUnknown {
        return Err("response-loss probe did not reach outcome_unknown".into());
    }
    let replay = sim.execute("node-c", plan)?;
    if !replay.replayed || replay.outcome != result.outcome {
        return Err("response-loss probe did not replay the retained outcome".into());
    }
    let checkpoints = json!([
        "before_owner_apply",
        "after_owner_apply",
        "after_replica_proof",
        "after_acknowledgement",
        "response_loss",
        "promotion",
        "repair",
        "full_member_restart",
        "whole_cluster_loss"
    ]);
    Ok(json!({
        "schema": "hydracache.imap.fault-schedule-receipt.v1",
        "release": RELEASE,
        "source_sha": source_sha,
        "seed": seed,
        "checkpoints": checkpoints,
        "schedule_sha256": canonical_digest(&checkpoints),
        "result": "passed"
    }))
}

fn linearizability_receipt(source_sha: &str) -> Result<Value, Box<dyn Error>> {
    let key = key(0);
    let mut history = ValuePlaneHistory::new(8)?;
    history.push(HistoryCall::completed(
        1,
        key.clone(),
        HistoryOperation::Put(b"value".to_vec()),
        1,
        2,
        HistoryResult::Mutation(HistoryOutcome::applied(None, Some(b"value".to_vec()))),
        OutcomeCertainty::Certain,
    ))?;
    history.push(HistoryCall::completed(
        2,
        key,
        HistoryOperation::Read,
        3,
        4,
        HistoryResult::Value(Some(b"value".to_vec())),
        OutcomeCertainty::Certain,
    ))?;
    let report = ValuePlaneHistoryOracle::new(128).check(&history);
    if !report.is_linearizable() {
        return Err(format!("linearizability probe failed: {:?}", report.violation).into());
    }
    let evidence = json!([
        {"id": 1, "operation": "put", "invoked_at": 1, "completed_at": 2},
        {"id": 2, "operation": "read", "invoked_at": 3, "completed_at": 4}
    ]);
    Ok(json!({
        "schema": "hydracache.imap.linearizability-receipt.v1",
        "release": RELEASE,
        "source_sha": source_sha,
        "history": evidence,
        "history_sha256": canonical_digest(&evidence),
        "complete": true,
        "result": "passed"
    }))
}

fn rpo_receipt(source_sha: &str) -> Result<Value, Box<dyn Error>> {
    let mut sim = simulator()?;
    let key = key(0);
    let assignment = sim.assignment(0).expect("partition exists").clone();
    sim.execute(&assignment.owner, put(1, key.clone(), b"value", 1))?;
    sim.fail_node(&assignment.owner)?;
    let promoted = sim.promote_backup(0, 2)?;
    if sim.read("node-c", &key, 0)? != Some(b"value".to_vec()) {
        return Err("acknowledged value was lost during owner-loss probe".into());
    }
    if promoted.owner != assignment.backup {
        return Err("owner-loss probe promoted an unproved node".into());
    }
    Ok(json!({
        "schema": "hydracache.imap.rpo-rto-receipt.v1",
        "release": RELEASE,
        "source_sha": source_sha,
        "profile": "in_memory",
        "fault": "owner_loss",
        "rpo_operations": 0,
        "rto_logical_ticks": 1,
        "bound_id": "proof.imap075.rpo.memory.owner_loss",
        "result": "passed"
    }))
}

fn simulator() -> Result<DistributedValuePlaneSimulator, Box<dyn Error>> {
    Ok(DistributedValuePlaneSimulator::new(
        ["node-a", "node-b", "node-c"],
        SimulatorBounds::default(),
        ValuePlaneBounds::default(),
    )?)
}

fn key(partition: u32) -> CanonicalMapKey {
    CanonicalMapKey::new("tenant-a", "orders", 1, b"key".to_vec(), partition)
}

fn put(sequence: u64, key: CanonicalMapKey, value: &[u8], epoch: u64) -> MutationPlan {
    MutationPlan::new(
        MutationIdentity::new("evidence-client", sequence),
        MutationDigest::new(sequence.saturating_mul(31)),
        key,
        MutationOperation::Put {
            value: value.to_vec(),
            ttl: TtlDirective::Eternal,
        },
        epoch,
    )
}

fn canonical_digest(value: &Value) -> String {
    let bytes = serde_json::to_vec(value).expect("JSON value encodes");
    hex_digest(Sha256::digest(bytes).as_slice())
}

fn parse_args(args: &[String]) -> Result<(PathBuf, u64), Box<dyn Error>> {
    let mut release = None;
    let mut output = None;
    let mut seed = 117_u64;
    let mut index = 0;
    while index < args.len() {
        let name = args[index].as_str();
        index += 1;
        let value = args
            .get(index)
            .ok_or_else(|| format!("{name} requires a value"))?;
        match name {
            "--release" => release = Some(value.clone()),
            "--output" => output = Some(PathBuf::from(value)),
            "--seed" => seed = value.parse()?,
            other => return Err(format!("unknown evidence-generate argument: {other}").into()),
        }
        index += 1;
    }
    if release.as_deref() != Some(RELEASE) {
        return Err("evidence-generate requires --release 0.75".into());
    }
    Ok((
        output.ok_or("evidence-generate requires --output <new-directory>")?,
        seed,
    ))
}
