use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;

const DOMAIN: &[u8] = b"hydracache-long-run-record-v1";
const GENESIS: &str = "0000000000000000000000000000000000000000000000000000000000000000";

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn decoded(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn record(sequence: u64, previous: &str, role: &str) -> Value {
    let process = json!({
        "boot_id": "boot-a", "pid": 10, "start_ticks": 20,
        "process_group": 10, "cgroup_path": "/hc/a", "cgroup_inode": 30,
        "unit_name": "hc-a.service"
    });
    let payload = json!({
        "campaign_id": "a".repeat(64), "role": role, "phase": "measured",
        "phase_epoch": 1, "monotonic_elapsed_ns": sequence * 1_000,
        "wall_clock_utc": format!("2026-10-04T00:00:0{sequence}Z"),
        "completed": sequence, "failed": 0, "rejected": 0, "timed_out": 0,
        "outstanding": 0, "telemetry_sequence": sequence, "milestone": "progress",
        "surface_counters": {}, "resource_counters": {}, "owner_counters": {},
        "harness": process, "daemon": process
    });
    let payload_sha256 = hex(&Sha256::digest(serde_json::to_vec(&payload).unwrap()));
    let mut digest = Sha256::new();
    digest.update(DOMAIN);
    digest.update([0]);
    digest.update(sequence.to_be_bytes());
    digest.update(decoded(previous));
    digest.update(decoded(&payload_sha256));
    json!({
        "schema_version": 1, "sequence": sequence,
        "previous_record_sha256": previous, "payload": payload,
        "payload_sha256": payload_sha256, "record_sha256": hex(&digest.finalize())
    })
}

#[test]
fn independent_verifier_accepts_exact_packet_and_rejects_manifest_drift() {
    let directory = tempfile::tempdir().unwrap();
    let first = record(1, GENESIS, "i74");
    let second = record(2, first["record_sha256"].as_str().unwrap(), "i74");
    let journal = directory.path().join("i74.jsonl");
    fs::write(
        &journal,
        format!(
            "{}\n{}\n",
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap()
        ),
    )
    .unwrap();
    let manifest = directory.path().join("packet.json");
    fs::write(
        &manifest,
        serde_json::to_vec_pretty(&json!({
            "schema_version": 1, "release": "0.74", "campaign_id": "a".repeat(64),
            "result": "incomplete", "promotable": false,
            "roles": [{
                "id": "i74", "journal": "i74.jsonl", "expected_records": 2,
                "expected_head_sha256": second["record_sha256"],
                "allow_incomplete_trailing_bytes": false
            }]
        }))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        xtask::long_run_campaign::verify_manifest(&manifest)
            .unwrap()
            .len(),
        1
    );

    let mut changed: Value = serde_json::from_slice(&fs::read(&manifest).unwrap()).unwrap();
    changed["roles"][0]["expected_records"] = json!(3);
    fs::write(&manifest, serde_json::to_vec(&changed).unwrap()).unwrap();
    assert!(xtask::long_run_campaign::verify_manifest(&manifest).is_err());
}

#[test]
fn independent_verifier_rejects_hash_identity_and_traversal_failures() {
    let record = record(1, GENESIS, "i74");
    let mut bytes = serde_json::to_vec(&record).unwrap();
    bytes.push(b'\n');
    assert!(xtask::long_run_campaign::verify_journal_bytes(&bytes).is_ok());

    let mut corrupt: Value = record;
    corrupt["payload"]["completed"] = json!(99);
    let mut corrupt_bytes = serde_json::to_vec(&corrupt).unwrap();
    corrupt_bytes.push(b'\n');
    assert!(xtask::long_run_campaign::verify_journal_bytes(&corrupt_bytes).is_err());

    let directory = tempfile::tempdir().unwrap();
    let manifest = directory.path().join("packet.json");
    fs::write(
        &manifest,
        serde_json::to_vec(&json!({
            "schema_version": 1, "release": "0.74", "campaign_id": "a".repeat(64),
            "result": "incomplete", "promotable": false,
            "roles": [{
                "id": "i74", "journal": "../escape.jsonl", "expected_records": 1,
                "expected_head_sha256": "b".repeat(64),
                "allow_incomplete_trailing_bytes": false
            }]
        }))
        .unwrap(),
    )
    .unwrap();
    assert!(xtask::long_run_campaign::verify_manifest(&manifest).is_err());
}
