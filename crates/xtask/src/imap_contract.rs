use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

const RELEASE: &str = "0.75";
const SURFACES: [&str; 5] = ["resp", "hc1", "hc2-rust", "hc2-java", "hazelcast-facade"];
const MUTATION_STAGES: [&str; 10] = [
    "received",
    "admitted",
    "routed",
    "owner_decided",
    "owner_applied",
    "visible_owner",
    "replica_proved",
    "acknowledged",
    "responded",
    "outcome_unknown",
];
const REQUIRED_OPERATIONS: [&str; 16] = [
    "get",
    "put",
    "delete",
    "conditional_replace",
    "conditional_remove",
    "contains_key",
    "put_if_absent",
    "replace_if_present",
    "get_and_remove",
    "get_and_put",
    "get_all",
    "put_all_detailed",
    "remove_all_detailed",
    "set_ttl",
    "remaining_ttl",
    "entry_listener",
];
const FAILURE_SCENARIOS: [&str; 9] = [
    "ready_owner",
    "stale_proxy",
    "missing_required_backup",
    "isolated_old_owner",
    "quorum_loss",
    "promotion",
    "repair",
    "full_member_restart",
    "whole_cluster_loss",
];
const RPO_PROFILES: [&str; 2] = ["in_memory", "durable"];
const RPO_FAULTS: [&str; 3] = [
    "owner_loss",
    "owner_plus_backup_loss",
    "whole_cluster_restart",
];
const REQUIRED_BOUNDS: [&str; 8] = [
    "request_entries",
    "request_bytes",
    "response_bytes",
    "key_bytes",
    "value_bytes",
    "listener_queue_events",
    "history_events",
    "dedup_entries_per_tenant",
];
const REQUIRED_THREATS: [&str; 8] = [
    "tenant_substitution",
    "replay",
    "false_backup_ack",
    "redirect_loop",
    "stale_generation",
    "trust_rotation",
    "audit_redaction",
    "hostile_bounded_decode",
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StatusContract {
    release: String,
    contract_version: u32,
    status: String,
    w0_disposition: String,
    production_capability: String,
    safe_foundation: SafeFoundationStatus,
    blocked_by_074: Vec<BlockedDependency>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SafeFoundationStatus {
    phase: String,
    implemented: Vec<String>,
    deferred: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BlockedDependency {
    id: String,
    required_artifact: String,
    reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OperationContract {
    release: String,
    contract_version: u32,
    status: String,
    result_outcomes: Vec<String>,
    errors: Vec<ErrorContract>,
    ttl_directives: Vec<TtlContract>,
    operations: Vec<Operation>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ErrorContract {
    id: String,
    retry_class: String,
    certainty: String,
    test_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TtlContract {
    id: String,
    zero_duration_allowed: bool,
    semantics: String,
    test_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Operation {
    id: String,
    disposition: String,
    outcomes: Vec<String>,
    errors: Vec<String>,
    ttl_policy: Vec<String>,
    retry_class: String,
    bound_ids: Vec<String>,
    test_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BoundsContract {
    release: String,
    contract_version: u32,
    status: String,
    bounds: Vec<Bound>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Bound {
    id: String,
    unit: String,
    minimum: u64,
    maximum: u64,
    enforcement: String,
    error: String,
    test_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SurfaceContract {
    release: String,
    contract_version: u32,
    status: String,
    surfaces: Vec<Surface>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Surface {
    id: String,
    backend: String,
    key_codec: String,
    value_codec: String,
    ttl_mapping: String,
    response_projection: String,
    event_projection: String,
    cells: Vec<SurfaceCell>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SurfaceCell {
    operation: String,
    disposition: String,
    unsupported_error: Option<String>,
    test_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MutationContract {
    release: String,
    contract_version: u32,
    status: String,
    stages: Vec<MutationStage>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MutationStage {
    id: String,
    reads_may_observe: bool,
    client_retry: String,
    failure_outcome: String,
    promoted_owner_proof_id: String,
    test_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RetryContract {
    release: String,
    contract_version: u32,
    status: String,
    dedup_bound_id: String,
    rules: Vec<RetryRule>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RetryRule {
    id: String,
    idempotency_identity_required: bool,
    ambiguous_outcome: String,
    retry: String,
    terminal_error: String,
    proof_id: String,
    test_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FailureContract {
    release: String,
    contract_version: u32,
    status: String,
    cells: Vec<FailureCell>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FailureCell {
    scenario: String,
    read_availability: String,
    write_availability: String,
    certainty: String,
    consistency: String,
    retry: String,
    readiness: String,
    proof_id: String,
    test_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RpoRtoContract {
    release: String,
    contract_version: u32,
    status: String,
    cells: Vec<RpoRtoCell>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RpoRtoCell {
    profile: String,
    fault: String,
    support: String,
    rpo: String,
    rto_class: String,
    proof_id: String,
    test_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SecurityContract {
    release: String,
    contract_version: u32,
    status: String,
    threats: Vec<Threat>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Threat {
    id: String,
    asset: String,
    boundary: String,
    mitigation: String,
    failure: String,
    bound_or_proof: String,
    test_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CanonicalKeyContract {
    release: String,
    contract_version: u32,
    status: String,
    codec_id: String,
    production_wire_identity: String,
    production_partition_hash: String,
    vectors: Vec<CanonicalKeyVector>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CanonicalKeyVector {
    id: String,
    tenant: String,
    namespace: String,
    namespace_generation: u64,
    key_hex: String,
    encoded_hex: String,
    test_ids: Vec<String>,
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let release = parse_release(&args)?;
    let root = crate::doc_check::find_repo_root()?;
    let problems = check_at_root(&root, &release)?;
    if problems.is_empty() {
        println!("imap-contract-check {release}: OK (provisional W0 foundation)");
        return Ok(());
    }
    for problem in &problems {
        eprintln!("imap-contract-check {release}: {problem}");
    }
    Err(format!("imap-contract-check found {} problem(s)", problems.len()).into())
}

fn parse_release(args: &[String]) -> Result<String, Box<dyn Error>> {
    let mut release = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--release" => {
                index += 1;
                release = Some(
                    args.get(index)
                        .ok_or("imap-contract-check requires a value after --release")?
                        .clone(),
                );
            }
            other => return Err(format!("unknown imap-contract-check argument: {other}").into()),
        }
        index += 1;
    }
    let release = release.ok_or("imap-contract-check requires --release 0.75")?;
    if release != RELEASE {
        return Err(
            format!("imap-contract-check currently supports only release {RELEASE}").into(),
        );
    }
    Ok(release)
}

pub fn check_at_root(root: &Path, release: &str) -> Result<Vec<String>, Box<dyn Error>> {
    if release != RELEASE {
        return Ok(vec![format!(
            "unsupported release {release}; expected {RELEASE}"
        )]);
    }
    check_contract_dir(&root.join("docs/testing/imap/0.75"))
}

pub fn check_contract_dir(dir: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let status: StatusContract = load(dir, "status.json")?;
    let operations: OperationContract = load(dir, "operation-contract.json")?;
    let bounds: BoundsContract = load(dir, "resource-bounds.json")?;
    let surfaces: SurfaceContract = load(dir, "surface-equivalence.json")?;
    let mutation: MutationContract = load(dir, "mutation-stages.json")?;
    let retry: RetryContract = load(dir, "retry-idempotency.json")?;
    let failure: FailureContract = load(dir, "failure-consistency-matrix.json")?;
    let rpo_rto: RpoRtoContract = load(dir, "rpo-rto-contract.json")?;
    let security: SecurityContract = load(dir, "security-contract.json")?;
    let canonical_key: CanonicalKeyContract = load(dir, "canonical-key-vectors.json")?;

    let mut problems = Vec::new();
    validate_headers(
        [
            (
                "status.json",
                &status.release,
                status.contract_version,
                &status.status,
            ),
            (
                "operation-contract.json",
                &operations.release,
                operations.contract_version,
                &operations.status,
            ),
            (
                "resource-bounds.json",
                &bounds.release,
                bounds.contract_version,
                &bounds.status,
            ),
            (
                "surface-equivalence.json",
                &surfaces.release,
                surfaces.contract_version,
                &surfaces.status,
            ),
            (
                "mutation-stages.json",
                &mutation.release,
                mutation.contract_version,
                &mutation.status,
            ),
            (
                "retry-idempotency.json",
                &retry.release,
                retry.contract_version,
                &retry.status,
            ),
            (
                "failure-consistency-matrix.json",
                &failure.release,
                failure.contract_version,
                &failure.status,
            ),
            (
                "rpo-rto-contract.json",
                &rpo_rto.release,
                rpo_rto.contract_version,
                &rpo_rto.status,
            ),
            (
                "security-contract.json",
                &security.release,
                security.contract_version,
                &security.status,
            ),
            (
                "canonical-key-vectors.json",
                &canonical_key.release,
                canonical_key.contract_version,
                &canonical_key.status,
            ),
        ],
        &mut problems,
    );

    if status.w0_disposition != "open" {
        problems.push("status.json must keep w0_disposition=open until 0.74 is published".into());
    }
    if status.production_capability != "disabled_fail_closed" {
        problems.push("status.json must keep production_capability=disabled_fail_closed".into());
    }
    if status.safe_foundation.phase != "scaffolded_not_admitted" {
        problems
            .push("status.json safe_foundation.phase must remain scaffolded_not_admitted".into());
    }
    let implemented = status
        .safe_foundation
        .implemented
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for required in [
        "provisional_w0_contracts",
        "reference_mutation_model",
        "linearizability_oracle",
        "deterministic_fault_scenarios",
        "java_semantic_harness",
        "executable_authority_model_receipts",
        "evidence_schema_validation",
        "value_plane_threat_model",
        "fairness_namespace_reference_models",
        "extended_java_semantic_harness",
        "provisional_backend_interfaces",
        "hazelcast_source_provenance",
        "distributed_value_plane_simulator",
        "executable_security_guards",
        "complete_foundation_receipt_generation",
        "cross_language_reference_key_codec",
        "local_distributed_correctness_gate",
        "seeded_stateful_chaos_campaign",
    ] {
        if !implemented.contains(required) {
            problems.push(format!(
                "status.json safe foundation is missing implemented slice {required}"
            ));
        }
    }
    if status.safe_foundation.deferred.is_empty() {
        problems.push("status.json safe foundation must retain deferred work".into());
    }
    if status.blocked_by_074.is_empty() {
        problems.push("status.json must name blocked-by-0.74 dependencies".into());
    }
    for blocked in &status.blocked_by_074 {
        require_text("blocked dependency id", &blocked.id, &mut problems);
        require_text(
            "blocked dependency artifact",
            &blocked.required_artifact,
            &mut problems,
        );
        require_text("blocked dependency reason", &blocked.reason, &mut problems);
    }

    let bound_ids = validate_bounds(&bounds, &mut problems);
    let operation_ids = validate_operations(&operations, &bound_ids, &mut problems);
    validate_surfaces(&surfaces, &operation_ids, &mut problems);
    validate_mutation(&mutation, &mut problems);
    validate_retry(&retry, &bound_ids, &mut problems);
    validate_failure(&failure, &mut problems);
    validate_rpo_rto(&rpo_rto, &mut problems);
    validate_security(&security, &mut problems);
    validate_canonical_key(&canonical_key, &mut problems);
    Ok(problems)
}

fn load<T: DeserializeOwned>(dir: &Path, name: &str) -> Result<T, Box<dyn Error>> {
    let path = dir.join(name);
    let bytes =
        fs::read(&path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid {}: {error}", path.display()).into())
}

fn validate_headers<'a>(
    headers: impl IntoIterator<Item = (&'a str, &'a String, u32, &'a String)>,
    problems: &mut Vec<String>,
) {
    for (file, release, version, status) in headers {
        if release != RELEASE {
            problems.push(format!("{file} release must be {RELEASE}"));
        }
        if version != 1 {
            problems.push(format!("{file} contract_version must be 1"));
        }
        if status != "provisional" {
            problems.push(format!(
                "{file} status must be provisional; W0 cannot be finalized before 0.74"
            ));
        }
    }
}

fn validate_bounds(bounds: &BoundsContract, problems: &mut Vec<String>) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for bound in &bounds.bounds {
        if !ids.insert(bound.id.clone()) {
            problems.push(format!("duplicate bound id {}", bound.id));
        }
        if bound.maximum == 0 || bound.maximum < bound.minimum {
            problems.push(format!("bound {} has invalid finite range", bound.id));
        }
        require_text("bound unit", &bound.unit, problems);
        require_text("bound enforcement", &bound.enforcement, problems);
        require_text("bound error", &bound.error, problems);
        require_test("bound", &bound.id, &bound.test_id, problems);
    }
    require_exact_ids("bounds", &ids, REQUIRED_BOUNDS, problems);
    ids
}

fn validate_operations(
    contract: &OperationContract,
    bounds: &BTreeSet<String>,
    problems: &mut Vec<String>,
) -> BTreeSet<String> {
    let required_outcomes = [
        "inserted",
        "replaced",
        "removed",
        "present",
        "absent",
        "mismatch",
        "expired_before_operation",
    ];
    let outcomes: BTreeSet<_> = contract.result_outcomes.iter().cloned().collect();
    require_exact_ids("result outcomes", &outcomes, required_outcomes, problems);

    let mut error_ids = BTreeSet::new();
    for error in &contract.errors {
        if !error_ids.insert(error.id.clone()) {
            problems.push(format!("duplicate error id {}", error.id));
        }
        require_text("error retry_class", &error.retry_class, problems);
        require_text("error certainty", &error.certainty, problems);
        require_test("error", &error.id, &error.test_id, problems);
    }
    let mut ttl_ids = BTreeSet::new();
    for ttl in &contract.ttl_directives {
        if !ttl_ids.insert(ttl.id.clone()) {
            problems.push(format!("duplicate TTL directive {}", ttl.id));
        }
        require_text("TTL semantics", &ttl.semantics, problems);
        require_test("TTL directive", &ttl.id, &ttl.test_id, problems);
        if ttl.id == "expire_after" && ttl.zero_duration_allowed {
            problems.push("expire_after must reject zero duration".into());
        }
    }
    require_exact_ids(
        "TTL directives",
        &ttl_ids,
        ["preserve", "eternal", "expire_after"],
        problems,
    );

    let mut operation_ids = BTreeSet::new();
    for operation in &contract.operations {
        if !operation_ids.insert(operation.id.clone()) {
            problems.push(format!("duplicate operation id {}", operation.id));
        }
        if !matches!(
            operation.disposition.as_str(),
            "supported" | "supported_with_documented_divergence" | "unsupported"
        ) {
            problems.push(format!(
                "operation {} has invalid disposition",
                operation.id
            ));
        }
        if operation.outcomes.is_empty() {
            problems.push(format!("operation {} has no result outcomes", operation.id));
        }
        for outcome in &operation.outcomes {
            if !outcomes.contains(outcome) {
                problems.push(format!(
                    "operation {} uses unknown outcome {outcome}",
                    operation.id
                ));
            }
        }
        if operation.errors.is_empty() {
            problems.push(format!("operation {} has no explicit errors", operation.id));
        }
        for error in &operation.errors {
            if !error_ids.contains(error) {
                problems.push(format!(
                    "operation {} uses unknown error {error}",
                    operation.id
                ));
            }
        }
        for ttl in &operation.ttl_policy {
            if !ttl_ids.contains(ttl) {
                problems.push(format!(
                    "operation {} uses unknown TTL directive {ttl}",
                    operation.id
                ));
            }
        }
        require_text("operation retry_class", &operation.retry_class, problems);
        if operation.bound_ids.is_empty() {
            problems.push(format!("operation {} has no bounds", operation.id));
        }
        for bound in &operation.bound_ids {
            if !bounds.contains(bound) {
                problems.push(format!(
                    "operation {} uses unknown bound {bound}",
                    operation.id
                ));
            }
        }
        if operation.test_ids.is_empty() || operation.test_ids.iter().any(|id| id.trim().is_empty())
        {
            problems.push(format!("operation {} has a missing test id", operation.id));
        }
    }
    require_exact_ids("operations", &operation_ids, REQUIRED_OPERATIONS, problems);
    operation_ids
}

fn validate_surfaces(
    contract: &SurfaceContract,
    operations: &BTreeSet<String>,
    problems: &mut Vec<String>,
) {
    let mut surface_ids = BTreeSet::new();
    for surface in &contract.surfaces {
        if !surface_ids.insert(surface.id.clone()) {
            problems.push(format!("duplicate surface {}", surface.id));
        }
        for (label, value) in [
            ("backend", &surface.backend),
            ("key_codec", &surface.key_codec),
            ("value_codec", &surface.value_codec),
            ("ttl_mapping", &surface.ttl_mapping),
            ("response_projection", &surface.response_projection),
            ("event_projection", &surface.event_projection),
        ] {
            require_text(&format!("surface {} {label}", surface.id), value, problems);
        }
        let mut cells = BTreeSet::new();
        for cell in &surface.cells {
            if !cells.insert(cell.operation.clone()) {
                problems.push(format!(
                    "surface {} has duplicate operation cell {}",
                    surface.id, cell.operation
                ));
            }
            if !operations.contains(&cell.operation) {
                problems.push(format!(
                    "surface {} has unknown operation cell {}",
                    surface.id, cell.operation
                ));
            }
            if !matches!(cell.disposition.as_str(), "enabled" | "unsupported") {
                problems.push(format!(
                    "surface {} operation {} has invalid disposition",
                    surface.id, cell.operation
                ));
            }
            if cell.disposition == "unsupported"
                && cell
                    .unsupported_error
                    .as_deref()
                    .unwrap_or("")
                    .trim()
                    .is_empty()
            {
                problems.push(format!(
                    "surface {} operation {} must fail loudly with unsupported_error",
                    surface.id, cell.operation
                ));
            }
            require_test(
                "surface cell",
                &format!("{}:{}", surface.id, cell.operation),
                &cell.test_id,
                problems,
            );
        }
        for missing in operations.difference(&cells) {
            problems.push(format!(
                "surface {} is missing operation cell {missing}",
                surface.id
            ));
        }
    }
    require_exact_ids("surfaces", &surface_ids, SURFACES, problems);
}

fn validate_mutation(contract: &MutationContract, problems: &mut Vec<String>) {
    let mut stages = BTreeSet::new();
    for stage in &contract.stages {
        if !stages.insert(stage.id.clone()) {
            problems.push(format!("duplicate mutation stage {}", stage.id));
        }
        require_text("mutation client_retry", &stage.client_retry, problems);
        require_text("mutation failure_outcome", &stage.failure_outcome, problems);
        require_text(
            "mutation promoted_owner_proof_id",
            &stage.promoted_owner_proof_id,
            problems,
        );
        require_test("mutation stage", &stage.id, &stage.test_id, problems);
        if stage.id == "outcome_unknown" && !stage.reads_may_observe {
            problems.push("outcome_unknown must allow that reads may observe the mutation".into());
        }
    }
    require_exact_ids("mutation stages", &stages, MUTATION_STAGES, problems);
    let actual_order: Vec<_> = contract
        .stages
        .iter()
        .map(|stage| stage.id.as_str())
        .collect();
    if actual_order != MUTATION_STAGES {
        problems.push("mutation stages must retain the canonical received-to-outcome order".into());
    }
}

fn validate_retry(contract: &RetryContract, bounds: &BTreeSet<String>, problems: &mut Vec<String>) {
    if !bounds.contains(&contract.dedup_bound_id) {
        problems.push(format!(
            "retry dedup_bound_id {} is not declared",
            contract.dedup_bound_id
        ));
    }
    let mut ids = BTreeSet::new();
    for rule in &contract.rules {
        if !ids.insert(rule.id.clone()) {
            problems.push(format!("duplicate retry rule {}", rule.id));
        }
        require_text("retry ambiguous_outcome", &rule.ambiguous_outcome, problems);
        require_text("retry advice", &rule.retry, problems);
        require_text("retry terminal_error", &rule.terminal_error, problems);
        require_text("retry proof_id", &rule.proof_id, problems);
        require_test("retry rule", &rule.id, &rule.test_id, problems);
        if rule.id == "ambiguous_without_identity" && rule.idempotency_identity_required {
            problems.push("ambiguous_without_identity must describe the no-identity case".into());
        }
    }
    require_exact_ids(
        "retry rules",
        &ids,
        [
            "pre_admission_failure",
            "ambiguous_with_retained_identity",
            "ambiguous_without_identity",
            "dedup_evicted",
        ],
        problems,
    );
}

fn validate_failure(contract: &FailureContract, problems: &mut Vec<String>) {
    let mut scenarios = BTreeSet::new();
    for cell in &contract.cells {
        if !scenarios.insert(cell.scenario.clone()) {
            problems.push(format!("duplicate failure cell {}", cell.scenario));
        }
        for (label, value) in [
            ("read_availability", &cell.read_availability),
            ("write_availability", &cell.write_availability),
            ("certainty", &cell.certainty),
            ("consistency", &cell.consistency),
            ("retry", &cell.retry),
            ("readiness", &cell.readiness),
            ("proof_id", &cell.proof_id),
        ] {
            require_text(
                &format!("failure {} {label}", cell.scenario),
                value,
                problems,
            );
        }
        require_test("failure cell", &cell.scenario, &cell.test_id, problems);
    }
    require_exact_ids("failure scenarios", &scenarios, FAILURE_SCENARIOS, problems);
}

fn validate_rpo_rto(contract: &RpoRtoContract, problems: &mut Vec<String>) {
    let mut cells = BTreeSet::new();
    for cell in &contract.cells {
        let key = format!("{}:{}", cell.profile, cell.fault);
        if !cells.insert(key.clone()) {
            problems.push(format!("duplicate RPO/RTO cell {key}"));
        }
        if !RPO_PROFILES.contains(&cell.profile.as_str()) {
            problems.push(format!("unknown RPO/RTO profile {}", cell.profile));
        }
        if !RPO_FAULTS.contains(&cell.fault.as_str()) {
            problems.push(format!("unknown RPO/RTO fault {}", cell.fault));
        }
        for (label, value) in [
            ("support", &cell.support),
            ("rpo", &cell.rpo),
            ("rto_class", &cell.rto_class),
            ("proof_id", &cell.proof_id),
        ] {
            require_text(&format!("RPO/RTO {key} {label}"), value, problems);
        }
        require_test("RPO/RTO cell", &key, &cell.test_id, problems);
    }
    for profile in RPO_PROFILES {
        for fault in RPO_FAULTS {
            let key = format!("{profile}:{fault}");
            if !cells.contains(&key) {
                problems.push(format!("missing RPO/RTO cell {key}"));
            }
        }
    }
}

fn validate_security(contract: &SecurityContract, problems: &mut Vec<String>) {
    let mut ids = BTreeSet::new();
    for threat in &contract.threats {
        if !ids.insert(threat.id.clone()) {
            problems.push(format!("duplicate security threat {}", threat.id));
        }
        for (label, value) in [
            ("asset", &threat.asset),
            ("boundary", &threat.boundary),
            ("mitigation", &threat.mitigation),
            ("failure", &threat.failure),
            ("bound_or_proof", &threat.bound_or_proof),
        ] {
            require_text(
                &format!("security threat {} {label}", threat.id),
                value,
                problems,
            );
        }
        require_test("security threat", &threat.id, &threat.test_id, problems);
    }
    require_exact_ids("security threats", &ids, REQUIRED_THREATS, problems);
}

fn validate_canonical_key(contract: &CanonicalKeyContract, problems: &mut Vec<String>) {
    if contract.codec_id != "hydracache.imap.reference-canonical-key.075.v1" {
        problems.push("canonical key contract has an unexpected reference codec id".into());
    }
    if contract.production_wire_identity != "unassigned"
        || contract.production_partition_hash != "unassigned"
    {
        problems.push("canonical key reference must not allocate production identities".into());
    }
    let mut ids = BTreeSet::new();
    for vector in &contract.vectors {
        if !ids.insert(vector.id.clone()) {
            problems.push(format!("duplicate canonical key vector {}", vector.id));
        }
        if vector.tenant.is_empty() || vector.namespace.is_empty() {
            problems.push(format!(
                "canonical key vector {} has empty identity",
                vector.id
            ));
        }
        if vector.namespace_generation == 0 {
            problems.push(format!(
                "canonical key vector {} has generation zero",
                vector.id
            ));
        }
        if vector.key_hex.len() % 2 != 0
            || vector.encoded_hex.is_empty()
            || vector.encoded_hex.len() % 2 != 0
            || !vector
                .key_hex
                .bytes()
                .chain(vector.encoded_hex.bytes())
                .all(|byte| byte.is_ascii_hexdigit())
        {
            problems.push(format!(
                "canonical key vector {} has invalid hex",
                vector.id
            ));
        }
        if vector.test_ids.is_empty() || vector.test_ids.iter().any(|id| id.trim().is_empty()) {
            problems.push(format!(
                "canonical key vector {} has no test ids",
                vector.id
            ));
        }
    }
    require_exact_ids(
        "canonical key vectors",
        &ids,
        ["ascii-empty-key", "unicode-composed", "unicode-decomposed"],
        problems,
    );
}

fn require_exact_ids<const N: usize>(
    label: &str,
    actual: &BTreeSet<String>,
    required: [&str; N],
    problems: &mut Vec<String>,
) {
    for id in required {
        if !actual.contains(id) {
            problems.push(format!("{label} missing required id {id}"));
        }
    }
}

fn require_text(label: &str, value: &str, problems: &mut Vec<String>) {
    if value.trim().is_empty() {
        problems.push(format!("{label} must not be empty"));
    }
}

fn require_test(kind: &str, id: &str, test_id: &str, problems: &mut Vec<String>) {
    if test_id.trim().is_empty() {
        problems.push(format!("{kind} {id} has a missing test id"));
    }
}

pub fn contract_dir(root: &Path) -> PathBuf {
    root.join("docs/testing/imap/0.75")
}

pub fn problem_counts_by_prefix(problems: &[String]) -> BTreeMap<&str, usize> {
    let mut counts = BTreeMap::new();
    for problem in problems {
        let prefix = problem.split_whitespace().next().unwrap_or("unknown");
        *counts.entry(prefix).or_insert(0) += 1;
    }
    counts
}
