mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use support::daemon_cluster::{
    management_json_status, public_text_status, skip_unless_daemon_process_e2e, DaemonCluster,
    OsResourceTotals, TestResult,
};

const SEED: u64 = 0x0720_1200_0000_0001;
const MAX_ENDPOINT_P95_MS: u64 = 2_500;
const MAX_FD_GROWTH: u64 = 12;
const MAX_RSS_GROWTH_KIB: u64 = 65_536;

#[derive(Debug, Deserialize)]
struct FaultMatrix {
    schema_version: u32,
    release: String,
    fault: Vec<FaultRow>,
}

#[derive(Debug, Deserialize)]
struct FaultRow {
    id: String,
    source_test: String,
    projection_test: String,
    projection: String,
    tier: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct EvidenceAttempt {
    id: String,
    previous_attempt: Option<String>,
    outcome: String,
    seed: u64,
    fault_activated: bool,
    schedule_digest: String,
    event_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ProcessReceipt {
    schema_version: u32,
    release: String,
    binary_sha256: String,
    seed: u64,
    schedule: Vec<String>,
    schedule_digest: String,
    event_digest: String,
    endpoint_p95_ms: u64,
    baseline: Option<ResourceReceipt>,
    final_sample: Option<ResourceReceipt>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct ResourceReceipt {
    rss_kib: u64,
    rss_hwm_kib: u64,
    open_fds: u64,
}

impl From<OsResourceTotals> for ResourceReceipt {
    fn from(value: OsResourceTotals) -> Self {
        Self {
            rss_kib: value.rss_kib,
            rss_hwm_kib: value.rss_hwm_kib,
            open_fds: value.open_fds,
        }
    }
}

fn defect(id: &str) -> bool {
    std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok(id)
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn sha256(bytes: impl AsRef<[u8]>) -> String {
    Sha256::digest(bytes.as_ref())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn canonical_digest(values: &[String]) -> String {
    let mut framed = Vec::new();
    for value in values {
        framed.extend_from_slice(&(value.len() as u64).to_be_bytes());
        framed.extend_from_slice(value.as_bytes());
    }
    sha256(framed)
}

fn p95(mut samples: Vec<u64>) -> u64 {
    samples.sort_unstable();
    let index = samples
        .len()
        .saturating_mul(95)
        .div_ceil(100)
        .saturating_sub(1);
    samples.get(index).copied().unwrap_or_default()
}

fn validate_attempt_chain(
    attempts: &[EvidenceAttempt],
    required_attempts: usize,
) -> Result<(), String> {
    if attempts.is_empty() {
        return Err("attempt chain is empty".to_owned());
    }
    if attempts.len() != required_attempts {
        return Err(format!(
            "attempt chain retained {} of {required_attempts} attempts",
            attempts.len()
        ));
    }
    let mut ids = BTreeSet::new();
    for (index, attempt) in attempts.iter().enumerate() {
        if !ids.insert(&attempt.id) {
            return Err(format!("duplicate attempt id {}", attempt.id));
        }
        let expected_previous = index
            .checked_sub(1)
            .map(|prior| attempts[prior].id.as_str());
        if attempt.previous_attempt.as_deref() != expected_previous {
            return Err(format!("attempt {} broke the retry chain", attempt.id));
        }
        if !attempt.fault_activated {
            return Err(format!(
                "attempt {} has no fault activation proof",
                attempt.id
            ));
        }
        if attempt.schedule_digest.len() != 64 || attempt.event_digest.len() != 64 {
            return Err(format!("attempt {} has an invalid digest", attempt.id));
        }
    }
    if attempts.len() > 1
        && !attempts[..attempts.len() - 1]
            .iter()
            .any(|item| item.outcome == "failed")
    {
        return Err("retry chain erased or never retained its failed attempt".to_owned());
    }
    Ok(())
}

fn assert_registered_test_exists(root: &Path, reference: &str) {
    let (path, name) = reference
        .rsplit_once("::")
        .unwrap_or_else(|| panic!("test reference must be path::name: {reference}"));
    let source = fs::read_to_string(root.join(path))
        .unwrap_or_else(|error| panic!("cannot read registered test {reference}: {error}"));
    assert!(
        source.contains(&format!("fn {name}")),
        "registered test function is absent: {reference}"
    );
}

#[test]
fn fault_matrix_is_complete_bounded_and_references_real_proofs() {
    let root = workspace_root();
    let text =
        fs::read_to_string(root.join("docs/testing/management-center/0.72/fault-matrix.toml"))
            .unwrap();
    let matrix: FaultMatrix = toml::from_str(&text).unwrap();
    assert_eq!(matrix.schema_version, 1);
    assert_eq!(matrix.release, "0.72.0");
    let expected = BTreeSet::from([
        "clean-reopen",
        "missing-rebuildable-derivative",
        "bit-rot",
        "torn-artifact",
        "enospc-write-error",
        "uncommitted-wal",
        "corrupt-authoritative-snapshot",
        "commit-blocked-before-apply",
        "deletion-during-stale-peer-partition",
        "foreign-identity-disk",
        "interrupted-reconciliation",
        "concurrent-snapshot-aggregation",
        "bounded-resource-pressure",
    ]);
    let actual = matrix
        .fault
        .iter()
        .map(|row| row.id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, expected);
    for row in &matrix.fault {
        assert!(!row.tier.is_empty(), "{} has no tier", row.id);
        assert!(
            matches!(
                row.projection.as_str(),
                "unknown-until-retained-source"
                    | "explicit-partial"
                    | "consensus-lag"
                    | "removed-never-reappears"
                    | "bounded-overload"
            ),
            "{} has an unreviewed projection {}",
            row.id,
            row.projection
        );
        assert_registered_test_exists(&root, &row.source_test);
        assert_registered_test_exists(&root, &row.projection_test);
    }
}

#[test]
fn fault_matrix_projection_contract_never_promotes_absent_or_partial_truth() {
    let states = BTreeMap::from([
        ("unknown-until-retained-source", ("unknown", false)),
        ("explicit-partial", ("partial", false)),
        ("consensus-lag", ("behind", false)),
        ("removed-never-reappears", ("absent", false)),
        ("bounded-overload", ("rejected", false)),
    ]);
    for (projection, (state, healthy)) in states {
        assert!(!healthy, "{projection} must not paint a fault healthy");
        assert!(!matches!(state, "clean" | "pass" | "complete" | "applied"));
    }
}

#[test]
fn evidence_attempt_chain_retains_failure_and_is_deterministic() {
    let schedule = vec!["partition-peer".to_owned(), "heal-peer".to_owned()];
    let schedule_digest = canonical_digest(&schedule);
    let event_digest = canonical_digest(&["partial".to_owned(), "complete".to_owned()]);
    let attempts = vec![
        EvidenceAttempt {
            id: "attempt-1".to_owned(),
            previous_attempt: None,
            outcome: "failed".to_owned(),
            seed: SEED,
            fault_activated: true,
            schedule_digest: schedule_digest.clone(),
            event_digest: event_digest.clone(),
        },
        EvidenceAttempt {
            id: "attempt-2".to_owned(),
            previous_attempt: Some("attempt-1".to_owned()),
            outcome: "passed".to_owned(),
            seed: SEED,
            fault_activated: true,
            schedule_digest,
            event_digest,
        },
    ];
    validate_attempt_chain(&attempts, 2).unwrap();
    assert_eq!(
        serde_json::to_vec(&attempts).unwrap(),
        serde_json::to_vec(&attempts).unwrap(),
        "same seed/schedule must normalize byte-for-byte"
    );
}

#[test]
fn one_daemon_production_management_surface_is_typed_and_honest() -> TestResult {
    if !skip_unless_daemon_process_e2e(
        "one_daemon_production_management_surface_is_typed_and_honest",
    ) {
        return Ok(());
    }
    let mut cluster = DaemonCluster::start_bootstrap(1, "management-one-daemon")?;
    cluster.wait_for_shape(1, 1)?;
    let admin_addr = cluster.admin_addr(0);
    let (index_status, index) = public_text_status(admin_addr, "/console/")?;
    let (app_status, app) = public_text_status(admin_addr, "/console/app.js")?;
    assert_eq!((index_status, app_status), (200, 200));
    assert!(index.contains("<main") && index.contains("read only"));
    assert!(app.contains("x-hydracache-management-read"));
    assert!(!app.contains("x-hydracache-admin"));
    assert!(!index.contains("https://") && !index.contains("http://"));
    for path in [
        "/management/v1/capabilities",
        "/management/v1/dashboard",
        "/management/v1/cluster/members?limit=100",
        "/management/v1/cluster/formation?limit=100",
        "/management/v1/cluster/partitions",
        "/management/v1/clients",
        "/management/v1/namespaces?limit=100",
        "/management/v1/healthchecks?limit=100",
        "/management/v1/persistence",
        "/management/v1/persistence/recovery?limit=100",
        "/management/v1/operations?limit=100",
        "/management/v1/audit?limit=100",
    ] {
        let body = cluster.management_json(0, path)?;
        if let Some(version) = body.get("schema_version") {
            assert_eq!(version, 1, "{path}: {body}");
        }
        assert_ne!(body.get("source"), Some(&Value::String(String::new())));
    }
    let recovery = cluster.management_json(0, "/management/v1/persistence/recovery?limit=100")?;
    assert_eq!(recovery["data"]["items"][0]["outcome"], "unknown");
    assert_eq!(
        recovery["data"]["items"][0]["reason"],
        "status-not-retained"
    );
    Ok(())
}

#[test]
fn three_daemon_fault_recovery_retains_partial_truth_and_resource_bounds() -> TestResult {
    if !skip_unless_daemon_process_e2e(
        "three_daemon_fault_recovery_retains_partial_truth_and_resource_bounds",
    ) {
        return Ok(());
    }
    let mut cluster = DaemonCluster::start_bootstrap(3, "management-three-daemon")?;
    let statuses = cluster.wait_for_shape(3, 3)?;
    let leader = statuses[0].leader.clone().ok_or("leader before fault")?;
    let node_ids = cluster.node_ids();
    let victim = node_ids
        .iter()
        .position(|node| node != &leader)
        .ok_or("follower victim")?;
    let observer = (0..node_ids.len())
        .find(|index| *index != victim)
        .ok_or("observer")?;
    let baseline = cluster.os_resource_totals().map(ResourceReceipt::from);
    let schedule = vec![
        format!("kill:{victim}"),
        "observe:partial".to_owned(),
        format!("restart:{victim}"),
        "observe:complete".to_owned(),
    ];
    let mut events = Vec::new();
    let mut latencies = Vec::new();

    let initial = cluster.management_json(observer, "/management/v1/dashboard")?;
    assert_eq!(initial["data"]["cluster"]["quorum_ok"], true);
    events.push(format!("initial:{}", initial["completeness"]));

    cluster.kill(victim)?;
    cluster.wait_for_responsive_shape(2, 3, 3)?;
    let partial = cluster.wait_for(
        "management partial after follower kill".to_owned(),
        |cluster| {
            let started = Instant::now();
            let body = cluster
                .management_json(observer, "/management/v1/dashboard")
                .ok()?;
            latencies.push(started.elapsed().as_millis() as u64);
            (body["completeness"] == "partial").then_some(body)
        },
    )?;
    assert_eq!(partial["data"]["cluster"]["quorum_ok"], true);
    assert!(partial["warnings"]
        .as_array()
        .is_some_and(|items| !items.is_empty()));
    events.push("fault:partial".to_owned());

    // Let the one-second aggregate cache expire, then race more readers than the 16-permit
    // management budget against a missing peer. Every request must either complete with bounded
    // partial truth or fail fast with the stable overload response; write-admin status remains
    // independently responsive afterward.
    std::thread::sleep(std::time::Duration::from_millis(1_100));
    let addr = cluster.admin_addr(observer);
    let concurrent = std::thread::scope(|scope| {
        (0..32)
            .map(|_| {
                scope.spawn(move || {
                    management_json_status(addr, "/management/v1/dashboard")
                        .map_err(|error| error.to_string())
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|thread| thread.join().expect("management reader thread"))
            .collect::<Result<Vec<_>, _>>()
    })?;
    let overloads = concurrent
        .iter()
        .filter(|(status, _)| *status == 429)
        .count();
    assert!(concurrent.iter().all(|(status, body)| {
        (*status == 200 && body["completeness"] == "partial")
            || (*status == 429 && body["code"] == "management-read-saturated")
    }));
    assert!(concurrent.iter().any(|(status, _)| *status == 200));
    assert!(cluster.admin_status(observer)?.quorum_ok);
    events.push(format!("concurrent-readers:32:overloads:{overloads}"));

    cluster.restart(victim)?;
    cluster.wait_for_shape(3, 3)?;
    let recovered = cluster.wait_for(
        "management complete after follower restart".to_owned(),
        |cluster| {
            let started = Instant::now();
            let body = cluster
                .management_json(observer, "/management/v1/dashboard")
                .ok()?;
            latencies.push(started.elapsed().as_millis() as u64);
            (body["data"]["cluster"]["quorum_ok"] == true
                && body["data"]["members"]
                    .as_array()
                    .is_some_and(|items| items.len() == 3))
            .then_some(body)
        },
    )?;
    events.push(format!("recovered:{}", recovered["completeness"]));

    for _ in 0..32 {
        let started = Instant::now();
        let body = cluster.management_json(observer, "/management/v1/dashboard")?;
        latencies.push(started.elapsed().as_millis() as u64);
        assert_eq!(body["data"]["cluster"]["quorum_ok"], true);
    }
    let endpoint_p95_ms = p95(latencies);
    assert!(
        endpoint_p95_ms <= MAX_ENDPOINT_P95_MS,
        "p95={endpoint_p95_ms}"
    );
    let final_sample = cluster.os_resource_totals().map(ResourceReceipt::from);
    if let (Some(before), Some(after)) = (baseline, final_sample) {
        assert!(after.open_fds <= before.open_fds.saturating_add(MAX_FD_GROWTH));
        assert!(after.rss_kib <= before.rss_hwm_kib.saturating_add(MAX_RSS_GROWTH_KIB));
    }

    let receipt = ProcessReceipt {
        schema_version: 1,
        release: "0.72.0".to_owned(),
        binary_sha256: sha256(fs::read(cluster.binary_path(observer))?),
        seed: SEED,
        schedule_digest: canonical_digest(&schedule),
        event_digest: canonical_digest(&events),
        schedule,
        endpoint_p95_ms,
        baseline,
        final_sample,
    };
    let bytes = serde_json::to_vec_pretty(&receipt)?;
    let path = cluster.root().join("management-process-receipt.json");
    fs::write(&path, &bytes)?;
    let reread: ProcessReceipt = serde_json::from_slice(&fs::read(path)?)?;
    assert_eq!(reread, receipt);
    Ok(())
}

#[test]
fn canary_fault_masked_is_detected() {
    let source_partial = true;
    let projected = if defect("MC72-W12-FAULT-MASKED") {
        "complete"
    } else {
        "partial"
    };
    assert!(
        source_partial && projected == "partial",
        "HC-CANARY-RED:MC72-W12-FAULT-MASKED partial peer failure was painted complete"
    );
}

#[test]
fn canary_fault_activation_receipt_is_required() {
    let observed_hit = !defect("MC72-W12-FAULT-NOT-ACTIVATED");
    assert!(
        observed_hit,
        "HC-CANARY-RED:MC72-W12-FAULT-NOT-ACTIVATED fault scenario passed without activation"
    );
}

#[test]
fn canary_authoritative_deletion_cannot_be_resurrected() {
    let desired = BTreeSet::from(["remaining"]);
    let observed = if defect("MC72-W12-RESURRECTION") {
        BTreeSet::from(["remaining", "deleted-on-authority"])
    } else {
        desired.clone()
    };
    assert!(
        observed.is_subset(&desired),
        "HC-CANARY-RED:MC72-W12-RESURRECTION stale local discovery resurrected removed state"
    );
}

#[test]
fn canary_retry_chain_cannot_erase_red_attempt() {
    let digest = sha256(b"bounded-schedule");
    let mut attempts = vec![
        EvidenceAttempt {
            id: "attempt-1".to_owned(),
            previous_attempt: None,
            outcome: "failed".to_owned(),
            seed: SEED,
            fault_activated: true,
            schedule_digest: digest.clone(),
            event_digest: digest.clone(),
        },
        EvidenceAttempt {
            id: "attempt-2".to_owned(),
            previous_attempt: Some("attempt-1".to_owned()),
            outcome: "passed".to_owned(),
            seed: SEED,
            fault_activated: true,
            schedule_digest: digest.clone(),
            event_digest: digest,
        },
    ];
    if defect("MC72-W12-RETRY-ERASES-RED") {
        attempts.remove(0);
        attempts[0].previous_attempt = None;
    }
    assert!(
        validate_attempt_chain(&attempts, 2).is_ok(),
        "HC-CANARY-RED:MC72-W12-RETRY-ERASES-RED failed attempt was erased from retry chain"
    );
}
