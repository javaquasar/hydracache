use serde_json::Value as JsonValue;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use toml::Value as TomlValue;

const CONTRACT: &str = "docs/testing/performance/0.73/local-screening.toml";
const RECEIPT_SCHEMA: &str = "docs/testing/performance/0.73/local-screening-receipt-v1.schema.json";
const CONTEXT_SCHEMA: &str = "docs/testing/performance/0.73/local-screening-context-v1.schema.json";
const EXAMPLE_RECEIPT: &str = "docs/testing/performance/0.73/local-screening-receipt.example.json";
const BASELINE_IDENTITIES: &str = "docs/testing/performance/0.73/baseline-identities.toml";
const POST_TAG_DELTA: &str = "docs/testing/performance/0.73/post-tag-delta.toml";
const SCENARIO_MATRIX: &str = "docs/testing/performance/0.73/scenario-matrix.toml";
const INSTRUMENTATION_OVERHEAD: &str =
    "docs/testing/performance/0.73/instrumentation-overhead.toml";
const LOCAL_OVERHEAD_SCREENING: &str =
    "docs/testing/performance/0.73/local-overhead-screening-307b3500.toml";
const LOCAL_OVERHEAD_ISOLATION: &str =
    "docs/testing/performance/0.73/local-overhead-isolation-5264d96c.toml";
const LOCAL_OVERHEAD_LISTENER_NOOP: &str =
    "docs/testing/performance/0.73/local-overhead-listener-noop-5e101e69.toml";
const NOTIFICATION_FEASIBILITY: &str =
    "docs/testing/performance/0.73/notification-feasibility-bf9f1382.toml";
const NOTIFICATION_OBSERVER_REQUIREMENTS: &str =
    "docs/testing/performance/0.73/notification-observer-requirements.toml";
const NOTIFICATION_OBSERVER_PROTOTYPE: &str =
    "docs/testing/performance/0.73/notification-observer-prototype-9a2ca114.toml";
const MOKA_OBSERVER_SPIKE: &str = "docs/testing/performance/0.73/moka-observer-spike-5d560170.toml";
const MOKA_OBSERVER_DIRECT: &str =
    "docs/testing/performance/0.73/moka-observer-direct-779849b6.toml";
const NOTIFICATION_OBSERVER_D2_REVIEW: &str =
    "docs/testing/performance/0.73/notification-observer-d2-review.toml";
const MOKA_OBSERVER_UPSTREAM_DRAFT: &str =
    "docs/testing/performance/0.73/moka-post-removal-observer-upstream-draft.md";
const SINGLE_MAINTAINER_REVIEW_POLICY: &str =
    "docs/testing/performance/0.73/single-maintainer-review-policy.toml";
const MOKA_FORK_DECISION: &str = "docs/testing/performance/0.73/moka-fork-decision-352e53fa.toml";
const NOTIFICATION_OBSERVER_PRODUCT: &str =
    "docs/testing/performance/0.73/notification-observer-product-73fc38a1.toml";
const LOCAL_OBSERVER_PRODUCT_SCREENING: &str =
    "docs/testing/performance/0.73/local-observer-product-screening-06a8bd95.toml";
const PROPOSAL_REGISTRY: &str = "docs/testing/performance/0.73/proposal-registry.toml";
const STATISTICS: &str = "docs/testing/performance/0.73/statistics.toml";
const HOST_PROFILE: &str = "docs/testing/performance/0.73/host-profile.toml";
const HOST_ADMISSION: &str = "docs/testing/performance/0.73/host-admission-3ba09fcc.toml";
const BASELINE_PILOT_CONTRACT: &str = "docs/testing/performance/0.73/baseline-pilot-contract.toml";
const BASELINE_PILOT_EVIDENCE: &str =
    "docs/testing/performance/0.73/baseline-pilot-insufficient-4ba93a1a.toml";
const CPU_ATTRIBUTION_CONTRACT: &str =
    "docs/testing/performance/0.73/cpu-attribution-contract.toml";
const CPU_ATTRIBUTION_EVIDENCE: &str =
    "docs/testing/performance/0.73/cpu-attribution-ed339846.toml";
const REMOVAL_DRAIN_FAST_PATH: &str =
    "docs/testing/performance/0.73/removal-drain-fast-path-affe4390.toml";
const BASELINE_PILOT_FAST_PATH_EVIDENCE: &str =
    "docs/testing/performance/0.73/baseline-pilot-insufficient-9bba762c.toml";
const REMOVAL_QUEUE_CONTRACT: &str = "docs/testing/performance/0.73/removal-queue-contract.toml";
const REMOVAL_QUEUE_PRODUCT: &str =
    "docs/testing/performance/0.73/removal-queue-product-daffd71b.toml";
const BASELINE_PILOT_ARRAY_QUEUE_EVIDENCE: &str =
    "docs/testing/performance/0.73/baseline-pilot-insufficient-90a40510.toml";
const REMOVAL_SEQUENCE_CONTRACT: &str =
    "docs/testing/performance/0.73/removal-sequence-contract.toml";
const REMOVAL_SEQUENCE_PRODUCT: &str =
    "docs/testing/performance/0.73/removal-sequence-product-549fbaeb.toml";
const BASELINE_PILOT_SEQUENCE_EVIDENCE: &str =
    "docs/testing/performance/0.73/baseline-pilot-insufficient-862c9015.toml";
const BASELINE_PILOT_V2_CONTRACT: &str =
    "docs/testing/performance/0.73/baseline-pilot-v2-contract.toml";
const BASELINE_PILOT_V2_PRODUCT: &str =
    "docs/testing/performance/0.73/baseline-pilot-v2-product-4979e2e1.toml";
const BASELINE_PILOT_V2_EVIDENCE: &str =
    "docs/testing/performance/0.73/baseline-pilot-v2-insufficient-72d491ac.toml";
const MEMORY_COUNTER_ATOMIC_CONTRACT: &str =
    "docs/testing/performance/0.73/memory-counter-atomic-contract.toml";
const MEMORY_COUNTER_ATOMIC_PRODUCT: &str =
    "docs/testing/performance/0.73/memory-counter-atomic-product-a205ce0a.toml";
const BASELINE_PILOT_V2_COUNTER_EVIDENCE: &str =
    "docs/testing/performance/0.73/baseline-pilot-v2-insufficient-2daccb47.toml";
const OBSERVER_ALLOCATION_ATTRIBUTION_CONTRACT: &str =
    "docs/testing/performance/0.73/observer-allocation-attribution-contract.toml";
const OBSERVER_ALLOCATION_ATTRIBUTION_EVIDENCE: &str =
    "docs/testing/performance/0.73/observer-allocation-attribution-2c37d2f1.toml";
const SHARED_ENTRY_TAGS_CONTRACT: &str =
    "docs/testing/performance/0.73/shared-entry-tags-contract.toml";
const SHARED_ENTRY_TAGS_PRODUCT: &str =
    "docs/testing/performance/0.73/shared-entry-tags-product-947e624d.toml";
const BASELINE_PILOT_V2_FREEZE_EVIDENCE: &str =
    "docs/testing/performance/0.73/baseline-pilot-v2-passed-e757556d.toml";
const W1_OWNER_CLASSIFICATION_CONTRACT: &str =
    "docs/testing/performance/0.73/w1-owner-classification-contract.toml";
const W2_EXPIRY_SWEEP_PROFILE_CONTRACT: &str =
    "docs/testing/performance/0.73/w2-expiry-sweep-profile-contract.toml";
const W2_EXPIRY_SWEEP_PROFILE_EVIDENCE: &str =
    "docs/testing/performance/0.73/w2-expiry-sweep-profile-e4a61d9f.toml";
const W2_BORROWED_EXPIRY_SCAN_CONTRACT: &str =
    "docs/testing/performance/0.73/w2-borrowed-expiry-scan-contract.toml";
const W2_BORROWED_EXPIRY_SCAN_EVIDENCE: &str =
    "docs/testing/performance/0.73/w2-borrowed-expiry-scan-a80839fd.toml";
const W5_HC2_EVENT_COPY_PROFILE_CONTRACT: &str =
    "docs/testing/performance/0.73/w5-hc2-event-copy-profile-contract.toml";
const W5_HC2_EVENT_COPY_PROFILE_EVIDENCE: &str =
    "docs/testing/performance/0.73/w5-hc2-event-copy-profile-02777da6.toml";
const W5_SHARED_EVENT_BYTES_CONTRACT: &str =
    "docs/testing/performance/0.73/w5-shared-event-bytes-contract.toml";
const W5_HC2_CONNECTION_CENSUS_CONTRACT: &str =
    "docs/testing/performance/0.73/w5-hc2-connection-census-contract.toml";
const W5_HC2_CONNECTION_PROFILE_CONTRACT: &str =
    "docs/testing/performance/0.73/w5-hc2-connection-profile-contract.toml";
const W5_HC2_CONNECTION_PROFILE_EVIDENCE: &str =
    "docs/testing/performance/0.73/w5-hc2-connection-profile-ff657fc5.toml";
const W5_HC2_SPLIT_CONNECTION_PROFILE_CONTRACT: &str =
    "docs/testing/performance/0.73/w5-hc2-split-connection-profile-contract.toml";
const W5_HC2_SPLIT_CONNECTION_PROFILE_EVIDENCE: &str =
    "docs/testing/performance/0.73/w5-hc2-split-connection-profile-19a440d4.toml";
const W5_HC2_SLOW_CONSUMER_PROFILE_CONTRACT: &str =
    "docs/testing/performance/0.73/w5-hc2-slow-consumer-profile-contract.toml";
const W5_HC2_SLOW_CONSUMER_PROFILE_EVIDENCE: &str =
    "docs/testing/performance/0.73/w5-hc2-slow-consumer-profile-ff432801.toml";
const W5_HC2_RAW_TRANSPORT_STALL_CONTRACT: &str =
    "docs/testing/performance/0.73/w5-hc2-raw-transport-stall-contract.toml";
const W5_HC2_RAW_TRANSPORT_STALL_EVIDENCE: &str =
    "docs/testing/performance/0.73/w5-hc2-raw-transport-stall-25cdfb61.toml";
const W5_HC2_OUTBOUND_BYTE_ADMISSION_CONTRACT: &str =
    "docs/testing/performance/0.73/w5-hc2-outbound-byte-admission-contract.toml";
const W5_HC2_OUTBOUND_BYTE_ADMISSION_EVIDENCE: &str =
    "docs/testing/performance/0.73/w5-hc2-outbound-byte-admission-2846e936.toml";
const W6_MANAGEMENT_OVERHEAD_PROFILE_CONTRACT: &str =
    "docs/testing/performance/0.73/w6-management-overhead-profile-contract.toml";
const W6_MANAGEMENT_OVERHEAD_PROFILE_EVIDENCE: &str =
    "docs/testing/performance/0.73/w6-management-overhead-316961e0.toml";
const W3_TAG_INDEX_PROFILE_CONTRACT: &str =
    "docs/testing/performance/0.73/w3-tag-index-profile-contract.toml";
const W3_TAG_INDEX_PROFILE_EVIDENCE: &str =
    "docs/testing/performance/0.73/w3-tag-index-profile-f8b90ef0.toml";
const W3_SHARED_EVENT_TAGS_CONTRACT: &str =
    "docs/testing/performance/0.73/w3-shared-event-tags-contract.toml";
const W3_SHARED_EVENT_TAGS_EVIDENCE: &str =
    "docs/testing/performance/0.73/w3-shared-event-tags-accepted-82f46245.toml";
const RELEASE: &str = "0.73";
const PROFILE: &str = "local-screening-073-v1";
const ENVIRONMENT_CLASS: &str = "local_screening";

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(args)?;
    let mut problems = check_at_root(&options.root, &options.release, options.receipt.as_deref())?;
    if options.require_ship {
        problems.push(
            "local screening is intentionally non-promotable and cannot satisfy --require-ship"
                .to_owned(),
        );
    }
    finish("performance-contract-check", &options.release, problems)
}

pub fn check_at_root(
    root: &Path,
    release: &str,
    receipt: Option<&Path>,
) -> Result<Vec<String>, Box<dyn Error>> {
    let contract: TomlValue = toml::from_str(&fs::read_to_string(root.join(CONTRACT))?)?;
    let schema: JsonValue = serde_json::from_slice(&fs::read(root.join(RECEIPT_SCHEMA))?)?;
    let example: JsonValue = serde_json::from_slice(&fs::read(root.join(EXAMPLE_RECEIPT))?)?;
    let identities: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(BASELINE_IDENTITIES))?)?;
    let delta: TomlValue = toml::from_str(&fs::read_to_string(root.join(POST_TAG_DELTA))?)?;
    let matrix: TomlValue = toml::from_str(&fs::read_to_string(root.join(SCENARIO_MATRIX))?)?;
    let overhead: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(INSTRUMENTATION_OVERHEAD))?)?;
    let local_overhead: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(LOCAL_OVERHEAD_SCREENING))?)?;
    let local_isolation: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(LOCAL_OVERHEAD_ISOLATION))?)?;
    let local_listener_noop: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(LOCAL_OVERHEAD_LISTENER_NOOP),
    )?)?;
    let notification_feasibility: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(NOTIFICATION_FEASIBILITY))?)?;
    let notification_observer_requirements: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(NOTIFICATION_OBSERVER_REQUIREMENTS),
    )?)?;
    let notification_observer_prototype: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(NOTIFICATION_OBSERVER_PROTOTYPE),
    )?)?;
    let moka_observer_spike: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(MOKA_OBSERVER_SPIKE))?)?;
    let moka_observer_direct: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(MOKA_OBSERVER_DIRECT))?)?;
    let notification_observer_d2_review: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(NOTIFICATION_OBSERVER_D2_REVIEW),
    )?)?;
    let single_maintainer_review_policy: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(SINGLE_MAINTAINER_REVIEW_POLICY),
    )?)?;
    let moka_fork_decision: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(MOKA_FORK_DECISION))?)?;
    let notification_observer_product: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(NOTIFICATION_OBSERVER_PRODUCT),
    )?)?;
    let local_observer_product_screening: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(LOCAL_OBSERVER_PRODUCT_SCREENING),
    )?)?;
    let proposal_registry: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(PROPOSAL_REGISTRY))?)?;
    let statistics: TomlValue = toml::from_str(&fs::read_to_string(root.join(STATISTICS))?)?;
    let host_profile: TomlValue = toml::from_str(&fs::read_to_string(root.join(HOST_PROFILE))?)?;
    let host_admission: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(HOST_ADMISSION))?)?;
    let baseline_pilot_contract: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(BASELINE_PILOT_CONTRACT))?)?;
    let baseline_pilot_evidence: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(BASELINE_PILOT_EVIDENCE))?)?;
    let cpu_attribution_contract: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(CPU_ATTRIBUTION_CONTRACT))?)?;
    let cpu_attribution_evidence: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(CPU_ATTRIBUTION_EVIDENCE))?)?;
    let removal_drain_fast_path: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(REMOVAL_DRAIN_FAST_PATH))?)?;
    let baseline_pilot_fast_path_evidence: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(BASELINE_PILOT_FAST_PATH_EVIDENCE),
    )?)?;
    let removal_queue_contract: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(REMOVAL_QUEUE_CONTRACT))?)?;
    let removal_queue_product: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(REMOVAL_QUEUE_PRODUCT))?)?;
    let baseline_pilot_array_queue_evidence: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(BASELINE_PILOT_ARRAY_QUEUE_EVIDENCE),
    )?)?;
    let removal_sequence_contract: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(REMOVAL_SEQUENCE_CONTRACT))?)?;
    let removal_sequence_product: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(REMOVAL_SEQUENCE_PRODUCT))?)?;
    let baseline_pilot_sequence_evidence: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(BASELINE_PILOT_SEQUENCE_EVIDENCE),
    )?)?;
    let baseline_pilot_v2_contract: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(BASELINE_PILOT_V2_CONTRACT))?)?;
    let baseline_pilot_v2_product: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(BASELINE_PILOT_V2_PRODUCT))?)?;
    let baseline_pilot_v2_evidence: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(BASELINE_PILOT_V2_EVIDENCE))?)?;
    let memory_counter_atomic_contract: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(MEMORY_COUNTER_ATOMIC_CONTRACT),
    )?)?;
    let memory_counter_atomic_product: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(MEMORY_COUNTER_ATOMIC_PRODUCT),
    )?)?;
    let baseline_pilot_v2_counter_evidence: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(BASELINE_PILOT_V2_COUNTER_EVIDENCE),
    )?)?;
    let observer_allocation_attribution_contract: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(OBSERVER_ALLOCATION_ATTRIBUTION_CONTRACT),
    )?)?;
    let observer_allocation_attribution_evidence: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(OBSERVER_ALLOCATION_ATTRIBUTION_EVIDENCE),
    )?)?;
    let shared_entry_tags_contract: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(SHARED_ENTRY_TAGS_CONTRACT))?)?;
    let shared_entry_tags_product: TomlValue =
        toml::from_str(&fs::read_to_string(root.join(SHARED_ENTRY_TAGS_PRODUCT))?)?;
    let baseline_pilot_v2_freeze_evidence: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(BASELINE_PILOT_V2_FREEZE_EVIDENCE),
    )?)?;
    let w1_owner_classification_contract: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(W1_OWNER_CLASSIFICATION_CONTRACT),
    )?)?;
    let w2_expiry_sweep_profile_contract: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(W2_EXPIRY_SWEEP_PROFILE_CONTRACT),
    )?)?;
    let w2_expiry_sweep_profile_evidence: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(W2_EXPIRY_SWEEP_PROFILE_EVIDENCE),
    )?)?;
    let w2_borrowed_expiry_scan_contract: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(W2_BORROWED_EXPIRY_SCAN_CONTRACT),
    )?)?;
    let w2_borrowed_expiry_scan_evidence: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(W2_BORROWED_EXPIRY_SCAN_EVIDENCE),
    )?)?;
    let w5_hc2_event_copy_profile_contract: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(W5_HC2_EVENT_COPY_PROFILE_CONTRACT),
    )?)?;
    let w5_hc2_event_copy_profile_evidence: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(W5_HC2_EVENT_COPY_PROFILE_EVIDENCE),
    )?)?;
    let w5_shared_event_bytes_contract: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(W5_SHARED_EVENT_BYTES_CONTRACT),
    )?)?;
    let w5_hc2_connection_census_contract: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(W5_HC2_CONNECTION_CENSUS_CONTRACT),
    )?)?;
    let w5_hc2_connection_profile_contract: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(W5_HC2_CONNECTION_PROFILE_CONTRACT),
    )?)?;
    let w5_hc2_connection_profile_evidence: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(W5_HC2_CONNECTION_PROFILE_EVIDENCE),
    )?)?;
    let w5_hc2_split_connection_profile_contract: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(W5_HC2_SPLIT_CONNECTION_PROFILE_CONTRACT),
    )?)?;
    let w5_hc2_split_connection_profile_evidence: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(W5_HC2_SPLIT_CONNECTION_PROFILE_EVIDENCE),
    )?)?;
    let w5_hc2_slow_consumer_profile_contract: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(W5_HC2_SLOW_CONSUMER_PROFILE_CONTRACT),
    )?)?;
    let w5_hc2_slow_consumer_profile_evidence: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(W5_HC2_SLOW_CONSUMER_PROFILE_EVIDENCE),
    )?)?;
    let w5_hc2_raw_transport_stall_contract: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(W5_HC2_RAW_TRANSPORT_STALL_CONTRACT),
    )?)?;
    let w5_hc2_raw_transport_stall_evidence: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(W5_HC2_RAW_TRANSPORT_STALL_EVIDENCE),
    )?)?;
    let w5_hc2_outbound_byte_admission_contract: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(W5_HC2_OUTBOUND_BYTE_ADMISSION_CONTRACT),
    )?)?;
    let w5_hc2_outbound_byte_admission_evidence: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(W5_HC2_OUTBOUND_BYTE_ADMISSION_EVIDENCE),
    )?)?;
    let w6_management_overhead_profile_contract: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(W6_MANAGEMENT_OVERHEAD_PROFILE_CONTRACT),
    )?)?;
    let w6_management_overhead_profile_evidence: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(W6_MANAGEMENT_OVERHEAD_PROFILE_EVIDENCE),
    )?)?;
    let w3_tag_index_profile_contract: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(W3_TAG_INDEX_PROFILE_CONTRACT),
    )?)?;
    let w3_tag_index_profile_evidence: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(W3_TAG_INDEX_PROFILE_EVIDENCE),
    )?)?;
    let w3_shared_event_tags_contract: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(W3_SHARED_EVENT_TAGS_CONTRACT),
    )?)?;
    let w3_shared_event_tags_evidence: TomlValue = toml::from_str(&fs::read_to_string(
        root.join(W3_SHARED_EVENT_TAGS_EVIDENCE),
    )?)?;
    let mut problems = check_contract(&contract, release);
    if !root.join(MOKA_OBSERVER_UPSTREAM_DRAFT).is_file() {
        problems.push("notification observer dependency review draft is missing".to_owned());
    }
    problems.extend(check_baseline_identities(root, &identities, release)?);
    problems.extend(check_post_tag_delta(root, &delta, release)?);
    problems.extend(check_scenario_matrix(&matrix, release));
    problems.extend(check_instrumentation_overhead(&overhead, release));
    problems.extend(check_local_overhead_screening(&local_overhead, release));
    problems.extend(check_local_overhead_isolation(&local_isolation, release));
    problems.extend(check_local_overhead_listener_noop(
        &local_listener_noop,
        release,
    ));
    problems.extend(check_notification_feasibility(
        &notification_feasibility,
        release,
    ));
    problems.extend(check_notification_observer_requirements(
        &notification_observer_requirements,
        release,
    ));
    problems.extend(check_notification_observer_prototype(
        &notification_observer_prototype,
        release,
    ));
    problems.extend(check_moka_observer_spike(&moka_observer_spike, release));
    problems.extend(check_moka_observer_direct(&moka_observer_direct, release));
    problems.extend(check_notification_observer_d2_review(
        &notification_observer_d2_review,
        release,
    ));
    problems.extend(check_single_maintainer_review_policy(
        &single_maintainer_review_policy,
        release,
    ));
    problems.extend(check_moka_fork_decision(&moka_fork_decision, release));
    problems.extend(check_notification_observer_product(
        &notification_observer_product,
        release,
    ));
    problems.extend(check_local_observer_product_screening(
        &local_observer_product_screening,
        release,
    ));
    problems.extend(check_proposal_registry(&proposal_registry, release));
    problems.extend(check_statistics(&statistics, release));
    problems.extend(check_host_profile(&host_profile, release));
    problems.extend(check_host_admission(&host_admission, release));
    problems.extend(check_baseline_pilot_contract(
        &baseline_pilot_contract,
        release,
    ));
    problems.extend(check_baseline_pilot_evidence(
        &baseline_pilot_evidence,
        release,
    ));
    problems.extend(check_cpu_attribution_contract(
        &cpu_attribution_contract,
        release,
    ));
    problems.extend(check_cpu_attribution_evidence(
        &cpu_attribution_evidence,
        release,
    ));
    problems.extend(check_removal_drain_fast_path(
        &removal_drain_fast_path,
        release,
    ));
    problems.extend(check_baseline_pilot_fast_path_evidence(
        &baseline_pilot_fast_path_evidence,
        release,
    ));
    problems.extend(check_removal_queue_contract(
        &removal_queue_contract,
        release,
    ));
    problems.extend(check_removal_queue_product(&removal_queue_product, release));
    problems.extend(check_baseline_pilot_array_queue_evidence(
        &baseline_pilot_array_queue_evidence,
        release,
    ));
    problems.extend(check_removal_sequence_contract(
        &removal_sequence_contract,
        release,
    ));
    problems.extend(check_removal_sequence_product(
        &removal_sequence_product,
        release,
    ));
    problems.extend(check_baseline_pilot_sequence_evidence(
        &baseline_pilot_sequence_evidence,
        release,
    ));
    problems.extend(check_baseline_pilot_v2_contract(
        &baseline_pilot_v2_contract,
        release,
    ));
    problems.extend(check_baseline_pilot_v2_product(
        &baseline_pilot_v2_product,
        release,
    ));
    problems.extend(check_baseline_pilot_v2_evidence(
        &baseline_pilot_v2_evidence,
        release,
    ));
    problems.extend(check_memory_counter_atomic_contract(
        &memory_counter_atomic_contract,
        release,
    ));
    problems.extend(check_memory_counter_atomic_product(
        &memory_counter_atomic_product,
        release,
    ));
    problems.extend(check_baseline_pilot_v2_counter_evidence(
        &baseline_pilot_v2_counter_evidence,
        release,
    ));
    problems.extend(check_observer_allocation_attribution_contract(
        &observer_allocation_attribution_contract,
        release,
    ));
    problems.extend(check_observer_allocation_attribution_evidence(
        &observer_allocation_attribution_evidence,
        release,
    ));
    problems.extend(check_shared_entry_tags_contract(
        &shared_entry_tags_contract,
        release,
    ));
    problems.extend(check_shared_entry_tags_product(
        &shared_entry_tags_product,
        release,
    ));
    problems.extend(check_baseline_pilot_v2_freeze_evidence(
        &baseline_pilot_v2_freeze_evidence,
        release,
    ));
    problems.extend(check_w1_owner_classification_contract(
        &w1_owner_classification_contract,
        release,
    ));
    problems.extend(check_w2_expiry_sweep_profile_contract(
        &w2_expiry_sweep_profile_contract,
        release,
    ));
    problems.extend(check_w2_expiry_sweep_profile_evidence(
        &w2_expiry_sweep_profile_evidence,
        release,
    ));
    problems.extend(check_w2_borrowed_expiry_scan_contract(
        &w2_borrowed_expiry_scan_contract,
        release,
    ));
    problems.extend(check_w2_borrowed_expiry_scan_evidence(
        &w2_borrowed_expiry_scan_evidence,
        release,
    ));
    problems.extend(check_w5_hc2_event_copy_profile_contract(
        &w5_hc2_event_copy_profile_contract,
        release,
    ));
    problems.extend(check_w5_hc2_event_copy_profile_evidence(
        &w5_hc2_event_copy_profile_evidence,
        release,
    ));
    problems.extend(check_w5_shared_event_bytes_contract(
        &w5_shared_event_bytes_contract,
        release,
    ));
    problems.extend(check_w5_hc2_connection_census_contract(
        &w5_hc2_connection_census_contract,
        release,
    ));
    problems.extend(check_w5_hc2_connection_profile_contract(
        &w5_hc2_connection_profile_contract,
        release,
    ));
    problems.extend(check_w5_hc2_connection_profile_evidence(
        &w5_hc2_connection_profile_evidence,
        release,
    ));
    problems.extend(check_w5_hc2_split_connection_profile_contract(
        &w5_hc2_split_connection_profile_contract,
        release,
    ));
    problems.extend(check_w5_hc2_split_connection_profile_evidence(
        &w5_hc2_split_connection_profile_evidence,
        release,
    ));
    problems.extend(check_w5_hc2_slow_consumer_profile_contract(
        &w5_hc2_slow_consumer_profile_contract,
        release,
    ));
    problems.extend(check_w5_hc2_slow_consumer_profile_evidence(
        &w5_hc2_slow_consumer_profile_evidence,
        release,
    ));
    problems.extend(check_w5_hc2_raw_transport_stall_contract(
        &w5_hc2_raw_transport_stall_contract,
        release,
    ));
    problems.extend(check_w5_hc2_raw_transport_stall_evidence(
        &w5_hc2_raw_transport_stall_evidence,
        release,
    ));
    problems.extend(check_w5_hc2_outbound_byte_admission_contract(
        &w5_hc2_outbound_byte_admission_contract,
        release,
    ));
    problems.extend(check_w5_hc2_outbound_byte_admission_evidence(
        &w5_hc2_outbound_byte_admission_evidence,
        release,
    ));
    problems.extend(check_w6_management_overhead_profile_contract(
        &w6_management_overhead_profile_contract,
        release,
    ));
    problems.extend(check_w6_management_overhead_profile_evidence(
        &w6_management_overhead_profile_evidence,
        release,
    ));
    problems.extend(check_w3_tag_index_profile_contract(
        &w3_tag_index_profile_contract,
        release,
    ));
    problems.extend(check_w3_tag_index_profile_evidence(
        &w3_tag_index_profile_evidence,
        release,
    ));
    problems.extend(check_w3_shared_event_tags_contract(
        &w3_shared_event_tags_contract,
        release,
    ));
    problems.extend(check_w3_shared_event_tags_evidence(
        &w3_shared_event_tags_evidence,
        release,
    ));
    problems.extend(check_schema(
        &schema,
        &example,
        "checked-in local screening example",
    ));
    problems.extend(check_receipt(&example, &contract));
    if let Some(path) = receipt {
        let path = if path.is_absolute() {
            path.to_owned()
        } else {
            root.join(path)
        };
        let value: JsonValue = serde_json::from_slice(&fs::read(&path)?)?;
        problems.extend(check_schema(
            &schema,
            &value,
            &format!("local screening receipt {}", path.display()),
        ));
        problems.extend(check_receipt(&value, &contract));
    }
    Ok(problems)
}

pub fn check_receipt_at_root(
    root: &Path,
    value: &JsonValue,
) -> Result<Vec<String>, Box<dyn Error>> {
    let contract: TomlValue = toml::from_str(&fs::read_to_string(root.join(CONTRACT))?)?;
    let schema: JsonValue = serde_json::from_slice(&fs::read(root.join(RECEIPT_SCHEMA))?)?;
    let mut problems = check_schema(&schema, value, "generated local screening receipt");
    problems.extend(check_receipt(value, &contract));
    Ok(problems)
}

pub fn check_contract(root: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if release != RELEASE {
        problems.push(format!(
            "unsupported performance contract release {release}"
        ));
        return problems;
    }
    if integer(root, "schema_version") != Some(1) {
        problems.push("local screening schema_version must be 1".to_owned());
    }
    for (field, expected) in [
        ("release", RELEASE),
        ("profile_id", PROFILE),
        ("environment_class", ENVIRONMENT_CLASS),
        ("receipt_schema", RECEIPT_SCHEMA),
        ("context_schema", CONTEXT_SCHEMA),
        ("baseline_identities", BASELINE_IDENTITIES),
        ("post_tag_delta", POST_TAG_DELTA),
        ("scenario_matrix", SCENARIO_MATRIX),
        ("instrumentation_overhead", INSTRUMENTATION_OVERHEAD),
        ("local_overhead_screening", LOCAL_OVERHEAD_SCREENING),
        ("local_overhead_isolation", LOCAL_OVERHEAD_ISOLATION),
        ("local_overhead_listener_noop", LOCAL_OVERHEAD_LISTENER_NOOP),
        ("notification_feasibility", NOTIFICATION_FEASIBILITY),
        (
            "notification_observer_requirements",
            NOTIFICATION_OBSERVER_REQUIREMENTS,
        ),
        (
            "notification_observer_prototype",
            NOTIFICATION_OBSERVER_PROTOTYPE,
        ),
        ("moka_observer_spike", MOKA_OBSERVER_SPIKE),
        ("moka_observer_direct", MOKA_OBSERVER_DIRECT),
        (
            "notification_observer_d2_review",
            NOTIFICATION_OBSERVER_D2_REVIEW,
        ),
        (
            "single_maintainer_review_policy",
            SINGLE_MAINTAINER_REVIEW_POLICY,
        ),
        ("moka_fork_decision", MOKA_FORK_DECISION),
        (
            "notification_observer_product",
            NOTIFICATION_OBSERVER_PRODUCT,
        ),
        (
            "local_observer_product_screening",
            LOCAL_OBSERVER_PRODUCT_SCREENING,
        ),
        ("proposal_registry", PROPOSAL_REGISTRY),
        ("statistics_contract", STATISTICS),
        ("host_profile", HOST_PROFILE),
        ("host_admission", HOST_ADMISSION),
        ("baseline_pilot_contract", BASELINE_PILOT_CONTRACT),
        ("baseline_pilot_evidence", BASELINE_PILOT_EVIDENCE),
        ("cpu_attribution_contract", CPU_ATTRIBUTION_CONTRACT),
        ("cpu_attribution_evidence", CPU_ATTRIBUTION_EVIDENCE),
        ("removal_drain_fast_path", REMOVAL_DRAIN_FAST_PATH),
        (
            "baseline_pilot_fast_path_evidence",
            BASELINE_PILOT_FAST_PATH_EVIDENCE,
        ),
        ("removal_queue_contract", REMOVAL_QUEUE_CONTRACT),
        ("removal_queue_product", REMOVAL_QUEUE_PRODUCT),
        (
            "baseline_pilot_array_queue_evidence",
            BASELINE_PILOT_ARRAY_QUEUE_EVIDENCE,
        ),
        ("removal_sequence_contract", REMOVAL_SEQUENCE_CONTRACT),
        ("removal_sequence_product", REMOVAL_SEQUENCE_PRODUCT),
        (
            "baseline_pilot_sequence_evidence",
            BASELINE_PILOT_SEQUENCE_EVIDENCE,
        ),
        ("baseline_pilot_v2_contract", BASELINE_PILOT_V2_CONTRACT),
        ("baseline_pilot_v2_product", BASELINE_PILOT_V2_PRODUCT),
        ("baseline_pilot_v2_evidence", BASELINE_PILOT_V2_EVIDENCE),
        (
            "memory_counter_atomic_contract",
            MEMORY_COUNTER_ATOMIC_CONTRACT,
        ),
        (
            "memory_counter_atomic_product",
            MEMORY_COUNTER_ATOMIC_PRODUCT,
        ),
        (
            "baseline_pilot_v2_counter_evidence",
            BASELINE_PILOT_V2_COUNTER_EVIDENCE,
        ),
        (
            "observer_allocation_attribution_contract",
            OBSERVER_ALLOCATION_ATTRIBUTION_CONTRACT,
        ),
        (
            "observer_allocation_attribution_evidence",
            OBSERVER_ALLOCATION_ATTRIBUTION_EVIDENCE,
        ),
        ("shared_entry_tags_contract", SHARED_ENTRY_TAGS_CONTRACT),
        ("shared_entry_tags_product", SHARED_ENTRY_TAGS_PRODUCT),
        (
            "baseline_pilot_v2_freeze_evidence",
            BASELINE_PILOT_V2_FREEZE_EVIDENCE,
        ),
        (
            "w1_owner_classification_contract",
            W1_OWNER_CLASSIFICATION_CONTRACT,
        ),
        (
            "w2_expiry_sweep_profile_contract",
            W2_EXPIRY_SWEEP_PROFILE_CONTRACT,
        ),
        (
            "w2_expiry_sweep_profile_evidence",
            W2_EXPIRY_SWEEP_PROFILE_EVIDENCE,
        ),
        (
            "w2_borrowed_expiry_scan_contract",
            W2_BORROWED_EXPIRY_SCAN_CONTRACT,
        ),
        (
            "w2_borrowed_expiry_scan_evidence",
            W2_BORROWED_EXPIRY_SCAN_EVIDENCE,
        ),
        (
            "w5_hc2_event_copy_profile_contract",
            W5_HC2_EVENT_COPY_PROFILE_CONTRACT,
        ),
        (
            "w5_hc2_event_copy_profile_evidence",
            W5_HC2_EVENT_COPY_PROFILE_EVIDENCE,
        ),
        (
            "w5_shared_event_bytes_contract",
            W5_SHARED_EVENT_BYTES_CONTRACT,
        ),
        (
            "w5_hc2_connection_census_contract",
            W5_HC2_CONNECTION_CENSUS_CONTRACT,
        ),
        (
            "w5_hc2_connection_profile_contract",
            W5_HC2_CONNECTION_PROFILE_CONTRACT,
        ),
        (
            "w5_hc2_connection_profile_evidence",
            W5_HC2_CONNECTION_PROFILE_EVIDENCE,
        ),
        (
            "w5_hc2_split_connection_profile_contract",
            W5_HC2_SPLIT_CONNECTION_PROFILE_CONTRACT,
        ),
        (
            "w5_hc2_split_connection_profile_evidence",
            W5_HC2_SPLIT_CONNECTION_PROFILE_EVIDENCE,
        ),
        (
            "w5_hc2_slow_consumer_profile_contract",
            W5_HC2_SLOW_CONSUMER_PROFILE_CONTRACT,
        ),
        (
            "w5_hc2_slow_consumer_profile_evidence",
            W5_HC2_SLOW_CONSUMER_PROFILE_EVIDENCE,
        ),
        (
            "w5_hc2_raw_transport_stall_contract",
            W5_HC2_RAW_TRANSPORT_STALL_CONTRACT,
        ),
        (
            "w5_hc2_raw_transport_stall_evidence",
            W5_HC2_RAW_TRANSPORT_STALL_EVIDENCE,
        ),
        (
            "w5_hc2_outbound_byte_admission_contract",
            W5_HC2_OUTBOUND_BYTE_ADMISSION_CONTRACT,
        ),
        (
            "w5_hc2_outbound_byte_admission_evidence",
            W5_HC2_OUTBOUND_BYTE_ADMISSION_EVIDENCE,
        ),
        (
            "w6_management_overhead_profile_contract",
            W6_MANAGEMENT_OVERHEAD_PROFILE_CONTRACT,
        ),
        (
            "w6_management_overhead_profile_evidence",
            W6_MANAGEMENT_OVERHEAD_PROFILE_EVIDENCE,
        ),
        (
            "w3_tag_index_profile_contract",
            W3_TAG_INDEX_PROFILE_CONTRACT,
        ),
        (
            "w3_tag_index_profile_evidence",
            W3_TAG_INDEX_PROFILE_EVIDENCE,
        ),
        (
            "w3_shared_event_tags_contract",
            W3_SHARED_EVENT_TAGS_CONTRACT,
        ),
        (
            "w3_shared_event_tags_evidence",
            W3_SHARED_EVENT_TAGS_EVIDENCE,
        ),
    ] {
        if text(root, field) != Some(expected) {
            problems.push(format!("local screening {field} must be {expected}"));
        }
    }
    for field in [
        "promotable",
        "numerical_claims_allowed",
        "candidate_thresholds_allowed",
    ] {
        if boolean(root, field) != Some(false) {
            problems.push(format!("local screening {field} must be false"));
        }
    }
    for field in [
        "require_counterbalanced_order",
        "require_prebuilt_binary",
        "require_binary_sha256",
        "require_scenario_sha256",
        "preserve_failed_attempts",
    ] {
        if boolean(root, field) != Some(true) {
            problems.push(format!("local screening {field} must be true"));
        }
    }
    let required_pairs = integer(root, "minimum_pairs").unwrap_or_default();
    let recommended_pairs = integer(root, "recommended_pairs").unwrap_or_default();
    if required_pairs < 3 || recommended_pairs < required_pairs {
        problems.push(
            "local screening requires at least three pairs and a recommendation no lower than the minimum"
                .to_owned(),
        );
    }
    if text(root, "non_promotion_reason").is_none_or(str::is_empty) {
        problems.push("local screening requires a non_promotion_reason".to_owned());
    }
    if !string_array(root.get("allowed_promotion_targets")).is_empty() {
        problems.push("local screening must not declare promotion targets".to_owned());
    }
    let expected_concurrency = [1_i64, 8, 32, 128];
    if integer_array(root.get("concurrency_lanes")) != expected_concurrency {
        problems.push("local screening concurrency_lanes must be 1,8,32,128".to_owned());
    }
    let load_fractions = float_array(root.get("offered_load_fractions"));
    if load_fractions.len() != 3
        || load_fractions
            .iter()
            .zip([0.25, 0.60, 0.85])
            .any(|(actual, expected)| (actual - expected).abs() > f64::EPSILON)
    {
        problems.push("local screening offered_load_fractions must be 0.25,0.60,0.85".to_owned());
    }
    if float(root, "maximum_goodput_regression") != Some(0.02) {
        problems.push("local screening maximum_goodput_regression must remain 0.02".to_owned());
    }
    if string_array(root.get("allowed_instrumentation_modes")) != ["off", "production", "profile"] {
        problems.push(
            "local screening allowed_instrumentation_modes must be off,production,profile"
                .to_owned(),
        );
    }
    let outcomes: BTreeSet<_> = string_array(root.get("required_outcome_fields"))
        .into_iter()
        .collect();
    let expected: BTreeSet<_> = [
        "attempted",
        "success",
        "rejected",
        "timeout",
        "late",
        "incomplete",
    ]
    .into_iter()
    .collect();
    if outcomes != expected {
        problems.push("local screening outcome accounting is incomplete".to_owned());
    }
    problems
}

pub fn check_local_overhead_screening(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_class") != Some("local_screening")
    {
        problems.push("local overhead screening identity mismatch".to_owned());
    }
    if boolean(value, "promotable") != Some(false)
        || boolean(value, "numerical_claim_eligible") != Some(false)
        || text(value, "thresholds_status") != Some("screening_only_unqualified")
        || text(value, "decision") != Some("blocked-instrumentation-overhead")
    {
        problems.push("local overhead screening must remain a non-promotable blocker".to_owned());
    }
    for field in ["source_sha"] {
        if text(value, field).is_none_or(|sha| !full_sha(sha)) {
            problems.push(format!(
                "local overhead screening {field} is not a full SHA"
            ));
        }
    }
    for field in [
        "binary_sha256",
        "host_fingerprint_sha256",
        "screening_sha256",
    ] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!("local overhead screening {field} is not SHA-256"));
        }
    }
    if integer(value, "pair_count").is_none_or(|count| count < 3)
        || boolean(value, "counterbalanced") != Some(true)
        || integer(value, "failed_attempts") != Some(0)
    {
        problems.push(
            "local overhead screening lacks three successful counterbalanced pairs".to_owned(),
        );
    }
    let observations = value
        .get("median_observations")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    let metrics: BTreeSet<_> = observations
        .iter()
        .filter_map(|item| text(item, "metric"))
        .collect();
    for required in [
        "elapsed_ns",
        "fill_allocated_bytes_per_operation",
        "steady_allocated_bytes_per_operation",
        "expire_delete_allocated_bytes_per_operation",
        "refill_allocated_bytes_per_operation",
        "post_idle_rss_delta_bytes",
        "peak_rss_delta_bytes",
    ] {
        if !metrics.contains(required) {
            problems.push(format!("local overhead screening omits {required}"));
        }
    }
    for item in &observations {
        for field in ["off", "production", "regression_fraction"] {
            if float(item, field).is_none_or(|number| !number.is_finite()) {
                problems.push(format!("local overhead observation has invalid {field}"));
            }
        }
    }
    if text(value, "suspected_owner").is_none_or(str::is_empty)
        || text(value, "next_evidence").is_none_or(str::is_empty)
    {
        problems.push(
            "local overhead screening requires owner hypothesis and next evidence".to_owned(),
        );
    }
    problems
}

pub fn check_local_overhead_isolation(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_class") != Some("local_screening")
        || text(value, "profile_id") != Some("instrumentation-overhead-counters-only-073-v1")
    {
        problems.push("local overhead isolation identity mismatch".to_owned());
    }
    if boolean(value, "diagnostic_only") != Some(true)
        || boolean(value, "counter_correctness_eligible") != Some(false)
        || boolean(value, "promotable") != Some(false)
        || boolean(value, "numerical_claim_eligible") != Some(false)
        || text(value, "thresholds_status") != Some("screening_only_unqualified")
        || text(value, "decision") != Some("listener-cost-attributed-local-diagnostic")
    {
        problems
            .push("local overhead isolation must remain diagnostic and non-promotable".to_owned());
    }
    if text(value, "source_sha").is_none_or(|sha| !full_sha(sha)) {
        problems.push("local overhead isolation source_sha is not a full SHA".to_owned());
    }
    for field in [
        "binary_sha256",
        "host_fingerprint_sha256",
        "screening_sha256",
    ] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!("local overhead isolation {field} is not SHA-256"));
        }
    }
    if integer(value, "pair_count").is_none_or(|count| count < 3)
        || boolean(value, "counterbalanced") != Some(true)
        || integer(value, "failed_attempts") != Some(0)
    {
        problems.push(
            "local overhead isolation lacks three successful counterbalanced pairs".to_owned(),
        );
    }
    let observations = value
        .get("median_observations")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    let metrics: BTreeSet<_> = observations
        .iter()
        .filter_map(|item| text(item, "metric"))
        .collect();
    for required in [
        "elapsed_ns",
        "fill_allocated_bytes_per_operation",
        "steady_allocated_bytes_per_operation",
        "expire_delete_allocated_bytes_per_operation",
        "refill_allocated_bytes_per_operation",
        "post_idle_rss_delta_bytes",
        "peak_rss_delta_bytes",
    ] {
        if !metrics.contains(required) {
            problems.push(format!("local overhead isolation omits {required}"));
        }
    }
    for item in &observations {
        for field in ["off", "production", "regression_fraction"] {
            if float(item, field).is_none_or(|number| !number.is_finite()) {
                problems.push(format!("local overhead isolation has invalid {field}"));
            }
        }
    }
    for field in ["isolated_factor", "conclusion", "next_evidence"] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!("local overhead isolation requires {field}"));
        }
    }
    problems
}

pub fn check_local_overhead_listener_noop(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_class") != Some("local_screening")
        || text(value, "profile_id") != Some("instrumentation-overhead-listener-noop-073-v1")
    {
        problems.push("local no-op listener screening identity mismatch".to_owned());
    }
    if boolean(value, "diagnostic_only") != Some(true)
        || boolean(value, "counter_correctness_eligible") != Some(false)
        || boolean(value, "promotable") != Some(false)
        || boolean(value, "numerical_claim_eligible") != Some(false)
        || text(value, "thresholds_status") != Some("screening_only_unqualified")
        || text(value, "decision") != Some("backend-notification-cost-attributed-local-diagnostic")
    {
        problems.push(
            "local no-op listener screening must remain diagnostic and non-promotable".to_owned(),
        );
    }
    if text(value, "source_sha").is_none_or(|sha| !full_sha(sha)) {
        problems.push("local no-op listener source_sha is not a full SHA".to_owned());
    }
    for field in [
        "binary_sha256",
        "host_fingerprint_sha256",
        "screening_sha256",
    ] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!("local no-op listener {field} is not SHA-256"));
        }
    }
    if integer(value, "pair_count").is_none_or(|count| count < 3)
        || boolean(value, "counterbalanced") != Some(true)
        || integer(value, "failed_attempts") != Some(0)
    {
        problems.push(
            "local no-op listener screening lacks three successful counterbalanced pairs"
                .to_owned(),
        );
    }
    let observations = value
        .get("median_observations")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    let metrics: BTreeSet<_> = observations
        .iter()
        .filter_map(|item| text(item, "metric"))
        .collect();
    for required in [
        "elapsed_ns",
        "fill_allocated_bytes_per_operation",
        "steady_allocated_bytes_per_operation",
        "expire_delete_allocated_bytes_per_operation",
        "refill_allocated_bytes_per_operation",
        "post_idle_rss_delta_bytes",
        "peak_rss_delta_bytes",
    ] {
        if !metrics.contains(required) {
            problems.push(format!("local no-op listener screening omits {required}"));
        }
    }
    for item in &observations {
        for field in ["off", "production", "regression_fraction"] {
            if float(item, field).is_none_or(|number| !number.is_finite()) {
                problems.push(format!(
                    "local no-op listener screening has invalid {field}"
                ));
            }
        }
    }
    for field in ["isolated_factor", "conclusion", "next_evidence"] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!("local no-op listener screening requires {field}"));
        }
    }
    problems
}

pub fn check_notification_feasibility(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_class") != Some("local_feasibility")
        || text(value, "experiment") != Some("moka-future-vs-sync-noop-listener")
    {
        problems.push("notification feasibility identity mismatch".to_owned());
    }
    if boolean(value, "diagnostic_only") != Some(true)
        || boolean(value, "product_semantics_eligible") != Some(false)
        || boolean(value, "promotable") != Some(false)
        || text(value, "decision") != Some("sync-backend-insufficient")
    {
        problems.push(
            "notification feasibility must remain diagnostic, non-promotable, and insufficient for sync migration"
                .to_owned(),
        );
    }
    if text(value, "source_sha").is_none_or(|sha| !full_sha(sha)) {
        problems.push("notification feasibility source_sha is not a full SHA".to_owned());
    }
    for field in ["binary_sha256", "receipt_sha256"] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!("notification feasibility {field} is not SHA-256"));
        }
    }
    if integer(value, "operations_per_case").is_none_or(|count| count < 1024)
        || integer(value, "repetitions").is_none_or(|count| count < 3)
        || boolean(value, "counterbalanced") != Some(true)
    {
        problems.push(
            "notification feasibility requires three counterbalanced repetitions of at least 1024 operations"
                .to_owned(),
        );
    }

    let observations = value
        .get("median_observations")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    for (backend, operation) in [
        ("moka-future", "insert"),
        ("moka-future", "remove"),
        ("moka-sync", "insert"),
        ("moka-sync", "remove"),
    ] {
        if !observations.iter().any(|item| {
            text(item, "backend") == Some(backend) && text(item, "operation") == Some(operation)
        }) {
            problems.push(format!(
                "notification feasibility omits {backend} {operation}"
            ));
        }
    }
    for item in &observations {
        for field in [
            "listener_off_bytes_per_operation",
            "listener_noop_bytes_per_operation",
            "incremental_bytes_per_operation",
            "regression_fraction",
        ] {
            if float(item, field).is_none_or(|number| !number.is_finite() || number < 0.0) {
                problems.push(format!("notification feasibility has invalid {field}"));
            }
        }
    }
    for field in ["conclusion", "next_evidence"] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!("notification feasibility requires {field}"));
        }
    }
    problems
}

pub fn check_notification_observer_requirements(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "contract_id") != Some("notification-observer-requirements-073-v1")
        || text(value, "state") != Some("local-screening-passed-awaiting-d3")
        || text(value, "proposal_id") != Some("P73-INSTRUMENTATION-NONBLOCKING-REMOVAL")
        || text(value, "prototype_scope") != Some("completed-lab-prototype")
    {
        problems.push("notification observer requirements identity mismatch".to_owned());
    }
    if boolean(value, "d2_authorized") != Some(true)
        || boolean(value, "product_mutation_allowed") != Some(true)
        || boolean(value, "candidate_measurements_allowed") != Some(false)
        || boolean(value, "local_candidate_screening_allowed") != Some(true)
        || boolean(value, "local_candidate_screening_completed") != Some(true)
        || text(value, "review_status") != Some("d2-authorized-pinned-fork")
    {
        problems.push("notification observer requirements must bind D2 to the pinned fork while candidate measurement remains disabled".to_owned());
    }
    for field in [
        "selected_direction",
        "prototype_exit",
        "implementation_commit",
        "implementation_receipt",
        "local_screening_evidence",
        "next_evidence",
    ] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!(
                "notification observer requirements require {field}"
            ));
        }
    }
    for (field, minimum) in [
        ("owner_split", 3),
        ("callback_constraints", 5),
        ("ordering_invariants", 5),
        ("delivery_invariants", 5),
        ("required_falsifiers", 6),
        ("compatibility_invariants", 4),
    ] {
        if string_array(value.get(field)).len() < minimum {
            problems.push(format!(
                "notification observer requirements have incomplete {field}"
            ));
        }
    }
    problems
}

pub fn check_notification_observer_prototype(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_class") != Some("local_feasibility")
        || text(value, "experiment") != Some("versioned-bounded-post-removal-observer")
    {
        problems.push("notification observer prototype identity mismatch".to_owned());
    }
    if boolean(value, "diagnostic_only") != Some(true)
        || boolean(value, "product_semantics_eligible") != Some(false)
        || boolean(value, "promotable") != Some(false)
        || text(value, "decision") != Some("reference-model-feasible-moka-seam-unproven")
    {
        problems.push(
            "notification observer prototype must remain diagnostic with the Moka seam unproven"
                .to_owned(),
        );
    }
    if text(value, "source_sha").is_none_or(|sha| !full_sha(sha)) {
        problems.push("notification observer prototype source_sha is not a full SHA".to_owned());
    }
    for field in ["binary_sha256", "receipt_sha256"] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!(
                "notification observer prototype {field} is not SHA-256"
            ));
        }
    }
    if integer(value, "operations_per_case").is_none_or(|count| count < 1024)
        || integer(value, "repetitions").is_none_or(|count| count < 3)
        || boolean(value, "counterbalanced") != Some(true)
    {
        problems.push(
            "notification observer prototype requires three counterbalanced repetitions of at least 1024 operations"
                .to_owned(),
        );
    }
    let observations = value
        .get("median_observations")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    for path in ["atomic-counter-only", "versioned-observer"] {
        if !observations
            .iter()
            .any(|item| text(item, "path") == Some(path))
        {
            problems.push(format!("notification observer prototype omits {path}"));
        }
    }
    for item in &observations {
        for field in ["gross_allocated_bytes_per_operation", "elapsed_ns"] {
            if float(item, field).is_none_or(|number| !number.is_finite() || number < 0.0) {
                problems.push(format!(
                    "notification observer prototype has invalid {field}"
                ));
            }
        }
    }
    for field in ["conclusion", "next_evidence"] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!("notification observer prototype requires {field}"));
        }
    }
    problems
}

pub fn check_moka_observer_spike(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_class") != Some("local_feasibility")
        || text(value, "experiment") != Some("moka-future-listener-vs-observer-key-lock-ablation")
    {
        problems.push("Moka observer spike identity mismatch".to_owned());
    }
    if boolean(value, "diagnostic_only") != Some(true)
        || boolean(value, "product_semantics_eligible") != Some(false)
        || boolean(value, "promotable") != Some(false)
        || text(value, "decision") != Some("key-lock-owner-confirmed-boxed-removal-cost-remains")
    {
        problems.push(
            "Moka observer spike must remain diagnostic with boxed removal cost unresolved"
                .to_owned(),
        );
    }
    if text(value, "source_sha").is_none_or(|sha| !full_sha(sha)) {
        problems.push("Moka observer spike source_sha is not a full SHA".to_owned());
    }
    for field in ["binary_sha256", "receipt_sha256", "patch_sha256"] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!("Moka observer spike {field} is not SHA-256"));
        }
    }
    if integer(value, "operations_per_case").is_none_or(|count| count < 1024)
        || integer(value, "repetitions").is_none_or(|count| count < 3)
        || boolean(value, "counterbalanced") != Some(true)
    {
        problems.push(
            "Moka observer spike requires three counterbalanced repetitions of at least 1024 operations"
                .to_owned(),
        );
    }
    let causes: BTreeSet<_> = string_array(value.get("verified_causes"))
        .into_iter()
        .collect();
    for cause in ["explicit", "replaced", "expired", "size"] {
        if !causes.contains(cause) {
            problems.push(format!("Moka observer spike omits {cause} cause"));
        }
    }
    let observations = value
        .get("median_observations")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    for mode in ["off", "listener", "observer"] {
        for operation in ["insert", "remove"] {
            if !observations.iter().any(|item| {
                text(item, "mode") == Some(mode) && text(item, "operation") == Some(operation)
            }) {
                problems.push(format!("Moka observer spike omits {mode} {operation}"));
            }
        }
    }
    for item in &observations {
        for field in ["gross_allocated_bytes_per_operation", "elapsed_ns"] {
            if float(item, field).is_none_or(|number| !number.is_finite() || number < 0.0) {
                problems.push(format!("Moka observer spike has invalid {field}"));
            }
        }
    }
    for field in ["conclusion", "next_evidence"] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!("Moka observer spike requires {field}"));
        }
    }
    problems
}

pub fn check_moka_observer_direct(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_class") != Some("local_feasibility")
        || text(value, "experiment") != Some("moka-direct-observer-with-versioned-cleanup")
    {
        problems.push("direct Moka observer identity mismatch".to_owned());
    }
    if boolean(value, "diagnostic_only") != Some(true)
        || boolean(value, "product_semantics_eligible") != Some(false)
        || boolean(value, "promotable") != Some(false)
        || boolean(value, "d2_authorized") != Some(false)
        || text(value, "decision") != Some("direct-observer-lab-feasible-d2-still-required")
    {
        problems
            .push("direct Moka observer must remain lab-only until D2 is authorized".to_owned());
    }
    if text(value, "source_sha").is_none_or(|sha| !full_sha(sha)) {
        problems.push("direct Moka observer source_sha is not a full SHA".to_owned());
    }
    for field in ["binary_sha256", "receipt_sha256", "patch_sha256"] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!("direct Moka observer {field} is not SHA-256"));
        }
    }
    if integer(value, "operations_per_case").is_none_or(|count| count < 1024)
        || integer(value, "repetitions").is_none_or(|count| count < 3)
        || boolean(value, "counterbalanced") != Some(true)
    {
        problems.push(
            "direct Moka observer requires three counterbalanced repetitions of at least 1024 operations"
                .to_owned(),
        );
    }
    let causes: BTreeSet<_> = string_array(value.get("verified_causes"))
        .into_iter()
        .collect();
    for cause in ["explicit", "replaced", "expired", "size"] {
        if !causes.contains(cause) {
            problems.push(format!("direct Moka observer omits {cause} cause"));
        }
    }
    if boolean(value, "versioned_replacement_ordering_verified") != Some(true) {
        problems
            .push("direct Moka observer requires versioned replacement ordering proof".to_owned());
    }
    let observations = value
        .get("median_observations")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    for mode in ["off", "listener", "observer"] {
        for operation in ["insert", "remove"] {
            if !observations.iter().any(|item| {
                text(item, "mode") == Some(mode) && text(item, "operation") == Some(operation)
            }) {
                problems.push(format!("direct Moka observer omits {mode} {operation}"));
            }
        }
    }
    for item in &observations {
        for field in ["gross_allocated_bytes_per_operation", "elapsed_ns"] {
            if float(item, field).is_none_or(|number| !number.is_finite() || number < 0.0) {
                problems.push(format!("direct Moka observer has invalid {field}"));
            }
        }
    }
    for field in ["conclusion", "next_evidence"] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!("direct Moka observer requires {field}"));
        }
    }
    problems
}

pub fn check_notification_observer_d2_review(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "proposal_id") != Some("P73-INSTRUMENTATION-NONBLOCKING-REMOVAL")
        || text(value, "review_packet_id") != Some("notification-observer-d2-review-v1")
        || text(value, "state") != Some("d2-authorized-pinned-fork")
    {
        problems.push("notification observer D2 review candidate identity mismatch".to_owned());
    }
    if text(value, "reviewer") != Some("project-maintainer-self-review")
        || boolean(value, "reviewer_independent") != Some(false)
        || boolean(value, "d2_authorized") != Some(true)
        || boolean(value, "thresholds_frozen") != Some(true)
        || boolean(value, "candidate_measurements_allowed") != Some(false)
        || boolean(value, "product_mutation_allowed") != Some(true)
        || boolean(value, "candidate_data_used_for_thresholds") != Some(false)
    {
        problems.push(
            "notification observer single-maintainer review must authorize only product integration and keep candidate measurement disabled"
                .to_owned(),
        );
    }
    for (field, expected) in [
        (
            "review_mode",
            "single-maintainer-with-compensating-controls",
        ),
        ("review_policy", SINGLE_MAINTAINER_REVIEW_POLICY),
        (
            "threshold_freeze_commit",
            "1e9fd748f1a234967e9303c7071dd0816fb7ac28",
        ),
        ("reviewed_at", "2026-09-24"),
        (
            "review_decision",
            "thresholds-accepted-pinned-fork-authorized",
        ),
    ] {
        if text(value, field) != Some(expected) {
            problems.push(format!(
                "notification observer single-maintainer review {field} must be {expected}"
            ));
        }
    }
    if text(value, "primary_metric") != Some("fill_allocated_bytes_per_operation")
        || float(value, "minimum_practical_improvement_fraction")
            .is_none_or(|number| (number - 0.15).abs() > f64::EPSILON)
    {
        problems.push("notification observer D2 review primary threshold changed".to_owned());
    }
    for (field, expected) in [
        ("dependency_preference", "upstream-first"),
        ("dependency_fallback", "reviewed-pinned-fork"),
        ("dependency_decision", MOKA_FORK_DECISION),
        ("dependency_review_draft", MOKA_OBSERVER_UPSTREAM_DRAFT),
    ] {
        if text(value, field) != Some(expected) {
            problems.push(format!(
                "notification observer D2 review {field} must be {expected}"
            ));
        }
    }
    for field in [
        "author_role",
        "compatibility_outcome",
        "rollback_class",
        "threshold_derivation",
        "reviewer_action",
        "implementation_scope_adjudication",
        "implementation_receipt",
    ] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!("notification observer D2 review requires {field}"));
        }
    }
    for (field, minimum) in [
        ("baseline_evidence", 4),
        ("candidate_evidence_excluded_from_threshold_derivation", 3),
        ("authorized_files_if_d2_approved", 6),
        ("authorized_surfaces_if_d2_approved", 4),
        ("required_correctness_falsifiers", 10),
        ("required_dependency_review", 4),
        ("required_d3_measurements", 7),
    ] {
        if string_array(value.get(field)).len() < minimum {
            problems.push(format!(
                "notification observer D2 review has incomplete {field}"
            ));
        }
    }
    let baseline: BTreeSet<_> = string_array(value.get("baseline_evidence"))
        .into_iter()
        .collect();
    let excluded: BTreeSet<_> =
        string_array(value.get("candidate_evidence_excluded_from_threshold_derivation"))
            .into_iter()
            .collect();
    if !baseline.is_disjoint(&excluded) {
        problems.push(
            "notification observer D2 review mixes candidate evidence into threshold derivation"
                .to_owned(),
        );
    }
    let falsifiers = string_array(value.get("required_correctness_falsifiers"));
    for required in ["panic", "reentrancy"] {
        if !falsifiers.iter().any(|item| item.contains(required)) {
            problems.push(format!(
                "notification observer D2 review omits {required} falsifier"
            ));
        }
    }
    let thresholds = value
        .get("threshold_proposal")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    let fill = thresholds
        .iter()
        .find(|item| text(item, "metric") == Some("fill_allocated_bytes_per_operation"));
    if fill.is_none_or(|item| {
        text(item, "role") != Some("primary")
            || text(item, "direction") != Some("lower")
            || float(item, "minimum_relative_improvement")
                .is_none_or(|number| (number - 0.15).abs() > f64::EPSILON)
    }) {
        problems.push("notification observer D2 review has invalid fill threshold".to_owned());
    }
    let allocations = thresholds
        .iter()
        .find(|item| text(item, "metric") == Some("unaffected_allocated_bytes_per_operation"));
    if allocations.is_none_or(|item| {
        float(item, "maximum_relative_regression")
            .is_none_or(|number| (number - 0.03).abs() > f64::EPSILON)
            || integer(item, "absolute_noise_floor_bytes_per_operation") != Some(16)
    }) {
        problems.push("notification observer D2 review has invalid allocation guard".to_owned());
    }
    let rss = thresholds
        .iter()
        .find(|item| text(item, "metric") == Some("post_idle_and_peak_rss_delta_bytes"));
    if rss.is_none_or(|item| {
        float(item, "maximum_relative_regression")
            .is_none_or(|number| (number - 0.05).abs() > f64::EPSILON)
            || integer(item, "absolute_noise_floor_bytes") != Some(1_048_576)
    }) {
        problems.push("notification observer D2 review has invalid RSS guard".to_owned());
    }
    problems
}

pub fn check_single_maintainer_review_policy(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "policy_id") != Some("single-maintainer-review-073-v1")
        || text(value, "state") != Some("active")
        || text(value, "scope") != Some("P73-INSTRUMENTATION-NONBLOCKING-REMOVAL")
        || text(value, "review_mode") != Some("single-maintainer-with-compensating-controls")
    {
        problems.push("single-maintainer review policy identity mismatch".to_owned());
    }
    if boolean(value, "independent_reviewer_required") != Some(false)
        || boolean(value, "independent_review_claim_allowed") != Some(false)
        || boolean(value, "thresholds_accepted") != Some(true)
        || text(value, "dependency_decision") != Some(MOKA_FORK_DECISION)
        || boolean(value, "d2_authorized") != Some(true)
        || boolean(value, "product_mutation_allowed") != Some(true)
    {
        problems.push(
            "single-maintainer policy must bind D2 to the reviewed dependency without claiming independence"
                .to_owned(),
        );
    }
    for (field, expected) in [
        (
            "threshold_freeze_commit",
            "1e9fd748f1a234967e9303c7071dd0816fb7ac28",
        ),
        (
            "dependency_review_draft_commit",
            "fb1e850d536c0ed970f5043f09ad3ce3582704f0",
        ),
    ] {
        if text(value, field) != Some(expected) {
            problems.push(format!(
                "single-maintainer policy {field} must be {expected}"
            ));
        }
    }
    if text(value, "authorization_source").is_none_or(str::is_empty) {
        problems.push("single-maintainer policy requires authorization_source".to_owned());
    }
    for (field, minimum) in [
        ("compensating_controls", 10),
        ("decision_sequence", 6),
        ("forbidden_shortcuts", 5),
    ] {
        if string_array(value.get(field)).len() < minimum {
            problems.push(format!("single-maintainer policy has incomplete {field}"));
        }
    }
    let controls = string_array(value.get("compensating_controls"));
    for required in [
        "threshold",
        "candidate evidence",
        "separate",
        "append-only",
        "dedicated host",
        "independently reviewed",
        "panic",
        "rollback",
    ] {
        if !controls.iter().any(|item| item.contains(required)) {
            problems.push(format!(
                "single-maintainer policy omits {required} compensating control"
            ));
        }
    }
    problems
}

pub fn check_moka_fork_decision(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "decision_id") != Some("moka-fork-352e53fa-073-v1")
        || text(value, "proposal_id") != Some("P73-INSTRUMENTATION-NONBLOCKING-REMOVAL")
        || text(value, "state") != Some("d2-authorized")
        || text(value, "decision") != Some("reviewed-pinned-fork")
        || text(value, "review_mode") != Some("single-maintainer-with-compensating-controls")
        || text(value, "review_policy") != Some(SINGLE_MAINTAINER_REVIEW_POLICY)
    {
        problems.push("Moka fork decision identity mismatch".to_owned());
    }
    if boolean(value, "d2_authorized") != Some(true)
        || boolean(value, "product_mutation_allowed") != Some(true)
        || boolean(value, "candidate_measurements_allowed") != Some(false)
    {
        problems.push(
            "Moka fork decision must authorize product integration but not candidate measurement"
                .to_owned(),
        );
    }

    let missing = TomlValue::Boolean(false);
    let source = value.get("source").unwrap_or(&missing);
    for (field, expected) in [
        ("repository", "https://github.com/javaquasar/moka.git"),
        ("branch", "hydracache/post-removal-observer-0.12.15"),
        ("revision", "352e53faa480c9997272b9c70798dd5b5c15d581"),
        (
            "upstream_revision",
            "616473ee923f4cd1429b3d8eb3be7df3eb9906b1",
        ),
        (
            "prototype_patch",
            "docs/testing/performance/0.73/moka-post-removal-observer-direct-0.12.15.patch",
        ),
    ] {
        if text(source, field) != Some(expected) {
            problems.push(format!(
                "Moka fork decision source {field} must be {expected}"
            ));
        }
    }
    for field in [
        "revision",
        "tree",
        "upstream_revision",
        "upstream_tree",
        "stable_patch_id",
    ] {
        if text(source, field).is_none_or(|digest| !full_sha(digest)) {
            problems.push(format!(
                "Moka fork decision source {field} is not a full digest"
            ));
        }
    }
    if text(source, "prototype_patch_sha256").is_none_or(|digest| !sha256(digest))
        || boolean(source, "remote_revision_verified") != Some(true)
    {
        problems.push("Moka fork decision source integrity is incomplete".to_owned());
    }

    let upstream = value.get("upstream").unwrap_or(&missing);
    if text(upstream, "proposal") != Some(MOKA_OBSERVER_UPSTREAM_DRAFT)
        || text(upstream, "submission_state") != Some("not-submitted")
        || text(upstream, "maintainer_disposition") != Some("not-requested")
        || text(upstream, "decision").is_none_or(str::is_empty)
        || text(upstream, "rationale").is_none_or(str::is_empty)
    {
        problems.push("Moka fork decision must disclose the absent upstream review".to_owned());
    }

    let maintenance = value.get("maintenance").unwrap_or(&missing);
    for field in [
        "repository_owner",
        "integration_owner",
        "upstream_sync_cadence",
        "advisory_cadence",
        "update_policy",
        "rollback",
    ] {
        if text(maintenance, field).is_none_or(str::is_empty) {
            problems.push(format!("Moka fork decision maintenance requires {field}"));
        }
    }

    let compatibility = value.get("compatibility").unwrap_or(&missing);
    if text(compatibility, "license_expression") != Some("(MIT OR Apache-2.0) AND Apache-2.0")
        || text(compatibility, "msrv") != Some("1.71.1")
        || string_array(compatibility.get("selected_features")) != ["future"]
        || boolean(compatibility, "observer_public_api_exposed_by_hydracache") != Some(false)
        || boolean(compatibility, "default_hydracache_behavior_changed") != Some(false)
    {
        problems.push("Moka fork decision compatibility contract changed".to_owned());
    }

    let supply_chain = value.get("supply_chain").unwrap_or(&missing);
    for field in ["cargo_lock_sha256", "sbom_sha256"] {
        if text(supply_chain, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!("Moka fork decision {field} is not SHA-256"));
        }
    }
    if text(supply_chain, "cargo_deny_result")
        != Some("advisories ok, bans ok, licenses ok, sources ok")
        || text(supply_chain, "sbom_format") != Some("CycloneDX 1.5 JSON")
    {
        problems.push("Moka fork decision supply-chain gate is incomplete".to_owned());
    }
    if string_array(value.get("validation")).len() < 9
        || string_array(value.get("authorization_limits")).len() < 4
    {
        problems.push("Moka fork decision omits validation or authorization limits".to_owned());
    }
    problems
}

pub fn check_notification_observer_product(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_id") != Some("notification-observer-product-73fc38a1-v1")
        || text(value, "proposal_id") != Some("P73-INSTRUMENTATION-NONBLOCKING-REMOVAL")
        || text(value, "state") != Some("local-correctness-admitted")
        || text(value, "implementation_commit") != Some("73fc38a131d26e78b246fe93d5edd71d33796bbf")
        || text(value, "implementation_parent") != Some("03f893541f6386cbf026c496250e28950c05e0ab")
        || text(value, "dependency_revision") != Some("352e53faa480c9997272b9c70798dd5b5c15d581")
    {
        problems.push("notification observer product admission identity mismatch".to_owned());
    }
    if text(value, "review_mode") != Some("single-maintainer-with-compensating-controls")
        || boolean(value, "promotable") != Some(false)
        || boolean(value, "numerical_claim_eligible") != Some(false)
        || boolean(value, "local_candidate_screening_allowed") != Some(true)
        || boolean(value, "dedicated_candidate_measurement_allowed") != Some(false)
        || boolean(value, "thresholds_changed") != Some(false)
        || boolean(value, "public_api_changed") != Some(false)
        || boolean(value, "instrumentation_off_observer_attached") != Some(false)
        || boolean(value, "rollback_verified") != Some(true)
    {
        problems.push(
            "product admission must open only non-promotable local screening without changing thresholds or public behavior"
                .to_owned(),
        );
    }
    if integer(value, "observer_queue_capacity") != Some(4_096)
        || integer(value, "cache_entry_inline_bytes") != Some(72)
    {
        problems
            .push("product admission observer bounds or frozen entry layout changed".to_owned());
    }
    for field in [
        "scope_variance",
        "shutdown_conclusion",
        "reentrancy_conclusion",
        "next_evidence",
    ] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!("product admission requires {field}"));
        }
    }
    let expected_files: BTreeSet<_> = [
        "Cargo.lock",
        "Cargo.toml",
        "crates/hydracache/src/builder.rs",
        "crates/hydracache/src/cache.rs",
        "crates/hydracache/src/entry.rs",
        "crates/hydracache/src/lib.rs",
        "crates/hydracache/src/memory_footprint.rs",
        "crates/hydracache/src/removal_observer.rs",
        "crates/hydracache/src/tag_index.rs",
        "crates/hydracache/src/tests/local_cache.rs",
        "crates/hydracache/tests/memory_footprint_071.rs",
        "deny.toml",
    ]
    .into_iter()
    .collect();
    let changed_files: BTreeSet<_> = string_array(value.get("changed_files"))
        .into_iter()
        .collect();
    if changed_files != expected_files {
        problems.push(
            "product admission changed-file ledger does not match implementation commit".to_owned(),
        );
    }
    if string_array(value.get("validation")).len() < 15 {
        problems.push("product admission validation matrix is incomplete".to_owned());
    }
    let falsifiers = value
        .get("falsifier")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    for id in [
        "explicit-replace-invalidate-flush",
        "ttl-expiry",
        "capacity-eviction",
        "delayed-replacement-cleanup",
        "duplicate-delivery",
        "queue-saturation",
        "pending-exact-barrier",
        "cancellation-gap",
        "panic-containment",
        "reentrancy",
        "shutdown",
        "default-compatibility",
        "rollback",
    ] {
        if !falsifiers.iter().any(|item| {
            text(item, "id") == Some(id)
                && text(item, "status") == Some("passed")
                && text(item, "evidence").is_some_and(|evidence| !evidence.is_empty())
        }) {
            problems.push(format!("product admission omits passing {id} falsifier"));
        }
    }
    problems
}

pub fn check_local_observer_product_screening(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_id") != Some("local-observer-product-screening-06a8bd95-v1")
        || text(value, "evidence_class") != Some("local_screening")
        || text(value, "state") != Some("passed-local-rejection-gates-awaiting-d3")
        || text(value, "proposal_id") != Some("P73-INSTRUMENTATION-NONBLOCKING-REMOVAL")
        || text(value, "implementation_commit") != Some("73fc38a131d26e78b246fe93d5edd71d33796bbf")
        || text(value, "baseline_source_sha") != Some("03f893541f6386cbf026c496250e28950c05e0ab")
        || text(value, "candidate_source_sha") != Some("06a8bd95c9650abd5a77aa480bc1cf1b58d1fd05")
        || text(value, "dependency_revision") != Some("352e53faa480c9997272b9c70798dd5b5c15d581")
    {
        problems.push("local observer product screening identity mismatch".to_owned());
    }
    for (field, expected) in [
        (
            "host_fingerprint_sha256",
            "6925691d8e0ce5d649fb02ae0f4c91f7be667aede4d0d23f62b24ac8cd34cc65",
        ),
        (
            "baseline_context_sha256",
            "1158e0b5c29415f83615727a702a798cbd66d44ca0ff2c215f936c078dd4262d",
        ),
        (
            "candidate_context_sha256",
            "568ce7ff03154a0445d4d1e3d059ccb28be61349578e2821f8e9c45858339d9b",
        ),
        (
            "baseline_binary_sha256",
            "f520e953254f8668b6fecb4fc0e249d4667ee69f22642c5af235cb36c52ef445",
        ),
        (
            "candidate_binary_sha256",
            "aaa1577bfd153b05860fe9e2466fddeefb0d46cdb1fd2ce57c99792449941180",
        ),
        (
            "baseline_screening_sha256",
            "a85ca96f470957d3e5895be566d1d2ea1f2b71d767e19ad351b4794116b73e25",
        ),
        (
            "candidate_screening_sha256",
            "b48c91d2717c93cdcbff4bb341c2d64718970634942b7d58fa3192856a50fe03",
        ),
    ] {
        if text(value, field) != Some(expected) {
            problems.push(format!(
                "local observer product screening {field} does not match retained evidence"
            ));
        }
    }
    for field in [
        "host_fingerprint_sha256",
        "baseline_context_sha256",
        "candidate_context_sha256",
        "baseline_binary_sha256",
        "candidate_binary_sha256",
        "baseline_screening_sha256",
        "candidate_screening_sha256",
    ] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!(
                "local observer product screening {field} is not SHA-256"
            ));
        }
    }
    if integer(value, "run_order_seed") != Some(7_302)
        || integer(value, "baseline_pair_count").is_none_or(|count| count < 5)
        || integer(value, "candidate_pair_count").is_none_or(|count| count < 5)
        || integer(value, "failed_attempts") != Some(0)
        || boolean(value, "counterbalanced_within_each_source") != Some(true)
        || boolean(value, "same_host_fingerprint") != Some(true)
        || boolean(value, "clean_source_contexts") != Some(true)
    {
        problems.push(
            "local observer product screening identity or attempt set is incomplete".to_owned(),
        );
    }
    if boolean(value, "promotable") != Some(false)
        || boolean(value, "numerical_claim_eligible") != Some(false)
        || boolean(value, "thresholds_changed") != Some(false)
        || text(value, "thresholds_status") != Some("screening_only_unqualified")
        || text(value, "decision")
            != Some("local-screening-passed-awaiting-dedicated-host-qualification")
    {
        problems.push(
            "local observer product screening must remain non-promotable with frozen thresholds"
                .to_owned(),
        );
    }
    for field in ["primary_conclusion", "limitations", "next_evidence"] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!("local observer product screening requires {field}"));
        }
    }
    let metrics = value
        .get("metric")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    for name in [
        "elapsed_ns",
        "fill_allocated_bytes_per_operation",
        "steady_allocated_bytes_per_operation",
        "expire_delete_allocated_bytes_per_operation",
        "refill_allocated_bytes_per_operation",
        "post_idle_rss_delta_bytes",
        "peak_rss_delta_bytes",
    ] {
        if !metrics.iter().any(|metric| {
            text(metric, "name") == Some(name) && boolean(metric, "passed") == Some(true)
        }) {
            problems.push(format!(
                "local observer product screening omits passing {name} metric"
            ));
        }
    }
    let fill = metrics
        .iter()
        .find(|metric| text(metric, "name") == Some("fill_allocated_bytes_per_operation"));
    if fill.is_none_or(|metric| {
        float(metric, "relative_change").is_none_or(|change| change > -0.15)
            || float(metric, "minimum_relative_improvement") != Some(0.15)
    }) {
        problems.push(
            "local observer product screening does not clear the frozen 15% fill gate".to_owned(),
        );
    }
    let steady = metrics
        .iter()
        .find(|metric| text(metric, "name") == Some("steady_allocated_bytes_per_operation"));
    if steady.is_none_or(|metric| {
        float(metric, "relative_change").is_none_or(|change| change > 0.03)
            || float(metric, "absolute_change").is_none_or(|change| change > 16.0)
    }) {
        problems.push(
            "local observer product screening exceeds the frozen steady-read guard".to_owned(),
        );
    }
    for name in [
        "expire_delete_allocated_bytes_per_operation",
        "refill_allocated_bytes_per_operation",
    ] {
        let metric = metrics
            .iter()
            .find(|metric| text(metric, "name") == Some(name));
        if metric.is_none_or(|metric| {
            let relative = float(metric, "candidate_production_over_off").unwrap_or(f64::INFINITY);
            let absolute = float(metric, "candidate_absolute_over_off").unwrap_or(f64::INFINITY);
            relative > 0.03 && absolute > 16.0
        }) {
            problems.push(format!(
                "local observer product screening exceeds the frozen {name} guard"
            ));
        }
    }
    for name in ["post_idle_rss_delta_bytes", "peak_rss_delta_bytes"] {
        let metric = metrics
            .iter()
            .find(|metric| text(metric, "name") == Some(name));
        if metric.is_none_or(|metric| {
            let relative = float(metric, "candidate_production_over_off").unwrap_or(f64::INFINITY);
            let absolute = float(metric, "candidate_absolute_over_off").unwrap_or(f64::INFINITY);
            relative > 0.05 && absolute > 1_048_576.0
        }) {
            problems.push(format!(
                "local observer product screening exceeds the frozen {name} guard"
            ));
        }
    }
    problems
}

pub fn check_proposal_registry(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "registry_state") != Some("i73_frozen_w1_classification_open")
    {
        problems.push("0.73 proposal registry identity mismatch".to_owned());
    }
    if boolean(value, "candidate_measurements_allowed") != Some(false)
        || boolean(value, "local_candidate_screening_allowed") != Some(true)
        || boolean(value, "local_candidate_screening_completed") != Some(true)
        || boolean(value, "product_mutations_allowed") != Some(true)
    {
        problems.push(
            "integrated registry must require a new D2 proposal while W1 classification is open"
                .to_owned(),
        );
    }
    let proposals = value
        .get("proposals")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    let Some(proposal) = proposals.iter().find(|proposal| {
        text(proposal, "proposal_id") == Some("P73-INSTRUMENTATION-NONBLOCKING-REMOVAL")
    }) else {
        problems.push("proposal registry omits instrumentation redesign".to_owned());
        return problems;
    };
    if text(proposal, "state") != Some("absorbed-into-frozen-i73")
        || boolean(proposal, "d2_authorized") != Some(true)
        || boolean(proposal, "product_mutation_allowed") != Some(true)
        || boolean(proposal, "candidate_measurements_allowed") != Some(false)
        || boolean(proposal, "d3_measurement_required") != Some(false)
        || boolean(proposal, "local_candidate_screening_allowed") != Some(true)
        || boolean(proposal, "local_candidate_screening_completed") != Some(true)
        || text(proposal, "practical_minimum_effect")
            != Some("fill allocations improve by at least 15%")
        || text(proposal, "threshold_status")
            != Some(
                "frozen by single-maintainer review at commit 1e9fd748f1a234967e9303c7071dd0816fb7ac28",
            )
        || text(proposal, "review_status") != Some("d2-authorized-pinned-fork")
    {
        problems.push(
            "instrumentation redesign must remain bound inside frozen I73 rather than become a candidate"
                .to_owned(),
        );
    }
    for field in [
        "owner",
        "hypothesis",
        "primary_metric",
        "threshold_status",
        "compatibility_outcome",
        "rollback_class",
        "dependency_delta",
        "implementation_commit",
        "implementation_receipt",
        "local_screening_evidence",
        "baseline_freeze_evidence",
        "next_evidence",
    ] {
        if text(proposal, field).is_none_or(str::is_empty) {
            problems.push(format!("instrumentation proposal requires {field}"));
        }
    }
    if text(proposal, "d2_review_candidate") != Some(NOTIFICATION_OBSERVER_D2_REVIEW) {
        problems.push("instrumentation proposal must reference its D2 review candidate".to_owned());
    }
    if text(proposal, "dependency_decision") != Some(MOKA_FORK_DECISION) {
        problems
            .push("instrumentation proposal must reference the pinned fork decision".to_owned());
    }
    if text(proposal, "implementation_receipt") != Some(NOTIFICATION_OBSERVER_PRODUCT)
        || text(proposal, "implementation_commit")
            != Some("73fc38a131d26e78b246fe93d5edd71d33796bbf")
    {
        problems.push(
            "instrumentation proposal must bind the admitted product implementation".to_owned(),
        );
    }
    if text(proposal, "local_screening_evidence") != Some(LOCAL_OBSERVER_PRODUCT_SCREENING) {
        problems.push("instrumentation proposal must bind the local product screen".to_owned());
    }
    if text(proposal, "baseline_freeze_evidence") != Some(BASELINE_PILOT_V2_FREEZE_EVIDENCE) {
        problems.push("instrumentation proposal must bind the I73 freeze evidence".to_owned());
    }
    for (field, minimum) in [
        ("baseline_evidence", 3),
        ("source_findings", 4),
        ("rejected_approaches", 5),
        ("candidate_options_requiring_d2", 2),
        ("required_correctness_tests", 6),
        ("required_regression_guards", 4),
    ] {
        if string_array(proposal.get(field)).len() < minimum {
            problems.push(format!("instrumentation proposal has incomplete {field}"));
        }
    }
    problems
}

pub fn check_statistics(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "contract_id") != Some("performance-statistics-073-v1")
        || text(value, "state") != Some("baseline-only-thresholds-frozen")
    {
        problems.push("0.73 statistics identity/state mismatch".to_owned());
    }
    if boolean(value, "baseline_only_derivation") != Some(true)
        || boolean(value, "candidate_may_amend") != Some(false)
        || boolean(value, "candidate_measurements_allowed") != Some(true)
        || boolean(value, "silent_retry_allowed") != Some(false)
        || text(value, "baseline_source_sha") != Some("e757556d3a31d565f52a9561d6d4e555bb1cc373")
        || text(value, "baseline_freeze_evidence") != Some(BASELINE_PILOT_V2_FREEZE_EVIDENCE)
    {
        problems.push(
            "statistics must bind frozen I73 while remaining immutable to candidate data"
                .to_owned(),
        );
    }
    for (field, expected) in [
        ("process_model", "independently-started"),
        ("pairing_method", "counterbalanced-seeded-v1"),
        ("multiple_comparison", "holm-bonferroni"),
        ("paired_estimator", "hodges-lehmann-v1"),
        ("slope_estimator", "theil-sen-v1"),
        ("bootstrap_method", "moving-block-v1"),
        ("allocation_limit_state", "frozen-single-maintainer-review"),
        ("rss_limit_state", "frozen-single-maintainer-review"),
        (
            "threshold_review_candidate",
            NOTIFICATION_OBSERVER_D2_REVIEW,
        ),
    ] {
        if text(value, field) != Some(expected) {
            problems.push(format!("statistics {field} must be {expected}"));
        }
    }
    if integer(value, "minimum_admitted_pairs").is_none_or(|count| count < 5)
        || integer(value, "bootstrap_iterations").is_none_or(|count| count < 1000)
        || integer(value, "bootstrap_block_samples").is_none_or(|count| count < 2)
        || float(value, "confidence_level").is_none_or(|level| !(0.95..1.0).contains(&level))
    {
        problems.push("statistics weakens sample or confidence requirements".to_owned());
    }
    let budgets = value
        .get("regression_budget")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    for (metric, maximum) in [
        ("goodput_operations_per_second", 0.02),
        ("cpu_seconds_per_operation", 0.03),
        ("p99_latency_seconds", 0.03),
    ] {
        let valid = budgets.iter().any(|budget| {
            text(budget, "metric") == Some(metric)
                && float(budget, "maximum_relative_regression") == Some(maximum)
        });
        if !valid {
            problems.push(format!("statistics weakens or omits {metric} budget"));
        }
    }
    let invalidating = value
        .get("invalidating_condition")
        .and_then(TomlValue::as_array)
        .map_or(0, Vec::len);
    if invalidating < 4 {
        problems.push("statistics requires all four invalidating conditions".to_owned());
    }
    problems
}

pub fn check_host_profile(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "profile_id") != Some("performance-reference-073-v1")
        || text(value, "state") != Some("admitted-i73-frozen-candidate-ready")
    {
        problems.push("0.73 host profile identity/state mismatch".to_owned());
    }
    if boolean(value, "eligible") != Some(true) {
        problems.push("admitted host profile eligible must be true".to_owned());
    }
    if text(value, "admission_receipt") != Some(HOST_ADMISSION)
        || text(value, "admitted_source_sha") != Some("3ba09fcc1b48b0b1cc2e1b8e29c941b42993ffa3")
        || text(value, "admitted_host_fingerprint")
            != Some("sha256:702282650a14db1fd52ae9f326cadc8909aff5046a18827ffd82ed0ddbfc465d")
        || integer(value, "admission_workflow_run_id") != Some(36_060_837_195)
    {
        problems.push("admitted host profile does not bind the reviewed admission".to_owned());
    }
    if boolean(value, "candidate_measurements_allowed") != Some(true)
        || text(value, "baseline_freeze_evidence") != Some(BASELINE_PILOT_V2_FREEZE_EVIDENCE)
    {
        problems.push("admitted host profile must bind the I73 freeze".to_owned());
    }
    for field in [
        "identity_reuse_from_071_allowed",
        "local_or_shared_runner_promotable",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!("admitted host profile {field} must be false"));
        }
    }
    for field in [
        "dedicated_bare_metal_required",
        "serialized_lease_required",
        "pre_calibration_required",
        "post_calibration_required",
        "completed_bootstrap_admission_required",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!("host profile {field} must be true"));
        }
    }
    if float(value, "calibration_max_relative_spread")
        .is_none_or(|spread| spread <= 0.0 || spread > 0.10)
    {
        problems.push("host profile calibration spread must be in (0, 0.10]".to_owned());
    }
    for (field, minimum) in [
        ("immutable_probes", 8),
        ("mutable_probes", 8),
        ("required_tools", 6),
        ("companion_platforms", 2),
        ("candidate_constraints", 3),
    ] {
        if string_array(value.get(field)).len() < minimum {
            problems.push(format!("host profile has incomplete {field}"));
        }
    }
    problems
}

pub fn check_host_admission(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_id") != Some("performance-host-admission-3ba09fcc-v1")
        || text(value, "profile_id") != Some("performance-reference-073-v1")
        || text(value, "state") != Some("admitted-host-candidate-measurement-closed")
        || integer(value, "workflow_run_id") != Some(36_060_837_195)
        || text(value, "source_sha") != Some("3ba09fcc1b48b0b1cc2e1b8e29c941b42993ffa3")
        || text(value, "host_fingerprint")
            != Some("sha256:702282650a14db1fd52ae9f326cadc8909aff5046a18827ffd82ed0ddbfc465d")
    {
        problems.push("0.73 host admission identity mismatch".to_owned());
    }
    for (field, expected) in [
        (
            "artifact_sha256",
            "1d64b40c5ddd1f8e6f2ffa6a8a405750369b05d21c78f5a72c70d22fddb38469",
        ),
        (
            "admission_manifest_sha256",
            "dd4613fde6f3e78f432493811d987293bde762d819e83c6f6a4347813d12e98e",
        ),
        (
            "preflight_sha256",
            "8f728fc9e20187c35945abc28094a9ab20022730bdac2ee54c25858af781fe7a",
        ),
        (
            "postflight_sha256",
            "7396792103e636c0274e285f3551794b4a643e8ca41cdde8add5825dda0c8c55",
        ),
    ] {
        if text(value, field) != Some(expected) {
            problems.push(format!(
                "host admission {field} does not bind the retained packet"
            ));
        }
    }
    if float(value, "calibration_limit") != Some(0.05)
        || float(value, "pre_calibration_relative_spread").is_none_or(|spread| spread > 0.05)
        || float(value, "post_calibration_relative_spread").is_none_or(|spread| spread > 0.05)
    {
        problems
            .push("host admission calibration exceeds or changes the frozen 5% limit".to_owned());
    }
    for field in [
        "same_fingerprint_before_after",
        "same_identity_probes_before_after",
        "same_lease_before_after",
        "dedicated_bare_metal",
        "failed_attempts_retained",
        "self_reviewed",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!("host admission {field} must be true"));
        }
    }
    for field in [
        "silent_retry_allowed",
        "identity_reuse_from_071",
        "candidate_measurement_authorized",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!("host admission {field} must remain false"));
        }
    }
    let attempts = value
        .get("attempt")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    let run_ids = attempts
        .iter()
        .filter_map(|attempt| integer(attempt, "run_id"))
        .collect::<Vec<_>>();
    if run_ids != [36_060_427_773, 36_060_688_889, 36_060_837_195]
        || attempts
            .iter()
            .filter(|attempt| text(attempt, "result") != Some("admitted"))
            .count()
            != 2
    {
        problems.push(
            "host admission must retain both failed attempts before the admitted run".to_owned(),
        );
    }
    problems
}

pub fn check_baseline_pilot_contract(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "contract_id") != Some("observer-baseline-pilot-073-v1")
        || text(value, "state") != Some("preregistered-unmeasured")
        || text(value, "evidence_class") != Some("dedicated_host_baseline_only")
        || text(value, "profile_id") != Some("performance-reference-073-v1")
        || text(value, "host_admission") != Some(HOST_ADMISSION)
    {
        problems.push("0.73 baseline pilot contract identity mismatch".to_owned());
    }
    for field in [
        "candidate_data_allowed",
        "promotable",
        "numerical_claim_eligible",
        "thresholds_changed",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!("baseline pilot {field} must be false"));
        }
    }
    for field in [
        "counterbalanced_order_required",
        "independent_processes_required",
        "complete_outcome_accounting_required",
        "exact_reconciliation_required",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!("baseline pilot {field} must be true"));
        }
    }
    if string_array(value.get("instrumentation_modes")) != ["off", "production"]
        || integer_array(value.get("offered_rates_per_second")) != [2_500, 5_000, 10_000, 20_000]
        || integer(value, "repeats_per_rate_and_mode") != Some(3)
        || integer(value, "window_seconds") != Some(5)
        || integer(value, "warmup_operations") != Some(5_000)
        || integer(value, "run_order_seed") != Some(73_073)
        || integer(value, "minimum_stable_rates") != Some(3)
    {
        problems.push("baseline pilot changes the preregistered sample or rate grid".to_owned());
    }
    for (field, expected) in [
        ("minimum_achieved_ratio", 0.98),
        ("maximum_goodput_relative_spread", 0.15),
        ("maximum_goodput_regression", 0.02),
        ("maximum_cpu_per_operation_regression", 0.03),
        ("maximum_p99_regression", 0.03),
    ] {
        if float(value, field) != Some(expected) {
            problems.push(format!("baseline pilot changes frozen {field}"));
        }
    }
    if integer(value, "p99_slo_microseconds") != Some(10_000)
        || text(value, "selection_rule").is_none_or(str::is_empty)
        || text(value, "failure_rule").is_none_or(str::is_empty)
    {
        problems.push("baseline pilot omits its p99 or fail-loud selection rule".to_owned());
    }
    problems
}

pub fn check_baseline_pilot_evidence(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_id") != Some("observer-baseline-pilot-insufficient-4ba93a1a-v1")
        || text(value, "contract_id") != Some("observer-baseline-pilot-073-v1")
        || text(value, "state") != Some("executed-insufficient-baseline")
        || text(value, "source_sha") != Some("4ba93a1a1334799f94974a7dcadcda62dae7b587")
        || integer(value, "workflow_run_id") != Some(36_064_788_229)
        || integer(value, "artifact_id") != Some(10_836_226_123)
    {
        problems.push("0.73 insufficient baseline evidence identity mismatch".to_owned());
    }
    for field in [
        "artifact_sha256",
        "campaign_manifest_sha256",
        "preflight_sha256",
        "postflight_sha256",
        "baseline_pilot_sha256",
        "binary_sha256",
    ] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!(
                "insufficient baseline evidence {field} is not SHA-256"
            ));
        }
    }
    for field in [
        "candidate_data_present",
        "candidate_measurement_authorized",
        "i73_freeze_eligible",
        "thresholds_changed",
        "silent_retry_allowed",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!(
                "insufficient baseline evidence {field} must be false"
            ));
        }
    }
    if integer(value, "attempts") != Some(24)
        || integer(value, "failed_attempts") != Some(0)
        || integer(value, "stable_rates") != Some(0)
        || value
            .get("rate")
            .and_then(TomlValue::as_array)
            .is_none_or(|rates| {
                rates.len() != 4
                    || rates
                        .iter()
                        .any(|rate| boolean(rate, "stable") != Some(false))
            })
    {
        problems.push(
            "insufficient baseline evidence must retain 24 attempts and zero stable rates"
                .to_owned(),
        );
    }
    if text(value, "conclusion").is_none_or(str::is_empty)
        || text(value, "next_evidence").is_none_or(str::is_empty)
    {
        problems.push(
            "insufficient baseline evidence requires conclusion and next evidence".to_owned(),
        );
    }
    problems
}

pub fn check_cpu_attribution_contract(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "contract_id") != Some("observer-cpu-attribution-073-v1")
        || text(value, "state") != Some("preregistered-unmeasured")
        || text(value, "evidence_class") != Some("dedicated_host_diagnostic_only")
        || text(value, "profile_id") != Some("performance-reference-073-v1")
        || text(value, "host_admission") != Some(HOST_ADMISSION)
        || text(value, "triggering_evidence") != Some(BASELINE_PILOT_EVIDENCE)
    {
        problems.push("0.73 CPU attribution contract identity mismatch".to_owned());
    }
    for field in [
        "candidate_data_allowed",
        "candidate_measurement_authorized",
        "promotable",
        "numerical_claim_eligible",
        "thresholds_changed",
        "acceptance_decision_allowed",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!("CPU attribution {field} must be false"));
        }
    }
    for field in [
        "counterbalanced_order_required",
        "independent_processes_required",
        "complete_outcome_accounting_required",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!("CPU attribution {field} must be true"));
        }
    }
    if integer(value, "offered_rate_per_second") != Some(10_000)
        || integer(value, "repeats_per_mode") != Some(5)
        || integer(value, "window_seconds") != Some(5)
        || integer(value, "warmup_operations") != Some(5_000)
        || integer(value, "run_order_seed") != Some(73_110)
        || string_array(value.get("instrumentation_modes"))
            != ["off", "counters-only", "observer-noop", "production"]
    {
        problems.push("CPU attribution changes the preregistered modes or sample grid".to_owned());
    }
    if string_array(value.get("comparison_chain"))
        != [
            "off_to_counters_only",
            "counters_only_to_observer_noop",
            "observer_noop_to_production",
            "off_to_production",
        ]
        || text(value, "interpretation_rule").is_none_or(str::is_empty)
        || text(value, "completion_rule").is_none_or(str::is_empty)
        || text(value, "next_decision").is_none_or(str::is_empty)
    {
        problems.push("CPU attribution omits its diagnostic-only interpretation rules".to_owned());
    }
    problems
}

pub fn check_cpu_attribution_evidence(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_id") != Some("observer-cpu-attribution-ed339846-v1")
        || text(value, "contract_id") != Some("observer-cpu-attribution-073-v1")
        || text(value, "state") != Some("complete-diagnostic")
        || text(value, "evidence_class") != Some("dedicated_host_diagnostic_only")
        || text(value, "source_sha") != Some("ed3398469c6a6eb4920a9d89831ded0dba470115")
        || integer(value, "workflow_run_id") != Some(36_066_691_730)
        || integer(value, "artifact_id") != Some(10_836_294_071)
    {
        problems.push("0.73 CPU attribution evidence identity mismatch".to_owned());
    }
    for field in [
        "artifact_sha256",
        "campaign_manifest_sha256",
        "preflight_sha256",
        "postflight_sha256",
        "cpu_attribution_sha256",
        "binary_sha256",
    ] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!("CPU attribution evidence {field} is not SHA-256"));
        }
    }
    for field in [
        "candidate_data_present",
        "candidate_measurement_authorized",
        "promotable",
        "numerical_claim_eligible",
        "acceptance_decision_allowed",
        "thresholds_changed",
        "silent_retry_allowed",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!("CPU attribution evidence {field} must be false"));
        }
    }
    if boolean(value, "diagnostic_complete") != Some(true)
        || integer(value, "attempts") != Some(20)
        || integer(value, "failed_attempts") != Some(0)
    {
        problems
            .push("CPU attribution evidence must retain one complete 20-attempt block".to_owned());
    }
    let modes = value
        .get("mode")
        .and_then(TomlValue::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| text(item, "name"))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if modes != ["off", "counters-only", "observer-noop", "production"] {
        problems.push("CPU attribution evidence mode ledger changed".to_owned());
    }
    let comparisons = value
        .get("comparison")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    let expected = [
        ("off_to_counters_only", -0.0019503437774635217),
        ("counters_only_to_observer_noop", 0.014187636555413205),
        ("observer_noop_to_production", 0.019927477354084042),
        ("off_to_production", 0.03238040632945276),
    ];
    for (name, relative_delta) in expected {
        if !comparisons.iter().any(|item| {
            text(item, "name") == Some(name)
                && float(item, "cpu_relative_delta") == Some(relative_delta)
        }) {
            problems.push(format!("CPU attribution evidence changed {name}"));
        }
    }
    if text(value, "decision") != Some("optimize-empty-removal-drain-fast-path-locally")
        || text(value, "conclusion").is_none_or(str::is_empty)
        || text(value, "next_evidence").is_none_or(str::is_empty)
    {
        problems.push("CPU attribution evidence omits its bounded local decision".to_owned());
    }
    problems
}

pub fn check_removal_drain_fast_path(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_id") != Some("removal-drain-fast-path-affe4390-v1")
        || text(value, "state") != Some("local-correctness-passed-awaiting-baseline-repeat")
        || text(value, "triggering_evidence") != Some(CPU_ATTRIBUTION_EVIDENCE)
        || text(value, "implementation_commit") != Some("affe4390b6b941f520b9bdb46d65d84d48c0a735")
        || text(value, "implementation_parent") != Some("00fc9c9fdf272d2e092ea74753f6b8cf28cbc896")
        || text(value, "changed_file") != Some("crates/hydracache/src/removal_observer.rs")
    {
        problems.push("removal drain fast-path identity mismatch".to_owned());
    }
    for field in [
        "correctness_surface_changed",
        "public_api_changed",
        "thresholds_changed",
        "promotable",
        "numerical_claim_eligible",
        "candidate_measurement_authorized",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!("removal drain fast path {field} must be false"));
        }
    }
    if boolean(value, "baseline_only_repeat_allowed") != Some(true)
        || text(value, "decision") != Some("repeat-unchanged-baseline-only-contract")
        || text(value, "optimization").is_none_or(str::is_empty)
        || text(value, "race_argument").is_none_or(str::is_empty)
        || text(value, "next_evidence").is_none_or(str::is_empty)
    {
        problems.push("removal drain fast path omits its bounded repeat decision".to_owned());
    }
    if string_array(value.get("validation")).len() < 7 {
        problems.push("removal drain fast path validation matrix is incomplete".to_owned());
    }
    let falsifiers = value
        .get("falsifier")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    for id in [
        "empty-drain-lock-elision",
        "pending-work-not-hidden",
        "duplicate-and-saturation-fail-closed",
        "versioned-cleanup",
        "exact-reconciliation",
    ] {
        if !falsifiers.iter().any(|item| {
            text(item, "id") == Some(id)
                && text(item, "status") == Some("passed")
                && text(item, "evidence").is_some_and(|evidence| !evidence.is_empty())
        }) {
            problems.push(format!(
                "removal drain fast path omits passing {id} falsifier"
            ));
        }
    }
    problems
}

pub fn check_baseline_pilot_fast_path_evidence(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_id") != Some("observer-baseline-pilot-insufficient-9bba762c-v1")
        || text(value, "state") != Some("executed-insufficient-baseline-two-stable-rates")
        || text(value, "source_sha") != Some("9bba762c27e4dd108d08651b6b7443a50957bce9")
        || integer(value, "workflow_run_id") != Some(36_067_885_432)
        || integer(value, "artifact_id") != Some(10_837_029_100)
    {
        problems.push("fast-path baseline evidence identity mismatch".to_owned());
    }
    for field in [
        "artifact_sha256",
        "campaign_manifest_sha256",
        "preflight_sha256",
        "postflight_sha256",
        "baseline_pilot_sha256",
        "binary_sha256",
    ] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!(
                "fast-path baseline evidence {field} is not SHA-256"
            ));
        }
    }
    for field in [
        "candidate_data_present",
        "candidate_measurement_authorized",
        "i73_freeze_eligible",
        "thresholds_changed",
        "silent_retry_allowed",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!("fast-path baseline evidence {field} must be false"));
        }
    }
    if integer(value, "attempts") != Some(24)
        || integer(value, "failed_attempts") != Some(0)
        || integer_array(value.get("stable_rates")) != [10_000, 20_000]
    {
        problems
            .push("fast-path baseline evidence must retain exactly two stable rates".to_owned());
    }
    let rates = value
        .get("rate")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    if rates.len() != 4
        || rates
            .iter()
            .filter(|rate| boolean(rate, "stable") == Some(true))
            .count()
            != 2
    {
        problems.push("fast-path baseline rate ledger changed".to_owned());
    }
    problems
}

pub fn check_removal_queue_contract(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "contract_id") != Some("removal-observer-array-queue-073-v1")
        || text(value, "state") != Some("preregistered-before-implementation")
        || text(value, "triggering_evidence") != Some(BASELINE_PILOT_FAST_PATH_EVIDENCE)
        || text(value, "selected_dependency") != Some("crossbeam-queue 0.3.12")
        || text(value, "queue_type") != Some("crossbeam_queue::ArrayQueue")
        || integer(value, "queue_capacity") != Some(4_096)
    {
        problems.push("removal queue contract identity or bound changed".to_owned());
    }
    for field in [
        "capacity_changed",
        "callback_may_block",
        "callback_may_await",
        "callback_may_allocate_queue_nodes",
        "thresholds_changed",
        "candidate_data_allowed",
        "dedicated_host_run_allowed_before_local_gates",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!("removal queue contract {field} must be false"));
        }
    }
    for field in [
        "dependency_already_present_in_lockfile",
        "duplicate_delivery_must_remain_idempotent",
        "overflow_must_mark_observer_dirty",
        "slot_collision_must_mark_observer_dirty",
        "accepted_acknowledged_barrier_preserved",
        "version_conditional_tag_cleanup_preserved",
        "reconciliation_recovery_preserved",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!("removal queue contract {field} must be true"));
        }
    }
    if string_array(value.get("local_gates")).len() < 6
        || text(value, "rollback").is_none_or(str::is_empty)
        || text(value, "success_rule").is_none_or(str::is_empty)
    {
        problems.push("removal queue contract omits local gates or rollback".to_owned());
    }
    problems
}

pub fn check_removal_queue_product(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_id") != Some("removal-queue-product-daffd71b-v1")
        || text(value, "state") != Some("local-correctness-passed-awaiting-baseline-repeat")
        || text(value, "contract") != Some(REMOVAL_QUEUE_CONTRACT)
        || text(value, "triggering_evidence") != Some(BASELINE_PILOT_FAST_PATH_EVIDENCE)
        || text(value, "implementation_commit") != Some("daffd71bf71e3640f33c9ddfd190e5cbce61a0c8")
        || text(value, "implementation_parent") != Some("3d1d9018cee87c76ffa0fcb7bfcec1ff4cd40e4b")
        || text(value, "selected_dependency") != Some("crossbeam-queue 0.3.12")
        || text(value, "queue_type") != Some("crossbeam_queue::ArrayQueue")
        || integer(value, "queue_capacity") != Some(4_096)
    {
        problems.push("removal queue product identity or bound changed".to_owned());
    }
    for field in [
        "capacity_changed",
        "callback_may_block",
        "callback_may_await",
        "callback_may_allocate_queue_nodes",
        "correctness_surface_changed",
        "public_api_changed",
        "thresholds_changed",
        "promotable",
        "numerical_claim_eligible",
        "candidate_measurement_authorized",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!("removal queue product {field} must be false"));
        }
    }
    for field in [
        "duplicate_delivery_remains_idempotent",
        "overflow_marks_observer_dirty",
        "slot_collision_marks_observer_dirty",
        "accepted_acknowledged_barrier_preserved",
        "version_conditional_tag_cleanup_preserved",
        "reconciliation_recovery_preserved",
        "consumer_claim_cancellation_safe",
        "baseline_only_repeat_allowed",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!("removal queue product {field} must be true"));
        }
    }
    let expected_files: BTreeSet<_> = [
        "Cargo.lock",
        "Cargo.toml",
        "crates/hydracache/Cargo.toml",
        "crates/hydracache/src/removal_observer.rs",
        "tools/performance-observer-073/Cargo.lock",
    ]
    .into_iter()
    .collect();
    let changed_files: BTreeSet<_> = string_array(value.get("changed_files"))
        .into_iter()
        .collect();
    if changed_files != expected_files {
        problems.push("removal queue product changed-file ledger is incomplete".to_owned());
    }
    if string_array(value.get("validation")).len() < 12
        || value
            .get("falsifier")
            .and_then(TomlValue::as_array)
            .is_none_or(|falsifiers| falsifiers.len() < 7)
    {
        problems.push("removal queue product validation or falsifiers are incomplete".to_owned());
    }
    for field in [
        "scope_variance",
        "race_argument",
        "decision",
        "next_evidence",
    ] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!("removal queue product requires {field}"));
        }
    }
    problems
}

pub fn check_baseline_pilot_array_queue_evidence(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_id") != Some("observer-baseline-pilot-insufficient-90a40510-v1")
        || text(value, "contract_id") != Some("observer-baseline-pilot-073-v1")
        || text(value, "state") != Some("executed-insufficient-baseline-two-stable-rates")
        || integer(value, "workflow_run_id") != Some(36_069_607_878)
        || integer(value, "duplicate_dispatch_run_id") != Some(36_069_619_626)
        || text(value, "duplicate_dispatch_state") != Some("cancelled-before-job")
        || text(value, "source_sha") != Some("90a405109e6f0a57474fb2c0ece1f6d6454d44ea")
        || integer(value, "artifact_id") != Some(10_837_672_645)
    {
        problems.push("array-queue baseline evidence identity changed".to_owned());
    }
    for field in [
        "artifact_sha256",
        "campaign_manifest_sha256",
        "preflight_sha256",
        "postflight_sha256",
        "baseline_pilot_sha256",
        "binary_sha256",
    ] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!(
                "array-queue baseline evidence {field} is not SHA-256"
            ));
        }
    }
    for field in [
        "candidate_data_present",
        "candidate_measurement_authorized",
        "i73_freeze_eligible",
        "thresholds_changed",
        "silent_retry_allowed",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!(
                "array-queue baseline evidence {field} must be false"
            ));
        }
    }
    if integer(value, "attempts") != Some(24)
        || integer(value, "failed_attempts") != Some(0)
        || integer_array(value.get("stable_rates")) != [10_000, 20_000]
    {
        problems
            .push("array-queue baseline evidence must retain exactly two stable rates".to_owned());
    }
    let rates = value
        .get("rate")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    if rates.len() != 4
        || rates
            .iter()
            .filter(|rate| boolean(rate, "stable") == Some(true))
            .count()
            != 2
    {
        problems.push("array-queue baseline rate ledger changed".to_owned());
    }
    problems
}

pub fn check_removal_sequence_contract(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "contract_id") != Some("removal-observer-sequence-fetch-add-073-v1")
        || text(value, "state") != Some("preregistered-before-implementation")
        || text(value, "triggering_evidence") != Some(BASELINE_PILOT_ARRAY_QUEUE_EVIDENCE)
        || integer(value, "queue_capacity") != Some(4_096)
    {
        problems.push("removal sequence contract identity or bound changed".to_owned());
    }
    for field in [
        "capacity_changed",
        "callback_may_block",
        "callback_may_await",
        "overflow_may_be_silently_accepted",
        "thresholds_changed",
        "candidate_data_allowed",
        "dedicated_host_run_allowed_before_local_gates",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!("removal sequence contract {field} must be false"));
        }
    }
    for field in [
        "accepted_increment_before_publication_preserved",
        "acknowledged_increment_after_cleanup_preserved",
        "overflow_must_mark_observer_dirty",
        "accepted_acknowledged_barrier_preserved",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!("removal sequence contract {field} must be true"));
        }
    }
    if string_array(value.get("implementation_scope"))
        != ["crates/hydracache/src/removal_observer.rs"]
        || string_array(value.get("local_gates")).len() < 6
        || text(value, "optimization").is_none_or(str::is_empty)
        || text(value, "rollback").is_none_or(str::is_empty)
        || text(value, "success_rule").is_none_or(str::is_empty)
    {
        problems.push("removal sequence contract scope or local gates are incomplete".to_owned());
    }
    problems
}

pub fn check_removal_sequence_product(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_id") != Some("removal-sequence-product-549fbaeb-v1")
        || text(value, "state") != Some("local-correctness-passed-awaiting-baseline-repeat")
        || text(value, "contract") != Some(REMOVAL_SEQUENCE_CONTRACT)
        || text(value, "triggering_evidence") != Some(BASELINE_PILOT_ARRAY_QUEUE_EVIDENCE)
        || text(value, "implementation_commit") != Some("549fbaeb1d35a9a4ca54aa86344dac275ea0650e")
        || text(value, "implementation_parent") != Some("f8d706a32140cba4c4cb1b5087966a387f3d30d4")
        || text(value, "changed_file") != Some("crates/hydracache/src/removal_observer.rs")
        || integer(value, "queue_capacity") != Some(4_096)
    {
        problems.push("removal sequence product identity or bound changed".to_owned());
    }
    for field in [
        "capacity_changed",
        "callback_may_block",
        "callback_may_await",
        "correctness_surface_changed",
        "public_api_changed",
        "thresholds_changed",
        "promotable",
        "numerical_claim_eligible",
        "candidate_measurement_authorized",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!("removal sequence product {field} must be false"));
        }
    }
    for field in [
        "accepted_increment_before_publication_preserved",
        "acknowledged_increment_after_cleanup_preserved",
        "overflow_marks_observer_dirty",
        "overflow_recovery_requires_reconciliation",
        "accepted_acknowledged_barrier_preserved",
        "baseline_only_repeat_allowed",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!("removal sequence product {field} must be true"));
        }
    }
    if string_array(value.get("validation")).len() < 10
        || value
            .get("falsifier")
            .and_then(TomlValue::as_array)
            .is_none_or(|falsifiers| falsifiers.len() < 4)
        || text(value, "optimization").is_none_or(str::is_empty)
        || text(value, "race_argument").is_none_or(str::is_empty)
        || text(value, "decision").is_none_or(str::is_empty)
        || text(value, "next_evidence").is_none_or(str::is_empty)
    {
        problems.push("removal sequence product validation is incomplete".to_owned());
    }
    problems
}

pub fn check_baseline_pilot_sequence_evidence(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_id") != Some("observer-baseline-pilot-insufficient-862c9015-v1")
        || text(value, "contract_id") != Some("observer-baseline-pilot-073-v1")
        || text(value, "state") != Some("executed-insufficient-baseline-one-stable-rate")
        || integer(value, "workflow_run_id") != Some(36_070_743_841)
        || integer(value, "invalid_dispatch_run_id") != Some(36_070_717_512)
        || text(value, "invalid_dispatch_state") != Some("cancelled-before-environment-approval")
        || text(value, "source_sha") != Some("862c9015ebde08025c21458d3421be6277d2ca7c")
        || integer(value, "artifact_id") != Some(10_838_825_744)
    {
        problems.push("sequence baseline evidence identity changed".to_owned());
    }
    for field in [
        "artifact_sha256",
        "campaign_manifest_sha256",
        "preflight_sha256",
        "postflight_sha256",
        "baseline_pilot_sha256",
        "binary_sha256",
    ] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!("sequence baseline evidence {field} is not SHA-256"));
        }
    }
    for field in [
        "candidate_data_present",
        "candidate_measurement_authorized",
        "i73_freeze_eligible",
        "thresholds_changed",
        "silent_retry_allowed",
        "code_effect_claimed",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!("sequence baseline evidence {field} must be false"));
        }
    }
    if integer(value, "attempts") != Some(24)
        || integer(value, "failed_attempts") != Some(0)
        || integer_array(value.get("stable_rates")) != [20_000]
    {
        problems.push("sequence baseline evidence must retain exactly one stable rate".to_owned());
    }
    let rates = value
        .get("rate")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    if rates.len() != 4
        || rates
            .iter()
            .filter(|rate| boolean(rate, "stable") == Some(true))
            .count()
            != 1
    {
        problems.push("sequence baseline rate ledger changed".to_owned());
    }
    problems
}

pub fn check_baseline_pilot_v2_contract(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "contract_id") != Some("observer-baseline-pilot-073-v2")
        || text(value, "state") != Some("preregistered-unmeasured")
        || text(value, "supersedes") != Some(BASELINE_PILOT_CONTRACT)
        || text(value, "triggering_evidence") != Some(BASELINE_PILOT_SEQUENCE_EVIDENCE)
        || text(value, "paired_estimator") != Some("hodges-lehmann-v1")
        || text(value, "workflow_trigger") != Some("manual-only-protected-environment")
    {
        problems.push("baseline pilot v2 identity or estimator changed".to_owned());
    }
    for field in [
        "candidate_data_allowed",
        "promotable",
        "numerical_claim_eligible",
        "thresholds_changed",
        "workload_changed",
        "offered_rates_changed",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!("baseline pilot v2 {field} must be false"));
        }
    }
    for field in [
        "counterbalanced_order_required",
        "independent_processes_required",
        "complete_outcome_accounting_required",
        "exact_reconciliation_required",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!("baseline pilot v2 {field} must be true"));
        }
    }
    if integer_array(value.get("offered_rates_per_second")) != [2_500, 5_000, 10_000, 20_000]
        || integer(value, "repeats_per_rate_and_mode") != Some(5)
        || integer(value, "window_seconds") != Some(10)
        || integer(value, "minimum_stable_rates") != Some(3)
        || float(value, "maximum_goodput_regression") != Some(0.02)
        || float(value, "maximum_cpu_per_operation_regression") != Some(0.03)
        || float(value, "maximum_p99_regression") != Some(0.03)
    {
        problems.push("baseline pilot v2 volume, rates, or unchanged ceilings changed".to_owned());
    }
    if text(value, "selection_rule").is_none_or(str::is_empty)
        || text(value, "failure_rule").is_none_or(str::is_empty)
    {
        problems.push("baseline pilot v2 decision rules are missing".to_owned());
    }
    problems
}

pub fn check_baseline_pilot_v2_product(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_id") != Some("baseline-pilot-v2-product-4979e2e1-v1")
        || text(value, "state") != Some("local-tooling-passed-awaiting-first-run")
        || text(value, "contract") != Some(BASELINE_PILOT_V2_CONTRACT)
        || text(value, "implementation_commit") != Some("4979e2e1ec2616ef8f6eddd53045500385600f61")
        || text(value, "implementation_parent") != Some("b2f133ca982564f8db2e9c46791c4e4fc39fb9d9")
        || text(value, "profile_id") != Some("observer-baseline-pilot-073-v2")
        || text(value, "paired_estimator") != Some("hodges-lehmann-v1")
    {
        problems.push("baseline pilot v2 product identity changed".to_owned());
    }
    for field in [
        "thresholds_changed",
        "workload_changed",
        "offered_rates_changed",
        "candidate_data_allowed",
        "promotable",
        "numerical_claim_eligible",
        "push_trigger_present",
        "independent_mode_medians_used_for_decision",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!("baseline pilot v2 product {field} must be false"));
        }
    }
    for field in [
        "manual_dispatch_only",
        "within_pair_differences_used_for_decision",
        "failed_attempts_retained",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!("baseline pilot v2 product {field} must be true"));
        }
    }
    if integer(value, "repeats_per_rate_and_mode") != Some(5)
        || integer(value, "window_seconds") != Some(10)
        || integer_array(value.get("offered_rates_per_second")) != [2_500, 5_000, 10_000, 20_000]
        || string_array(value.get("changed_files")).len() != 4
        || string_array(value.get("validation")).len() < 7
        || value
            .get("falsifier")
            .and_then(TomlValue::as_array)
            .is_none_or(|falsifiers| falsifiers.len() < 4)
    {
        problems.push("baseline pilot v2 product volume or validation is incomplete".to_owned());
    }
    problems
}

pub fn check_baseline_pilot_v2_evidence(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_id") != Some("observer-baseline-pilot-v2-insufficient-72d491ac-v1")
        || text(value, "contract_id") != Some("observer-baseline-pilot-073-v2")
        || text(value, "state") != Some("executed-insufficient-baseline-two-stable-rates")
        || integer(value, "workflow_run_id") != Some(36_072_021_641)
        || integer(value, "cancelled_auto_admission_run_id") != Some(36_072_022_062)
        || text(value, "cancelled_auto_admission_state")
            != Some("cancelled-before-environment-approval")
        || text(value, "source_sha") != Some("72d491acefff4233b80b208833056b3bfee4779d")
        || integer(value, "artifact_id") != Some(10_838_049_240)
        || text(value, "paired_estimator") != Some("hodges-lehmann-v1")
    {
        problems.push("baseline pilot v2 evidence identity changed".to_owned());
    }
    for field in [
        "artifact_sha256",
        "campaign_manifest_sha256",
        "preflight_sha256",
        "postflight_sha256",
        "baseline_pilot_sha256",
        "binary_sha256",
    ] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!("baseline pilot v2 evidence {field} is not SHA-256"));
        }
    }
    for field in [
        "candidate_data_present",
        "candidate_measurement_authorized",
        "i73_freeze_eligible",
        "thresholds_changed",
        "silent_retry_allowed",
        "code_effect_claimed",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!("baseline pilot v2 evidence {field} must be false"));
        }
    }
    if integer(value, "attempts") != Some(40)
        || integer(value, "failed_attempts") != Some(0)
        || integer_array(value.get("stable_rates")) != [5_000, 20_000]
    {
        problems.push("baseline pilot v2 evidence must retain exactly two stable rates".to_owned());
    }
    let rates = value
        .get("rate")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    if rates.len() != 4
        || rates
            .iter()
            .filter(|rate| boolean(rate, "stable") == Some(true))
            .count()
            != 2
    {
        problems.push("baseline pilot v2 rate ledger changed".to_owned());
    }
    problems
}

pub fn check_memory_counter_atomic_contract(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "contract_id") != Some("memory-counter-single-atomic-073-v1")
        || text(value, "state") != Some("preregistered-before-implementation")
        || text(value, "triggering_evidence") != Some(BASELINE_PILOT_V2_EVIDENCE)
    {
        problems.push("memory counter atomic contract identity changed".to_owned());
    }
    for field in [
        "active_mutation_algorithm_changed",
        "version_algorithm_changed",
        "epoch_algorithm_changed",
        "counter_set_changed",
        "public_estimator_changed",
        "callback_may_block",
        "callback_may_await",
        "thresholds_changed",
        "candidate_data_allowed",
        "dedicated_host_run_allowed_before_local_gates",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!(
                "memory counter atomic contract {field} must be false"
            ));
        }
    }
    for field in [
        "counter_updates_inside_mutation_guard",
        "fault_recorded_before_guard_release",
        "overflow_must_fault",
        "underflow_must_fault",
        "exact_snapshot_must_fail_after_fault",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!(
                "memory counter atomic contract {field} must be true"
            ));
        }
    }
    if string_array(value.get("implementation_scope"))
        != ["crates/hydracache/src/memory_footprint.rs"]
        || string_array(value.get("local_gates")).len() < 6
        || text(value, "optimization").is_none_or(str::is_empty)
        || text(value, "rollback").is_none_or(str::is_empty)
        || text(value, "success_rule").is_none_or(str::is_empty)
    {
        problems
            .push("memory counter atomic contract scope or local gates are incomplete".to_owned());
    }
    problems
}

pub fn check_memory_counter_atomic_product(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_id") != Some("memory-counter-atomic-product-a205ce0a-v1")
        || text(value, "state") != Some("local-correctness-passed-awaiting-baseline-v2-repeat")
        || text(value, "contract") != Some(MEMORY_COUNTER_ATOMIC_CONTRACT)
        || text(value, "triggering_evidence") != Some(BASELINE_PILOT_V2_EVIDENCE)
        || text(value, "implementation_commit") != Some("a205ce0ab8e3b76bcb9341483694587fb5e85838")
        || text(value, "implementation_parent") != Some("fbb9fec1df92a88c32411e47cf7623abaa298193")
        || text(value, "changed_file") != Some("crates/hydracache/src/memory_footprint.rs")
    {
        problems.push("memory counter atomic product identity changed".to_owned());
    }
    for field in [
        "active_mutation_algorithm_changed",
        "version_algorithm_changed",
        "epoch_algorithm_changed",
        "counter_set_changed",
        "public_estimator_changed",
        "callback_may_block",
        "callback_may_await",
        "thresholds_changed",
        "promotable",
        "numerical_claim_eligible",
        "candidate_measurement_authorized",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!(
                "memory counter atomic product {field} must be false"
            ));
        }
    }
    for field in [
        "counter_updates_inside_mutation_guard",
        "fault_recorded_before_guard_release",
        "overflow_faults_exact_capture",
        "underflow_faults_exact_capture",
        "fault_is_permanent",
        "baseline_v2_repeat_allowed",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!(
                "memory counter atomic product {field} must be true"
            ));
        }
    }
    if string_array(value.get("validation")).len() < 10
        || value
            .get("falsifier")
            .and_then(TomlValue::as_array)
            .is_none_or(|falsifiers| falsifiers.len() < 4)
        || text(value, "safety_argument").is_none_or(str::is_empty)
        || text(value, "scope_argument").is_none_or(str::is_empty)
        || text(value, "decision") != Some("run-one-unchanged-manual-baseline-v2-repeat")
        || text(value, "next_evidence").is_none_or(str::is_empty)
    {
        problems.push("memory counter atomic product evidence is incomplete".to_owned());
    }
    problems
}

pub fn check_baseline_pilot_v2_counter_evidence(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_id") != Some("observer-baseline-pilot-v2-insufficient-2daccb47-v1")
        || text(value, "contract_id") != Some("observer-baseline-pilot-073-v2")
        || text(value, "state") != Some("executed-insufficient-baseline-one-stable-rate")
        || integer(value, "workflow_run_id") != Some(36_073_786_075)
        || text(value, "source_sha") != Some("2daccb47a854dee245d84c9fac238130ff7298b4")
        || text(value, "implementation_commit") != Some("a205ce0ab8e3b76bcb9341483694587fb5e85838")
        || integer(value, "artifact_id") != Some(10_839_591_990)
        || text(value, "paired_estimator") != Some("hodges-lehmann-v1")
    {
        problems.push("counter fast-path baseline evidence identity changed".to_owned());
    }
    for field in [
        "artifact_sha256",
        "campaign_manifest_sha256",
        "preflight_sha256",
        "postflight_sha256",
        "baseline_pilot_sha256",
        "binary_sha256",
    ] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!(
                "counter fast-path baseline evidence {field} is not SHA-256"
            ));
        }
    }
    for field in [
        "candidate_data_present",
        "candidate_measurement_authorized",
        "i73_freeze_eligible",
        "derived_d3_rates_admitted",
        "thresholds_changed",
        "silent_retry_allowed",
        "code_effect_claimed",
        "manual_repeat_authorized",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!(
                "counter fast-path baseline evidence {field} must be false"
            ));
        }
    }
    let rates = value
        .get("rate")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    if integer(value, "attempts") != Some(40)
        || integer(value, "failed_attempts") != Some(0)
        || integer(value, "packet_file_count") != Some(164)
        || integer_array(value.get("stable_rates")) != [20_000]
        || rates.len() != 4
        || rates
            .iter()
            .filter(|rate| boolean(rate, "stable") == Some(true))
            .count()
            != 1
        || rates.iter().any(|rate| {
            rate.get("cpu_pair_regressions")
                .and_then(TomlValue::as_array)
                .is_none_or(|pairs| pairs.len() != 5)
        })
    {
        problems.push(
            "counter fast-path baseline must retain 40 attempts and exactly one stable rate"
                .to_owned(),
        );
    }
    if text(value, "decision") != Some("attribute-persistent-allocation-and-observer-cost-locally")
        || text(value, "cross_run_interpretation").is_none_or(str::is_empty)
        || text(value, "next_evidence").is_none_or(str::is_empty)
    {
        problems.push("counter fast-path baseline decision is incomplete".to_owned());
    }
    problems
}

pub fn check_observer_allocation_attribution_contract(
    value: &TomlValue,
    release: &str,
) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "contract_id") != Some("observer-allocation-attribution-073-v1")
        || text(value, "state") != Some("preregistered-before-tooling")
        || text(value, "triggering_evidence") != Some(BASELINE_PILOT_V2_COUNTER_EVIDENCE)
        || text(value, "evidence_class") != Some("local_diagnostic_only")
    {
        problems.push("observer allocation attribution contract identity changed".to_owned());
    }
    for field in [
        "cpu_claims_allowed",
        "rss_claims_allowed",
        "correctness_claims_from_ablation_modes_allowed",
        "candidate_data_allowed",
        "numerical_release_claims_allowed",
        "promotable",
        "dedicated_host_run_allowed",
        "product_source_changes_allowed",
        "thresholds_changed",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!(
                "observer allocation attribution contract {field} must be false"
            ));
        }
    }
    for field in [
        "counterbalanced_order_required",
        "independent_processes_required",
        "prebuilt_binary_required",
        "binary_sha256_required",
        "raw_attempts_retained",
        "failed_attempts_retained",
        "allocation_measurement_only",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!(
                "observer allocation attribution contract {field} must be true"
            ));
        }
    }
    if string_array(value.get("implementation_scope")).len() != 3
        || string_array(value.get("instrumentation_modes"))
            != ["off", "counters-only", "observer-noop", "production"]
        || string_array(value.get("scenarios"))
            != [
                "mixed",
                "get",
                "replace",
                "remove-refill",
                "tag-invalidate-refill",
                "ttl-put",
            ]
        || integer(value, "repeats_per_scenario_and_mode") != Some(5)
        || integer(value, "operations_per_attempt") != Some(8_192)
        || integer(value, "offered_rate_per_second") != Some(20_000)
        || integer(value, "warmup_operations") != Some(4_096)
        || integer(value, "run_order_seed") != Some(73_074)
        || text(value, "question").is_none_or(str::is_empty)
        || text(value, "interpretation_rule").is_none_or(str::is_empty)
        || text(value, "success_rule").is_none_or(str::is_empty)
        || text(value, "next_decision").is_none_or(str::is_empty)
    {
        problems
            .push("observer allocation attribution volume or interpretation changed".to_owned());
    }
    problems
}

pub fn check_observer_allocation_attribution_evidence(
    value: &TomlValue,
    release: &str,
) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_id") != Some("observer-allocation-attribution-2c37d2f1-v1")
        || text(value, "state") != Some("local-diagnostic-complete-owner-identified")
        || text(value, "contract") != Some(OBSERVER_ALLOCATION_ATTRIBUTION_CONTRACT)
        || text(value, "triggering_evidence") != Some(BASELINE_PILOT_V2_COUNTER_EVIDENCE)
        || text(value, "source_sha") != Some("2c37d2f167163a364a71cf24fee21eaaf839bd27")
    {
        problems.push("observer allocation attribution evidence identity changed".to_owned());
    }
    for field in ["binary_sha256", "result_sha256"] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!(
                "observer allocation attribution evidence {field} is not SHA-256"
            ));
        }
    }
    for field in [
        "cpu_claims_allowed",
        "rss_claims_allowed",
        "correctness_claims_from_ablation_modes_allowed",
        "candidate_data_present",
        "numerical_release_claims_allowed",
        "promotable",
        "dedicated_host_repeat_authorized",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!(
                "observer allocation attribution evidence {field} must be false"
            ));
        }
    }
    let scenarios = value
        .get("scenario")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    if boolean(value, "allocation_measurement_only") != Some(true)
        || integer(value, "attempts") != Some(120)
        || integer(value, "failed_attempts") != Some(0)
        || integer(value, "packet_file_count") != Some(361)
        || integer(value, "repeats_per_scenario_and_mode") != Some(5)
        || scenarios.len() != 6
        || scenarios.iter().any(|scenario| {
            scenario
                .get("mode_medians")
                .and_then(TomlValue::as_array)
                .is_none_or(|values| values.len() != 4)
                || scenario
                    .get("adjacent_deltas")
                    .and_then(TomlValue::as_array)
                    .is_none_or(|values| values.len() != 3)
        })
    {
        problems.push("observer allocation attribution evidence volume changed".to_owned());
    }
    if text(value, "owner").is_none_or(str::is_empty)
        || text(value, "source_argument").is_none_or(str::is_empty)
        || text(value, "interpretation").is_none_or(str::is_empty)
        || text(value, "decision") != Some("preregister-shared-immutable-entry-tags")
        || text(value, "next_evidence").is_none_or(str::is_empty)
    {
        problems.push("observer allocation attribution evidence decision is incomplete".to_owned());
    }
    problems
}

pub fn check_shared_entry_tags_contract(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "contract_id") != Some("shared-entry-tags-073-v1")
        || text(value, "state") != Some("preregistered-before-implementation")
        || text(value, "triggering_evidence") != Some(OBSERVER_ALLOCATION_ATTRIBUTION_EVIDENCE)
    {
        problems.push("shared entry tags contract identity changed".to_owned());
    }
    for field in [
        "callback_may_block",
        "callback_may_await",
        "public_api_changed",
        "retained_estimator_schema_changed",
        "retained_estimator_values_changed",
        "tag_index_schema_changed",
        "workload_changed",
        "thresholds_changed",
        "candidate_data_allowed",
        "dedicated_host_run_allowed",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!("shared entry tags contract {field} must be false"));
        }
    }
    for field in [
        "cache_entry_tags_shared_immutable",
        "cleanup_ticket_shares_same_tags",
        "event_payload_remains_owned_strings",
        "versioned_cleanup_preserved",
        "queue_capacity_preserved",
        "overflow_and_dirty_semantics_preserved",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!("shared entry tags contract {field} must be true"));
        }
    }
    if string_array(value.get("implementation_scope"))
        != [
            "crates/hydracache/src/entry.rs",
            "crates/hydracache/src/cache.rs",
            "crates/hydracache/src/removal_observer.rs",
        ]
        || string_array(value.get("local_gates")).len() < 8
        || text(value, "optimization").is_none_or(str::is_empty)
        || text(value, "rollback").is_none_or(str::is_empty)
        || text(value, "success_rule").is_none_or(str::is_empty)
    {
        problems.push("shared entry tags contract scope or gates changed".to_owned());
    }
    problems
}

pub fn check_shared_entry_tags_product(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_id") != Some("shared-entry-tags-product-947e624d-v1")
        || text(value, "state") != Some("local-allocation-passed-awaiting-baseline-v2-repeat")
        || text(value, "contract") != Some(SHARED_ENTRY_TAGS_CONTRACT)
        || text(value, "triggering_evidence") != Some(OBSERVER_ALLOCATION_ATTRIBUTION_EVIDENCE)
        || text(value, "implementation_commit") != Some("947e624ddcf388a86fba61601118e2f078d8bbe1")
        || text(value, "implementation_parent") != Some("609b73e2898740fa28cb86755ba3b3a928138ce3")
    {
        problems.push("shared entry tags product identity changed".to_owned());
    }
    for field in ["binary_sha256", "result_sha256"] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!("shared entry tags product {field} is not SHA-256"));
        }
    }
    for field in [
        "cpu_claims_allowed",
        "rss_claims_allowed",
        "numerical_release_claims_allowed",
        "promotable",
        "candidate_measurement_authorized",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!("shared entry tags product {field} must be false"));
        }
    }
    let scenarios = value
        .get("scenario")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    let scenario_names = scenarios
        .iter()
        .filter_map(|scenario| text(scenario, "name"))
        .collect::<Vec<_>>();
    if boolean(value, "allocation_measurement_only") != Some(true)
        || boolean(value, "baseline_v2_repeat_allowed") != Some(true)
        || integer(value, "attempts") != Some(120)
        || integer(value, "failed_attempts") != Some(0)
        || integer(value, "packet_file_count") != Some(361)
        || integer(value, "repeats_per_scenario_and_mode") != Some(5)
        || integer(value, "operations_per_attempt") != Some(8_192)
        || integer(value, "offered_rate_per_second") != Some(20_000)
        || integer(value, "warmup_operations") != Some(4_096)
        || integer(value, "run_order_seed") != Some(73_074)
        || scenario_names
            != [
                "mixed",
                "get",
                "replace",
                "remove-refill",
                "tag-invalidate-refill",
                "ttl-put",
            ]
        || scenarios.iter().any(|scenario| {
            [
                "before_observer_noop_delta",
                "after_observer_noop_delta",
                "before_off_to_production_delta",
                "after_off_to_production_delta",
            ]
            .iter()
            .any(|field| scenario.get(*field).and_then(TomlValue::as_float).is_none())
        })
    {
        problems.push("shared entry tags product allocation evidence changed".to_owned());
    }
    for field in [
        "optimization",
        "interpretation",
        "remaining_cost",
        "safety_argument",
        "next_evidence",
    ] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!("shared entry tags product {field} is missing"));
        }
    }
    if text(value, "decision") != Some("run-one-unchanged-manual-baseline-v2-repeat")
        || string_array(value.get("validation")).len() < 10
    {
        problems.push("shared entry tags product decision or validation is incomplete".to_owned());
    }
    problems
}

pub fn check_baseline_pilot_v2_freeze_evidence(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_id") != Some("observer-baseline-pilot-v2-passed-e757556d-v1")
        || text(value, "contract_id") != Some("observer-baseline-pilot-073-v2")
        || text(value, "state") != Some("executed-passed-four-stable-rates-i73-frozen")
        || text(value, "evidence_class") != Some("dedicated_host_baseline_only")
        || integer(value, "workflow_run_id") != Some(36_077_381_099)
        || text(value, "source_sha") != Some("e757556d3a31d565f52a9561d6d4e555bb1cc373")
        || text(value, "implementation_commit") != Some("947e624ddcf388a86fba61601118e2f078d8bbe1")
    {
        problems.push("baseline v2 freeze evidence identity changed".to_owned());
    }
    for field in [
        "binary_sha256",
        "artifact_sha256",
        "campaign_manifest_sha256",
        "preflight_sha256",
        "postflight_sha256",
        "baseline_pilot_sha256",
    ] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!(
                "baseline v2 freeze evidence {field} is not SHA-256"
            ));
        }
    }
    if text(value, "host_fingerprint")
        .is_none_or(|digest| !digest.strip_prefix("sha256:").is_some_and(sha256))
    {
        problems.push("baseline v2 freeze host fingerprint is not SHA-256".to_owned());
    }
    for field in [
        "candidate_data_present",
        "thresholds_changed",
        "silent_retry_allowed",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!("baseline v2 freeze evidence {field} must be false"));
        }
    }
    for field in [
        "candidate_measurement_authorized",
        "i73_freeze_eligible",
        "derived_d3_rates_admitted",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!("baseline v2 freeze evidence {field} must be true"));
        }
    }
    let rates = value
        .get("rate")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    if integer(value, "attempts") != Some(40)
        || integer(value, "failed_attempts") != Some(0)
        || integer(value, "packet_file_count") != Some(164)
        || text(value, "paired_estimator") != Some("hodges-lehmann-v1")
        || integer_array(value.get("stable_rates")) != [2_500, 5_000, 10_000, 20_000]
        || integer(value, "selected_knee_rate_per_second") != Some(20_000)
        || integer_array(value.get("frozen_d3_rates_per_second")) != [5_000, 12_000, 17_000]
        || rates.len() != 4
        || rates.iter().any(|rate| {
            boolean(rate, "stable") != Some(true)
                || float(rate, "cpu_per_operation_regression").is_none()
                || float(rate, "allocation_overhead_bytes_per_operation").is_none()
                || float(rate, "p99_relative_regression").is_none()
                || float(rate, "goodput_relative_regression").is_none()
                || float_array(rate.get("cpu_pair_regressions")).len() != 5
        })
    {
        problems.push("baseline v2 freeze evidence volume or rate decision changed".to_owned());
    }
    if text(value, "decision") != Some("freeze-i73-and-open-preregistered-d3-candidate-measurement")
        || text(value, "conclusion").is_none_or(str::is_empty)
        || text(value, "cross_run_interpretation").is_none_or(str::is_empty)
        || text(value, "next_evidence").is_none_or(str::is_empty)
    {
        problems.push("baseline v2 freeze decision is incomplete".to_owned());
    }
    problems
}

pub fn check_w1_owner_classification_contract(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "contract_id") != Some("w1-owner-classification-073-v1")
        || text(value, "state") != Some("preregistered-source-audit-complete-probes-pending")
        || text(value, "baseline_identity") != Some("I73")
        || text(value, "baseline_source_sha") != Some("e757556d3a31d565f52a9561d6d4e555bb1cc373")
        || text(value, "baseline_freeze_evidence") != Some(BASELINE_PILOT_V2_FREEZE_EVIDENCE)
    {
        problems.push("W1 owner classification identity changed".to_owned());
    }
    for field in [
        "candidate_data_allowed",
        "product_mutation_allowed",
        "dedicated_host_run_allowed",
        "local_results_promotable",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!("W1 owner classification {field} must be false"));
        }
    }
    for field in [
        "production_counter_addition_requires_missing_owner",
        "profile_identity_separate",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!("W1 owner classification {field} must be true"));
        }
    }
    if string_array(value.get("required_terminal_dispositions"))
        != [
            "accepted",
            "measured-no-win",
            "not-applicable",
            "rejected",
            "deferred-external-blocker",
        ]
        || string_array(value.get("probe_order")).len() < 5
        || text(value, "success_rule").is_none_or(str::is_empty)
    {
        problems.push("W1 owner classification process or terminal states changed".to_owned());
    }
    let surfaces = value
        .get("surface")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    let expected = [
        ("W2", "shared-store-expiry"),
        ("W3", "tag-index"),
        ("W4", "resp-translation"),
        ("W5", "hc2-connection-state"),
        ("W6", "management-service-overhead"),
        ("W7", "durability-page-cache"),
        ("W8", "allocator"),
        ("W9", "retained-byte-admission"),
    ];
    if surfaces.len() != expected.len() {
        problems.push("W1 owner classification must retain all W2--W9 surfaces".to_owned());
    }
    for (work_item, id) in expected {
        let matches = surfaces
            .iter()
            .filter(|surface| {
                text(surface, "work_item") == Some(work_item) && text(surface, "id") == Some(id)
            })
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            problems.push(format!(
                "W1 owner classification requires exactly one {work_item}/{id}"
            ));
            continue;
        }
        let surface = matches[0];
        if boolean(surface, "candidate_authorized") != Some(false)
            || text(surface, "owner").is_none_or(str::is_empty)
            || text(surface, "current_visibility").is_none_or(str::is_empty)
            || text(surface, "next_probe").is_none_or(str::is_empty)
            || text(surface, "falsifier").is_none_or(str::is_empty)
            || string_array(surface.get("source_files")).is_empty()
            || string_array(surface.get("missing_signals")).is_empty()
        {
            problems.push(format!(
                "W1 owner classification has incomplete {work_item}/{id}"
            ));
        }
    }
    problems
}

pub fn check_w2_expiry_sweep_profile_contract(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "contract_id") != Some("w2-expiry-sweep-profile-073-v1")
        || text(value, "state") != Some("preregistered-awaiting-local-profile")
        || text(value, "parent_contract") != Some(W1_OWNER_CLASSIFICATION_CONTRACT)
        || text(value, "baseline_identity") != Some("I73")
        || text(value, "baseline_source_sha") != Some("e757556d3a31d565f52a9561d6d4e555bb1cc373")
        || text(value, "profile_source_parent") != Some("f471d1964a2f5cd650ada3dc720c077b9168b71b")
        || text(value, "profile_feature") != Some("performance-profile")
        || text(value, "profile_tool") != Some("tools/expiry-sweep-profile-073")
    {
        problems.push("W2 expiry sweep profile identity changed".to_owned());
    }
    for field in [
        "production_feature_enabled",
        "product_mutation_allowed",
        "candidate_data_allowed",
        "dedicated_host_run_allowed",
        "local_results_promotable",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!("W2 expiry sweep profile {field} must be false"));
        }
    }
    if boolean(value, "independent_processes_required") != Some(true)
        || integer(value, "repeats_per_scenario") != Some(5)
        || integer(value, "fixture_entries") != Some(512)
        || integer(value, "scan_limit") != Some(256)
        || integer(value, "identity_bytes_per_key") != Some(96)
        || string_array(value.get("scenarios"))
            != [
                "none-expired",
                "half-expired",
                "all-expired",
                "cursor-wrap-none-expired",
            ]
        || string_array(value.get("primary_metrics"))
            != [
                "allocation_count",
                "gross_allocated_bytes",
                "cloned_identity_bytes",
            ]
    {
        problems.push("W2 expiry sweep profile volume or metrics changed".to_owned());
    }
    for field in [
        "hypothesis",
        "falsifier",
        "interpretation_limit",
        "next_decision",
    ] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!("W2 expiry sweep profile {field} is missing"));
        }
    }
    problems
}

pub fn check_w2_expiry_sweep_profile_evidence(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_id") != Some("w2-expiry-sweep-profile-e4a61d9f-v1")
        || text(value, "state") != Some("local-owner-attributed-candidate-contract-required")
        || text(value, "contract") != Some(W2_EXPIRY_SWEEP_PROFILE_CONTRACT)
        || text(value, "parent_contract") != Some(W1_OWNER_CLASSIFICATION_CONTRACT)
        || text(value, "profile_source_commit") != Some("e4a61d9fbbd3fa3fa5eb3b12e39b2ca5b5872404")
        || text(value, "baseline_identity") != Some("I73")
    {
        problems.push("W2 expiry sweep evidence identity changed".to_owned());
    }
    for field in [
        "binary_sha256",
        "contract_sha256",
        "tool_lock_sha256",
        "tool_source_sha256",
        "raw_result_sha256",
    ] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!("W2 expiry sweep evidence {field} is not SHA-256"));
        }
    }
    for field in [
        "production_feature_enabled",
        "product_mutation_present",
        "candidate_implementation_allowed",
        "dedicated_host_run_allowed",
        "promotable",
        "cpu_claims_allowed",
        "latency_claims_allowed",
        "rss_claims_allowed",
        "numerical_release_claims_allowed",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!("W2 expiry sweep evidence {field} must be false"));
        }
    }
    if integer(value, "attempts") != Some(20)
        || integer(value, "failed_attempts") != Some(0)
        || integer(value, "repeats_per_scenario") != Some(5)
        || integer(value, "fixture_entries") != Some(512)
        || integer(value, "scan_limit") != Some(256)
        || integer(value, "identity_bytes_per_key") != Some(96)
    {
        problems.push("W2 expiry sweep evidence volume changed".to_owned());
    }
    let scenarios = value
        .get("scenario")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    let expected = [
        ("none-expired", 0, 257, 24_672, 772, 43_104),
        ("half-expired", 128, 385, 36_960, 1_162, 73_536),
        ("all-expired", 256, 513, 49_248, 1_547, 104_256),
        ("cursor-wrap-none-expired", 0, 257, 24_672, 772, 43_104),
    ];
    for (name, expired, clones, clone_bytes, allocations, gross_bytes) in expected {
        let matches = scenarios
            .iter()
            .filter(|scenario| text(scenario, "name") == Some(name))
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            problems.push(format!(
                "W2 expiry sweep evidence requires one {name} scenario"
            ));
            continue;
        }
        let scenario = matches[0];
        if integer(scenario, "examined_keys") != Some(256)
            || integer(scenario, "expired_keys") != Some(expired)
            || integer(scenario, "cloned_keys") != Some(clones)
            || integer(scenario, "cloned_identity_bytes") != Some(clone_bytes)
            || integer(scenario, "allocation_count") != Some(allocations)
            || integer(scenario, "gross_allocated_bytes") != Some(gross_bytes)
        {
            problems.push(format!("W2 expiry sweep evidence {name} result changed"));
        }
    }
    if scenarios.len() != expected.len()
        || text(value, "owner").is_none_or(str::is_empty)
        || text(value, "allocation_model").is_none_or(str::is_empty)
        || text(value, "falsifier_result").is_none_or(str::is_empty)
        || text(value, "interpretation").is_none_or(str::is_empty)
        || text(value, "decision") != Some("preregister-d2-borrowed-scan-candidate")
        || text(value, "next_evidence").is_none_or(str::is_empty)
    {
        problems.push("W2 expiry sweep evidence decision is incomplete".to_owned());
    }
    problems
}

pub fn check_w2_borrowed_expiry_scan_contract(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "contract_id") != Some("w2-borrowed-expiry-scan-073-v1")
        || text(value, "state") != Some("preregistered-candidate-implementation-authorized")
        || text(value, "triggering_evidence") != Some(W2_EXPIRY_SWEEP_PROFILE_EVIDENCE)
        || text(value, "baseline_profile_source")
            != Some("e4a61d9fbbd3fa3fa5eb3b12e39b2ca5b5872404")
        || text(value, "candidate_source_parent")
            != Some("b4ffe170bd96383971d40dbd925202343f9f6600")
        || text(value, "baseline_identity") != Some("I73")
        || text(value, "profile_tool") != Some("tools/expiry-sweep-profile-073")
    {
        problems.push("W2 borrowed expiry scan candidate identity changed".to_owned());
    }
    for field in [
        "production_feature_enabled",
        "candidate_data_present",
        "dedicated_host_run_allowed",
        "local_results_promotable",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!("W2 borrowed expiry scan {field} must be false"));
        }
    }
    if boolean(value, "product_mutation_allowed") != Some(true)
        || boolean(value, "independent_processes_required") != Some(true)
        || integer(value, "repeats_per_scenario") != Some(5)
        || text(value, "primary_metric") != Some("gross_allocated_bytes")
        || string_array(value.get("secondary_metrics"))
            != ["allocation_count", "cloned_identity_bytes"]
        || string_array(value.get("forbidden_changes")).len() != 7
        || string_array(value.get("required_correctness")).len() != 5
    {
        problems.push("W2 borrowed expiry scan candidate bounds changed".to_owned());
    }
    for field in [
        "candidate",
        "acceptance",
        "rejection",
        "interpretation_limit",
        "next_decision",
    ] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!("W2 borrowed expiry scan {field} is missing"));
        }
    }
    problems
}

pub fn check_w2_borrowed_expiry_scan_evidence(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_id") != Some("w2-borrowed-expiry-scan-a80839fd-v1")
        || text(value, "state") != Some("local-candidate-accepted-no-dedicated-run")
        || text(value, "contract") != Some(W2_BORROWED_EXPIRY_SCAN_CONTRACT)
        || text(value, "triggering_evidence") != Some(W2_EXPIRY_SWEEP_PROFILE_EVIDENCE)
        || text(value, "implementation_commit") != Some("df88c28cdd4f068a315959b141d42cee4f31d4b0")
        || text(value, "candidate_source_commit")
            != Some("a80839fd67aad086a9881375d19964dcc3449498")
        || text(value, "baseline_profile_source")
            != Some("e4a61d9fbbd3fa3fa5eb3b12e39b2ca5b5872404")
        || text(value, "baseline_identity") != Some("I73")
    {
        problems.push("W2 borrowed expiry scan evidence identity changed".to_owned());
    }
    for field in [
        "binary_sha256",
        "contract_sha256",
        "tool_lock_sha256",
        "tool_source_sha256",
        "raw_result_sha256",
    ] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!(
                "W2 borrowed expiry scan evidence {field} is not SHA-256"
            ));
        }
    }
    for field in [
        "production_feature_enabled",
        "dedicated_host_run_allowed",
        "promotable",
        "cpu_claims_allowed",
        "mutex_wait_claims_allowed",
        "latency_claims_allowed",
        "rss_claims_allowed",
        "numerical_release_claims_allowed",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!(
                "W2 borrowed expiry scan evidence {field} must be false"
            ));
        }
    }
    if boolean(value, "candidate_local_accepted") != Some(true)
        || integer(value, "attempts") != Some(20)
        || integer(value, "failed_attempts") != Some(0)
        || integer(value, "repeats_per_scenario") != Some(5)
        || integer(value, "fixture_entries") != Some(512)
        || integer(value, "scan_limit") != Some(256)
        || integer(value, "identity_bytes_per_key") != Some(96)
        || integer(value, "gross_allocated_bytes_removed_per_bounded_scan") != Some(43_008)
        || integer(value, "allocations_removed_per_bounded_scan") != Some(769)
        || integer(value, "cloned_identity_bytes_removed_per_bounded_scan") != Some(24_576)
        || text(value, "w1_disposition") != Some("accepted")
    {
        problems.push("W2 borrowed expiry scan evidence volume or disposition changed".to_owned());
    }
    let scenarios = value
        .get("scenario")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    let expected = [
        ("none-expired", 43_104, 96, 772, 3, 1, 96),
        ("half-expired", 73_536, 30_528, 1_162, 393, 129, 12_384),
        ("all-expired", 104_256, 61_248, 1_547, 778, 257, 24_672),
        ("cursor-wrap-none-expired", 43_104, 96, 772, 3, 1, 96),
    ];
    for (
        name,
        baseline_bytes,
        candidate_bytes,
        baseline_count,
        candidate_count,
        clones,
        clone_bytes,
    ) in expected
    {
        let matches = scenarios
            .iter()
            .filter(|scenario| text(scenario, "name") == Some(name))
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            problems.push(format!(
                "W2 borrowed expiry scan evidence requires one {name} scenario"
            ));
            continue;
        }
        let scenario = matches[0];
        if integer(scenario, "baseline_gross_allocated_bytes") != Some(baseline_bytes)
            || integer(scenario, "candidate_gross_allocated_bytes") != Some(candidate_bytes)
            || integer(scenario, "baseline_allocation_count") != Some(baseline_count)
            || integer(scenario, "candidate_allocation_count") != Some(candidate_count)
            || integer(scenario, "candidate_cloned_keys") != Some(clones)
            || integer(scenario, "candidate_cloned_identity_bytes") != Some(clone_bytes)
            || float(scenario, "relative_change").is_none()
        {
            problems.push(format!(
                "W2 borrowed expiry scan evidence {name} result changed"
            ));
        }
    }
    if scenarios.len() != expected.len()
        || text(value, "correctness").is_none_or(str::is_empty)
        || text(value, "allocation_model").is_none_or(str::is_empty)
        || text(value, "interpretation").is_none_or(str::is_empty)
        || text(value, "decision") != Some("accept-w2-locally-and-continue-owner-classification")
        || text(value, "next_evidence").is_none_or(str::is_empty)
    {
        problems.push("W2 borrowed expiry scan evidence decision is incomplete".to_owned());
    }
    problems
}

pub fn check_w5_hc2_event_copy_profile_contract(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "contract_id") != Some("w5-hc2-event-copy-profile-073-v1")
        || text(value, "state") != Some("preregistered-awaiting-local-profile")
        || text(value, "parent_contract") != Some(W1_OWNER_CLASSIFICATION_CONTRACT)
        || text(value, "source_parent") != Some("9bcafa654281a4ba56d93972e8d5db16096a99ee")
        || text(value, "source_owner")
            != Some("crates/hydracache-server/src/hc2.rs::emit_matching_events")
        || text(value, "profile_tool") != Some("tools/hc2-event-copy-profile-073")
    {
        problems.push("W5 HC/2 event copy profile identity changed".to_owned());
    }
    for field in [
        "production_counter_addition",
        "product_mutation_allowed",
        "candidate_data_allowed",
        "dedicated_host_run_allowed",
        "local_results_promotable",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!("W5 HC/2 event copy profile {field} must be false"));
        }
    }
    if boolean(value, "independent_processes_required") != Some(true)
        || integer(value, "repeats_per_scenario") != Some(5)
        || integer(value, "key_bytes") != Some(64)
        || integer_array(value.get("fanouts")) != [1, 8, 16]
        || integer_array(value.get("value_bytes")) != [128, 4_096]
        || string_array(value.get("scenarios"))
            != [
                "fanout-1-value-128",
                "fanout-8-value-128",
                "fanout-16-value-128",
                "fanout-16-value-4096",
            ]
    {
        problems.push("W5 HC/2 event copy profile volume changed".to_owned());
    }
    for field in [
        "queue_capacity_owner",
        "copy_owner",
        "hypothesis",
        "falsifier",
        "interpretation_limit",
        "next_decision",
    ] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!("W5 HC/2 event copy profile {field} is missing"));
        }
    }
    problems
}

pub fn check_w5_hc2_event_copy_profile_evidence(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_id") != Some("w5-hc2-event-copy-profile-02777da6-v1")
        || text(value, "state") != Some("local-owner-attributed-candidate-contract-required")
        || text(value, "contract") != Some(W5_HC2_EVENT_COPY_PROFILE_CONTRACT)
        || text(value, "source_commit") != Some("02777da68210d166bf3d072d71bbff6975bd51b4")
        || text(value, "source_owner")
            != Some("crates/hydracache-server/src/hc2.rs::emit_matching_events")
    {
        problems.push("W5 HC/2 event copy evidence identity changed".to_owned());
    }
    for field in [
        "binary_sha256",
        "contract_sha256",
        "tool_source_sha256",
        "tool_lock_sha256",
        "raw_result_sha256",
    ] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!(
                "W5 HC/2 event copy evidence {field} is not SHA-256"
            ));
        }
    }
    for field in [
        "production_counter_addition",
        "product_mutation_present",
        "candidate_implementation_allowed",
        "dedicated_host_run_allowed",
        "promotable",
        "cpu_claims_allowed",
        "latency_claims_allowed",
        "rss_claims_allowed",
        "numerical_release_claims_allowed",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!("W5 HC/2 event copy evidence {field} must be false"));
        }
    }
    if integer(value, "attempts") != Some(20)
        || integer(value, "failed_attempts") != Some(0)
        || integer(value, "repeats_per_scenario") != Some(5)
    {
        problems.push("W5 HC/2 event copy evidence volume changed".to_owned());
    }
    let scenarios = value
        .get("scenario")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    let expected = [
        ("fanout-1-value-128", 1, 128, 2, 192),
        ("fanout-8-value-128", 8, 128, 16, 1_536),
        ("fanout-16-value-128", 16, 128, 32, 3_072),
        ("fanout-16-value-4096", 16, 4_096, 32, 66_560),
    ];
    for (name, fanout, value_bytes, allocations, gross_bytes) in expected {
        let matches = scenarios
            .iter()
            .filter(|scenario| text(scenario, "name") == Some(name))
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            problems.push(format!("W5 HC/2 event copy evidence requires one {name}"));
            continue;
        }
        let scenario = matches[0];
        if integer(scenario, "fanout") != Some(fanout)
            || integer(scenario, "value_bytes") != Some(value_bytes)
            || integer(scenario, "allocation_count") != Some(allocations)
            || integer(scenario, "gross_allocated_bytes") != Some(gross_bytes)
        {
            problems.push(format!("W5 HC/2 event copy evidence {name} changed"));
        }
    }
    if scenarios.len() != expected.len()
        || text(value, "owner").is_none_or(str::is_empty)
        || text(value, "queue_bound").is_none_or(str::is_empty)
        || text(value, "falsifier_result").is_none_or(str::is_empty)
        || text(value, "decision") != Some("preregister-d2-shared-bytes-event-candidate")
        || text(value, "next_evidence").is_none_or(str::is_empty)
    {
        problems.push("W5 HC/2 event copy evidence decision is incomplete".to_owned());
    }
    problems
}

pub fn check_w5_shared_event_bytes_contract(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "contract_id") != Some("w5-shared-event-bytes-073-v1")
        || text(value, "state") != Some("preregistered-candidate-implementation-authorized")
        || text(value, "triggering_evidence") != Some(W5_HC2_EVENT_COPY_PROFILE_EVIDENCE)
        || text(value, "candidate_source_parent") != Some("a792df8e")
    {
        problems.push("W5 shared event bytes contract identity changed".to_owned());
    }
    for field in [
        "candidate_data_present",
        "dedicated_host_run_allowed",
        "local_results_promotable",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!("W5 shared event bytes {field} must be false"));
        }
    }
    if boolean(value, "product_mutation_allowed") != Some(true)
        || string_array(value.get("forbidden_changes")).len() != 7
    {
        problems.push("W5 shared event bytes candidate bounds changed".to_owned());
    }
    for field in [
        "candidate",
        "acceptance",
        "rejection",
        "interpretation_limit",
        "next_decision",
    ] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!("W5 shared event bytes {field} is missing"));
        }
    }
    problems
}

pub fn check_w5_hc2_connection_census_contract(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "contract_id") != Some("w5-hc2-connection-census-073-v1")
        || text(value, "state") != Some("preregistered-awaiting-local-census")
        || text(value, "parent_contract") != Some(W1_OWNER_CLASSIFICATION_CONTRACT)
        || text(value, "source_parent") != Some("bd246207bc1b4f01e0800193abca17958fb96b58")
        || text(value, "environment") != Some("local-real-mtls")
    {
        problems.push("W5 HC/2 connection census identity changed".to_owned());
    }
    for field in [
        "production_counter_addition",
        "product_semantics_change_allowed",
        "dedicated_host_run_allowed",
        "promotable",
        "cpu_claims_allowed",
        "allocation_claims_allowed",
        "rss_claims_allowed",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!("W5 HC/2 connection census {field} must be false"));
        }
    }
    if integer_array(value.get("cardinalities")) != [1, 10, 100, 1_000] {
        problems.push("W5 HC/2 connection census cardinalities changed".to_owned());
    }
    for field in [
        "required_active_invariant",
        "required_close_invariant",
        "required_client_invariant",
        "falsifier",
        "interpretation_limit",
        "next_decision",
    ] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!("W5 HC/2 connection census {field} is missing"));
        }
    }
    problems
}

pub fn check_w5_hc2_connection_profile_contract(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "contract_id") != Some("w5-hc2-connection-profile-073-v1")
        || text(value, "state") != Some("preregistered-awaiting-local-process-matrix")
        || text(value, "triggering_evidence")
            != Some("docs/testing/performance/0.73/w5-hc2-connection-census-e673f9cd.toml")
        || text(value, "source_parent") != Some("b77d061c72a51eaae17e87bc16dbccde4b4e2862")
        || text(value, "profile_tool") != Some("tools/hc2-connection-profile-073")
        || text(value, "ownership_scope") != Some("combined-local-client-server-process")
    {
        problems.push("W5 HC/2 connection profile identity changed".to_owned());
    }
    for field in [
        "production_counter_addition",
        "candidate_data_allowed",
        "dedicated_host_run_allowed",
        "promotable",
        "per_server_connection_claim_allowed",
        "release_numerical_claim_allowed",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!("W5 HC/2 connection profile {field} must be false"));
        }
    }
    if boolean(value, "independent_processes_required") != Some(true)
        || integer(value, "repeats_per_cardinality") != Some(3)
        || integer_array(value.get("cardinalities")) != [1, 10, 100, 1_000]
    {
        problems.push("W5 HC/2 connection profile sampling matrix changed".to_owned());
    }
    if string_array(value.get("metrics"))
        != [
            "gross_allocated_bytes",
            "working_set_bytes",
            "peak_working_set_bytes",
            "pagefile_bytes",
            "peak_pagefile_bytes",
        ]
    {
        problems.push("W5 HC/2 connection profile metrics changed".to_owned());
    }
    for field in ["falsifier", "interpretation_limit", "next_decision"] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!("W5 HC/2 connection profile {field} is missing"));
        }
    }
    problems
}

pub fn check_w5_hc2_connection_profile_evidence(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_id") != Some("w5-hc2-connection-profile-ff657fc5-v1")
        || text(value, "state") != Some("local-combined-process-profile-passed-split-required")
        || text(value, "contract") != Some(W5_HC2_CONNECTION_PROFILE_CONTRACT)
        || text(value, "source_commit") != Some("ff657fc5c462ef67d1fe28c3e0bdd6d61f3cf9f4")
        || text(value, "profile_tool") != Some("tools/hc2-connection-profile-073")
        || text(value, "ownership_scope") != Some("combined-local-client-server-process")
    {
        problems.push("W5 HC/2 connection profile evidence identity changed".to_owned());
    }
    for field in [
        "binary_sha256",
        "contract_sha256",
        "tool_source_sha256",
        "tool_lock_sha256",
    ] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!(
                "W5 HC/2 connection profile evidence {field} is not SHA-256"
            ));
        }
    }
    let raw_results = string_array(value.get("raw_results"));
    if raw_results.len() != 12
        || raw_results.iter().any(|entry| {
            entry
                .rsplit_once(':')
                .is_none_or(|(_, digest)| !sha256(digest))
        })
    {
        problems.push("W5 HC/2 connection profile raw result manifest changed".to_owned());
    }
    for field in [
        "production_counter_addition",
        "product_mutation_present",
        "candidate_data_present",
        "dedicated_host_run_allowed",
        "promotable",
        "per_server_connection_claim_allowed",
        "release_numerical_claim_allowed",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!(
                "W5 HC/2 connection profile evidence {field} must be false"
            ));
        }
    }
    for field in [
        "independent_processes",
        "logical_accounting_passed",
        "client_resource_reconciliation_passed",
        "close_reconciliation_passed",
        "stderr_empty",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!(
                "W5 HC/2 connection profile evidence {field} must be true"
            ));
        }
    }
    if integer(value, "attempts") != Some(12)
        || integer(value, "failed_attempts") != Some(0)
        || integer(value, "repeats_per_cardinality") != Some(3)
    {
        problems.push("W5 HC/2 connection profile evidence volume changed".to_owned());
    }
    let cardinalities = value
        .get("cardinality")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    let expected = [
        (1, 278_316, 2_699_264, 2_748_416, 319_488, 212_992),
        (10, 2_782_807, 4_198_400, 3_489_792, 2_478_080, 1_441_792),
        (
            100, 27_828_312, 15_675_392, 4_358_144, 19_435_520, 2_482_176,
        ),
        (
            1_000,
            278_280_142,
            122_925_056,
            6_656_000,
            181_301_248,
            4_939_776,
        ),
    ];
    for (connections, allocation, working_set, post_working_set, pagefile, post_pagefile) in
        expected
    {
        let matches = cardinalities
            .iter()
            .filter(|row| integer(row, "connections") == Some(connections))
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            problems.push(format!(
                "W5 HC/2 connection profile evidence requires one {connections}-connection row"
            ));
            continue;
        }
        let row = matches[0];
        if integer(row, "median_gross_allocated_bytes") != Some(allocation)
            || float(row, "median_gross_allocated_bytes_per_connection").is_none()
            || integer(row, "median_working_set_delta_bytes") != Some(working_set)
            || integer(row, "median_peak_working_set_delta_bytes") != Some(working_set)
            || integer(row, "median_post_close_working_set_delta_bytes") != Some(post_working_set)
            || integer(row, "median_pagefile_delta_bytes") != Some(pagefile)
            || integer(row, "median_peak_pagefile_delta_bytes") != Some(pagefile)
            || integer(row, "median_post_close_pagefile_delta_bytes") != Some(post_pagefile)
        {
            problems.push(format!(
                "W5 HC/2 connection profile evidence {connections}-connection result changed"
            ));
        }
    }
    if cardinalities.len() != expected.len()
        || text(value, "allocation_interpretation").is_none_or(str::is_empty)
        || text(value, "rss_interpretation").is_none_or(str::is_empty)
        || text(value, "decision") != Some("preregister-split-client-server-process-profile")
        || text(value, "next_evidence").is_none_or(str::is_empty)
    {
        problems.push("W5 HC/2 connection profile evidence decision is incomplete".to_owned());
    }
    problems
}

pub fn check_w5_hc2_split_connection_profile_contract(
    value: &TomlValue,
    release: &str,
) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "contract_id") != Some("w5-hc2-split-connection-profile-073-v1")
        || text(value, "state") != Some("preregistered-awaiting-local-split-process-matrix")
        || text(value, "triggering_evidence") != Some(W5_HC2_CONNECTION_PROFILE_EVIDENCE)
        || text(value, "source_parent") != Some("241e1c8a57218614490515089239f2d5659ea5e1")
        || text(value, "profile_tool") != Some("tools/hc2-connection-profile-073")
        || text(value, "profile_mode") != Some("split")
        || text(value, "server_scope") != Some("dedicated-local-server-process")
        || text(value, "client_scope") != Some("dedicated-local-client-controller-process")
    {
        problems.push("W5 HC/2 split connection profile identity changed".to_owned());
    }
    for field in [
        "production_counter_addition",
        "product_mutation_allowed",
        "candidate_data_allowed",
        "dedicated_host_run_allowed",
        "promotable",
        "universal_per_connection_claim_allowed",
        "release_numerical_claim_allowed",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!(
                "W5 HC/2 split connection profile {field} must be false"
            ));
        }
    }
    if boolean(value, "independent_process_pairs_required") != Some(true)
        || integer(value, "repeats_per_cardinality") != Some(3)
        || integer_array(value.get("cardinalities")) != [1, 10, 100, 1_000]
    {
        problems.push("W5 HC/2 split connection profile sampling matrix changed".to_owned());
    }
    if string_array(value.get("metrics"))
        != [
            "client_gross_allocated_bytes",
            "server_gross_allocated_bytes",
            "client_working_set_bytes",
            "server_working_set_bytes",
            "client_pagefile_bytes",
            "server_pagefile_bytes",
        ]
    {
        problems.push("W5 HC/2 split connection profile metrics changed".to_owned());
    }
    for field in [
        "server_allocation_window",
        "client_allocation_window",
        "falsifier",
        "interpretation_limit",
        "next_decision",
    ] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!(
                "W5 HC/2 split connection profile {field} is missing"
            ));
        }
    }
    problems
}

pub fn check_w5_hc2_split_connection_profile_evidence(
    value: &TomlValue,
    release: &str,
) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_id") != Some("w5-hc2-split-connection-profile-19a440d4-v1")
        || text(value, "state")
            != Some("local-endpoints-separated-idle-server-owner-still-composite")
        || text(value, "contract") != Some(W5_HC2_SPLIT_CONNECTION_PROFILE_CONTRACT)
        || text(value, "source_commit") != Some("19a440d4224bad928230d1bb7cf2923457381fe6")
        || text(value, "profile_tool") != Some("tools/hc2-connection-profile-073")
        || text(value, "profile_mode") != Some("split")
    {
        problems.push("W5 HC/2 split connection evidence identity changed".to_owned());
    }
    for field in [
        "binary_sha256",
        "contract_sha256",
        "tool_source_sha256",
        "tool_lock_sha256",
    ] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!(
                "W5 HC/2 split connection evidence {field} is not SHA-256"
            ));
        }
    }
    let raw_results = string_array(value.get("raw_results"));
    if raw_results.len() != 12
        || raw_results.iter().any(|entry| {
            entry
                .rsplit_once(':')
                .is_none_or(|(_, digest)| !sha256(digest))
        })
    {
        problems.push("W5 HC/2 split connection raw result manifest changed".to_owned());
    }
    for field in [
        "production_counter_addition",
        "product_mutation_present",
        "candidate_data_present",
        "dedicated_host_run_allowed",
        "promotable",
        "universal_per_connection_claim_allowed",
        "release_numerical_claim_allowed",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!(
                "W5 HC/2 split connection evidence {field} must be false"
            ));
        }
    }
    for field in [
        "independent_process_pairs",
        "logical_accounting_passed",
        "client_resource_reconciliation_passed",
        "close_reconciliation_passed",
        "stderr_empty",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!(
                "W5 HC/2 split connection evidence {field} must be true"
            ));
        }
    }
    if integer(value, "attempts") != Some(12)
        || integer(value, "failed_attempts") != Some(0)
        || integer(value, "repeats_per_cardinality") != Some(3)
    {
        problems.push("W5 HC/2 split connection evidence volume changed".to_owned());
    }
    let cardinalities = value
        .get("cardinality")
        .and_then(TomlValue::as_array)
        .cloned()
        .unwrap_or_default();
    let expected = [
        (1, 181_421, 97_336, 2_756_608, 1_667_072),
        (10, 1_813_232, 970_503, 3_624_960, 2_322_432),
        (100, 18_132_352, 9_703_054, 10_383_360, 7_176_192),
        (1_000, 181_324_189, 97_024_116, 73_859_072, 51_838_976),
    ];
    for (connections, client_alloc, server_alloc, client_ws, server_ws) in expected {
        let matches = cardinalities
            .iter()
            .filter(|row| integer(row, "connections") == Some(connections))
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            problems.push(format!(
                "W5 HC/2 split connection evidence requires one {connections}-connection row"
            ));
            continue;
        }
        let row = matches[0];
        if integer(row, "median_client_gross_allocated_bytes") != Some(client_alloc)
            || integer(row, "median_server_gross_allocated_bytes") != Some(server_alloc)
            || integer(row, "median_client_working_set_delta_bytes") != Some(client_ws)
            || integer(row, "median_server_working_set_delta_bytes") != Some(server_ws)
        {
            problems.push(format!(
                "W5 HC/2 split connection evidence {connections}-connection result changed"
            ));
        }
    }
    if cardinalities.len() != expected.len()
        || float(value, "client_allocation_marginal_100_to_1000_bytes").is_none()
        || float(value, "server_allocation_marginal_100_to_1000_bytes").is_none()
        || float(value, "client_working_set_marginal_100_to_1000_bytes").is_none()
        || float(value, "server_working_set_marginal_100_to_1000_bytes").is_none()
        || text(value, "comparison").is_none_or(str::is_empty)
        || text(value, "interpretation").is_none_or(str::is_empty)
        || text(value, "decision")
            != Some("accept-idle-endpoint-attribution-and-preregister-slow-consumer-bound")
        || text(value, "next_evidence").is_none_or(str::is_empty)
    {
        problems.push("W5 HC/2 split connection evidence decision is incomplete".to_owned());
    }
    problems
}

pub fn check_w5_hc2_slow_consumer_profile_contract(
    value: &TomlValue,
    release: &str,
) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "contract_id") != Some("w5-hc2-slow-consumer-profile-073-v1")
        || text(value, "state") != Some("preregistered-awaiting-local-drained-vs-unread-pairs")
        || text(value, "triggering_evidence") != Some(W5_HC2_SPLIT_CONNECTION_PROFILE_EVIDENCE)
        || text(value, "source_parent") != Some("a653dddf4d694e15917862763ff49417b1ad44fc")
        || text(value, "profile_tool") != Some("tools/hc2-connection-profile-073")
    {
        problems.push("W5 HC/2 slow-consumer profile identity changed".to_owned());
    }
    for field in [
        "production_counter_addition",
        "product_mutation_allowed",
        "candidate_data_allowed",
        "dedicated_host_run_allowed",
        "promotable",
        "server_queued_byte_claim_allowed",
        "release_numerical_claim_allowed",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!(
                "W5 HC/2 slow-consumer profile {field} must be false"
            ));
        }
    }
    for field in [
        "independent_process_pairs_required",
        "unique_prefix_per_connection_required",
        "same_mutation_volume_required",
        "transport_reader_remains_active",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!(
                "W5 HC/2 slow-consumer profile {field} must be true"
            ));
        }
    }
    if integer(value, "connections") != Some(100)
        || integer(value, "subscriptions_per_connection") != Some(1)
        || integer(value, "mutations_per_connection") != Some(1_100)
        || integer(value, "total_mutations") != Some(110_000)
        || integer(value, "value_bytes") != Some(128)
        || integer(value, "client_subscription_item_capacity") != Some(1_024)
        || integer(value, "server_outbound_item_capacity") != Some(16)
        || integer(value, "pairs") != Some(5)
        || string_array(value.get("scenarios"))
            != [
                "drained-application-consumers",
                "unread-application-consumers",
            ]
        || string_array(value.get("counterbalanced_order")).len() != 5
    {
        problems.push("W5 HC/2 slow-consumer profile workload changed".to_owned());
    }
    for field in [
        "control_invariant",
        "treatment_invariant",
        "close_invariant",
        "falsifier",
        "interpretation_limit",
        "next_decision",
    ] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!("W5 HC/2 slow-consumer profile {field} is missing"));
        }
    }
    problems
}

pub fn check_w5_hc2_slow_consumer_profile_evidence(
    value: &TomlValue,
    release: &str,
) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_id") != Some("w5-hc2-slow-consumer-profile-ff432801-v1")
        || text(value, "state") != Some("local-application-slow-consumer-client-owned-and-bounded")
        || text(value, "contract") != Some(W5_HC2_SLOW_CONSUMER_PROFILE_CONTRACT)
        || text(value, "source_commit") != Some("ff43280136644d887053b8f3a7642d54329e0756")
        || text(value, "profile_tool") != Some("tools/hc2-connection-profile-073")
    {
        problems.push("W5 HC/2 slow-consumer evidence identity changed".to_owned());
    }
    for field in [
        "binary_sha256",
        "contract_sha256",
        "tool_source_sha256",
        "tool_lock_sha256",
    ] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!(
                "W5 HC/2 slow-consumer evidence {field} is not SHA-256"
            ));
        }
    }
    let raw_results = string_array(value.get("raw_results"));
    if raw_results.len() != 10
        || raw_results.iter().any(|entry| {
            entry
                .rsplit_once(':')
                .is_none_or(|(_, digest)| !sha256(digest))
        })
    {
        problems.push("W5 HC/2 slow-consumer raw result manifest changed".to_owned());
    }
    for field in [
        "production_counter_addition",
        "product_mutation_present",
        "candidate_data_present",
        "dedicated_host_run_allowed",
        "promotable",
        "server_queued_byte_claim_allowed",
        "release_numerical_claim_allowed",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!(
                "W5 HC/2 slow-consumer evidence {field} must be false"
            ));
        }
    }
    for field in [
        "control_invariant_passed",
        "treatment_invariant_passed",
        "close_invariant_passed",
        "stderr_empty",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!(
                "W5 HC/2 slow-consumer evidence {field} must be true"
            ));
        }
    }
    if integer(value, "pairs") != Some(5)
        || integer(value, "attempts") != Some(10)
        || integer(value, "failed_attempts") != Some(0)
        || integer(value, "connections") != Some(100)
        || integer(value, "subscriptions") != Some(100)
        || integer(value, "mutations_per_attempt") != Some(110_000)
    {
        problems.push("W5 HC/2 slow-consumer evidence volume changed".to_owned());
    }
    if integer(value, "median_paired_client_gross_allocated_delta_bytes") != Some(20_037_582)
        || integer(value, "median_paired_server_gross_allocated_delta_bytes") != Some(-1_829)
        || integer(value, "median_paired_client_working_set_delta_bytes") != Some(21_204_992)
        || integer(value, "median_paired_server_working_set_delta_bytes") != Some(-57_344)
        || integer(value, "median_unread_events") != Some(51_300)
        || integer(value, "median_unread_dropped_events") != Some(117_494)
        || integer(value, "control_events") != Some(110_000)
        || integer(value, "control_dropped_events") != Some(0)
    {
        problems.push("W5 HC/2 slow-consumer evidence result changed".to_owned());
    }
    if value
        .get("pair")
        .and_then(TomlValue::as_array)
        .is_none_or(|pairs| pairs.len() != 5)
        || text(value, "interpretation").is_none_or(str::is_empty)
        || text(value, "fixture_correction").is_none_or(str::is_empty)
        || text(value, "boundedness").is_none_or(str::is_empty)
        || text(value, "decision")
            != Some("accept-application-slow-consumer-as-client-owned-no-server-candidate")
        || text(value, "next_evidence").is_none_or(str::is_empty)
    {
        problems.push("W5 HC/2 slow-consumer evidence decision is incomplete".to_owned());
    }
    problems
}

pub fn check_w5_hc2_raw_transport_stall_contract(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "contract_id") != Some("w5-hc2-raw-transport-stall-073-v1")
        || text(value, "state") != Some("preregistered-before-raw-transport-tooling")
        || text(value, "triggering_evidence") != Some(W5_HC2_SLOW_CONSUMER_PROFILE_EVIDENCE)
        || text(value, "source_parent") != Some("c77e0febbb70203b0257f26b9ee6079e3c600545")
        || text(value, "profile_tool") != Some("tools/hc2-connection-profile-073")
    {
        problems.push("W5 HC/2 raw transport-stall contract identity changed".to_owned());
    }
    for field in [
        "production_counter_addition",
        "product_mutation_allowed",
        "candidate_data_allowed",
        "dedicated_host_run_allowed",
        "promotable",
        "exact_server_queued_byte_claim_allowed",
        "release_numerical_claim_allowed",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!(
                "W5 HC/2 raw transport-stall contract {field} must be false"
            ));
        }
    }
    for field in [
        "independent_process_pairs_required",
        "same_encoded_inputs_within_pair_required",
        "handshake_ack_required_before_stall",
        "subscription_ack_required_before_stall",
        "request_stream_remains_open_at_snapshot",
        "response_stream_object_retained_at_snapshot",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!(
                "W5 HC/2 raw transport-stall contract {field} must be true"
            ));
        }
    }
    if integer(value, "connections") != Some(8)
        || integer(value, "subscriptions_per_connection") != Some(1)
        || integer(value, "offered_value_bytes_per_connection") != Some(8_388_608)
        || integer_array(value.get("value_bytes")) != [4_096, 262_144]
        || integer_array(value.get("mutations_per_connection")) != [2_048, 32]
        || integer(value, "server_outbound_item_capacity") != Some(16)
        || integer(value, "settle_milliseconds") != Some(2_000)
        || integer(value, "stability_window_milliseconds") != Some(500)
        || integer(value, "pairs_per_value_size") != Some(5)
        || string_array(value.get("counterbalanced_order")).len() != 5
        || string_array(value.get("scenarios"))
            != ["drained-transport-reader", "unpolled-transport-reader"]
    {
        problems.push("W5 HC/2 raw transport-stall workload changed".to_owned());
    }
    for field in [
        "control_invariant",
        "treatment_invariant",
        "payload_invariant",
        "close_invariant",
        "falsifier",
        "interpretation_limit",
        "decision_rule",
    ] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!(
                "W5 HC/2 raw transport-stall contract {field} is missing"
            ));
        }
    }
    problems
}

pub fn check_w5_hc2_raw_transport_stall_evidence(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_id") != Some("w5-hc2-raw-transport-stall-25cdfb61-v1")
        || text(value, "state") != Some("local-raw-transport-stall-payload-sensitive-and-bounded")
        || text(value, "contract") != Some(W5_HC2_RAW_TRANSPORT_STALL_CONTRACT)
        || text(value, "source_commit") != Some("25cdfb6153e9ffe04ca67c3a7ac4bdba11380f55")
        || text(value, "profile_tool") != Some("tools/hc2-connection-profile-073")
    {
        problems.push("W5 HC/2 raw transport-stall evidence identity changed".to_owned());
    }
    for field in [
        "binary_sha256",
        "contract_sha256",
        "tool_source_sha256",
        "tool_lock_sha256",
    ] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!(
                "W5 HC/2 raw transport-stall evidence {field} is not SHA-256"
            ));
        }
    }
    let raw_results = string_array(value.get("raw_results"));
    if raw_results.len() != 20
        || raw_results.iter().any(|entry| {
            entry
                .rsplit_once(':')
                .is_none_or(|(_, digest)| !sha256(digest))
        })
    {
        problems.push("W5 HC/2 raw transport-stall raw result manifest changed".to_owned());
    }
    for field in [
        "production_counter_addition",
        "product_mutation_present",
        "candidate_data_present",
        "dedicated_host_run_allowed",
        "promotable",
        "exact_server_queued_byte_claim_allowed",
        "release_numerical_claim_allowed",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!(
                "W5 HC/2 raw transport-stall evidence {field} must be false"
            ));
        }
    }
    for field in [
        "control_invariant_passed",
        "treatment_invariant_passed",
        "stability_invariant_passed",
        "close_invariant_passed",
        "stderr_empty",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!(
                "W5 HC/2 raw transport-stall evidence {field} must be true"
            ));
        }
    }
    if integer(value, "pairs_per_value_size") != Some(5)
        || integer(value, "attempts") != Some(20)
        || integer(value, "matrix_failed_attempts") != Some(0)
        || integer(value, "fixture_smoke_failures") != Some(2)
        || integer(value, "connections") != Some(8)
        || integer(value, "subscriptions") != Some(8)
        || integer(value, "offered_value_bytes_per_connection") != Some(8_388_608)
        || integer_array(value.get("value_bytes")) != [4_096, 262_144]
        || integer_array(value.get("mutations_per_connection")) != [2_048, 32]
        || integer(value, "default_hyper_client_stream_window_bytes") != Some(2_097_152)
        || integer(value, "default_hyper_client_connection_window_bytes") != Some(5_242_880)
        || text(value, "transport_dependency") != Some("hyper-1.11.1")
        || value
            .get("pair")
            .and_then(TomlValue::as_array)
            .is_none_or(|pairs| pairs.len() != 10)
    {
        problems.push("W5 HC/2 raw transport-stall evidence volume changed".to_owned());
    }
    if integer(value, "median_unpolled_dispatch_4096") != Some(4_160)
        || integer(value, "median_unpolled_dispatch_262144") != Some(128)
        || integer(value, "median_paired_server_working_set_delta_4096_bytes") != Some(9_216_000)
        || integer(value, "median_paired_server_working_set_delta_262144_bytes") != Some(22_667_264)
        || integer(value, "median_paired_server_pagefile_delta_4096_bytes") != Some(9_461_760)
        || integer(value, "median_paired_server_pagefile_delta_262144_bytes") != Some(22_663_168)
    {
        problems.push("W5 HC/2 raw transport-stall evidence result changed".to_owned());
    }
    for field in [
        "interpretation",
        "source_attribution",
        "fixture_failures",
        "boundedness",
        "next_evidence",
    ] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!(
                "W5 HC/2 raw transport-stall evidence {field} is missing"
            ));
        }
    }
    if text(value, "decision") != Some("preregister-hc2-outbound-byte-admission-candidate") {
        problems.push("W5 HC/2 raw transport-stall evidence decision changed".to_owned());
    }
    problems
}

pub fn check_w5_hc2_outbound_byte_admission_contract(
    value: &TomlValue,
    release: &str,
) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "contract_id") != Some("w5-hc2-outbound-byte-admission-073-v1")
        || text(value, "state") != Some("preregistered-before-candidate-implementation")
        || text(value, "triggering_evidence") != Some(W5_HC2_RAW_TRANSPORT_STALL_EVIDENCE)
        || text(value, "source_parent") != Some("d715a87d0f1e52eda1417887f622f3ceafc2642f")
    {
        problems.push("W5 HC/2 outbound byte-admission contract identity changed".to_owned());
    }
    if string_array(value.get("implementation_scope"))
        != [
            "crates/hydracache-server/src/hc2.rs",
            "crates/hydracache-server/Cargo.toml",
        ]
        || integer(value, "application_queue_byte_budget") != Some(1_048_576)
        || text(value, "application_queue_item_capacity_source")
            != Some("ClientSurfaceLimits.max_streams_per_connection")
        || integer(value, "default_application_queue_item_capacity") != Some(16)
        || text(value, "charge_measure") != Some("prost::Message::encoded_len(ServerEnvelope)")
    {
        problems.push("W5 HC/2 outbound byte-admission design changed".to_owned());
    }
    for field in [
        "public_configuration_changed",
        "wire_contract_changed",
        "client_api_changed",
        "production_counter_addition",
        "item_capacity_changed",
        "dispatch_semantics_changed",
        "frame_rejection_added",
        "http2_window_claimed",
        "tls_or_socket_bytes_claimed",
        "dedicated_host_run_allowed",
        "release_numerical_claim_allowed",
        "promotable",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!(
                "W5 HC/2 outbound byte-admission contract {field} must be false"
            ));
        }
    }
    for field in [
        "candidate_implementation_allowed",
        "encoded_length_charging_required",
        "oversize_progress_required",
        "permit_release_on_send_failure_required",
        "permit_release_on_disconnect_required",
        "existing_item_bound_required",
        "existing_ordering_required",
        "existing_backpressure_required",
        "existing_close_reconciliation_required",
        "local_raw_matrix_required",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!(
                "W5 HC/2 outbound byte-admission contract {field} must be true"
            ));
        }
    }
    if integer_array(value.get("value_bytes")) != [4_096, 262_144]
        || integer(value, "pairs_per_value_size") != Some(5)
        || integer(value, "connections") != Some(8)
        || integer(value, "offered_value_bytes_per_connection") != Some(8_388_608)
    {
        problems.push("W5 HC/2 outbound byte-admission validation matrix changed".to_owned());
    }
    for field in [
        "oversize_policy",
        "permit_lifetime",
        "single_producer_bound",
        "candidate_success_rule",
        "candidate_falsifier",
        "claim_boundary",
        "rollback",
        "next_decision",
    ] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!(
                "W5 HC/2 outbound byte-admission contract {field} is missing"
            ));
        }
    }
    problems
}

pub fn check_w5_hc2_outbound_byte_admission_evidence(
    value: &TomlValue,
    release: &str,
) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_id") != Some("w5-hc2-outbound-byte-admission-2846e936-v1")
        || text(value, "state") != Some("local-candidate-accepted-for-integrated-campaign")
        || text(value, "contract") != Some(W5_HC2_OUTBOUND_BYTE_ADMISSION_CONTRACT)
        || text(value, "triggering_evidence") != Some(W5_HC2_RAW_TRANSPORT_STALL_EVIDENCE)
        || text(value, "implementation_commit") != Some("2846e93633cc14dea5c00b2f742e27260c794875")
        || text(value, "profile_tool") != Some("tools/hc2-connection-profile-073")
    {
        problems.push("W5 HC/2 outbound byte-admission evidence identity changed".to_owned());
    }
    for field in [
        "binary_sha256",
        "contract_sha256",
        "server_source_sha256",
        "root_lock_sha256",
        "tool_lock_sha256",
    ] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!(
                "W5 HC/2 outbound byte-admission evidence {field} is not SHA-256"
            ));
        }
    }
    let raw_results = string_array(value.get("raw_results"));
    if raw_results.len() != 20
        || raw_results.iter().any(|entry| {
            entry
                .rsplit_once(':')
                .is_none_or(|(_, digest)| !sha256(digest))
        })
    {
        problems.push("W5 HC/2 outbound byte-admission raw result manifest changed".to_owned());
    }
    for field in [
        "public_configuration_changed",
        "wire_contract_changed",
        "frame_rejection_added",
        "dedicated_host_run_allowed",
        "dedicated_host_repeat_authorized",
        "release_numerical_claim_allowed",
        "promotable",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!(
                "W5 HC/2 outbound byte-admission evidence {field} must be false"
            ));
        }
    }
    for field in [
        "product_mutation_present",
        "candidate_data_present",
        "candidate_accepted_locally",
        "unit_invariants_passed",
        "real_mtls_invariant_passed",
        "full_server_suite_passed",
        "strict_clippy_passed",
        "control_invariant_passed",
        "treatment_invariant_passed",
        "stability_invariant_passed",
        "close_invariant_passed",
        "stderr_empty",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!(
                "W5 HC/2 outbound byte-admission evidence {field} must be true"
            ));
        }
    }
    if integer(value, "application_queue_byte_budget") != Some(1_048_576)
        || integer(value, "application_queue_item_capacity") != Some(16)
        || integer(value, "pairs_per_value_size") != Some(5)
        || integer(value, "attempts") != Some(20)
        || integer(value, "failed_attempts") != Some(0)
        || integer(value, "connections") != Some(8)
        || integer(value, "subscriptions") != Some(8)
        || integer(value, "offered_value_bytes_per_connection") != Some(8_388_608)
        || integer_array(value.get("value_bytes")) != [4_096, 262_144]
        || integer_array(value.get("mutations_per_connection")) != [2_048, 32]
        || value
            .get("pair")
            .and_then(TomlValue::as_array)
            .is_none_or(|pairs| pairs.len() != 10)
    {
        problems.push("W5 HC/2 outbound byte-admission evidence volume changed".to_owned());
    }
    if integer(value, "baseline_median_unpolled_dispatch_4096") != Some(4_160)
        || integer(value, "candidate_median_unpolled_dispatch_4096") != Some(4_152)
        || integer(value, "baseline_median_unpolled_dispatch_262144") != Some(128)
        || integer(value, "candidate_median_unpolled_dispatch_262144") != Some(104)
        || integer(
            value,
            "baseline_median_paired_server_working_set_delta_4096_bytes",
        ) != Some(9_216_000)
        || integer(
            value,
            "candidate_median_paired_server_working_set_delta_4096_bytes",
        ) != Some(9_863_168)
        || integer(
            value,
            "baseline_median_paired_server_working_set_delta_262144_bytes",
        ) != Some(22_667_264)
        || integer(
            value,
            "candidate_median_paired_server_working_set_delta_262144_bytes",
        ) != Some(15_949_824)
        || integer(value, "high_payload_working_set_reduction_bytes") != Some(6_717_440)
        || integer(
            value,
            "candidate_median_paired_server_pagefile_delta_262144_bytes",
        ) != Some(15_167_488)
        || integer(value, "high_payload_pagefile_reduction_bytes") != Some(7_495_680)
    {
        problems.push("W5 HC/2 outbound byte-admission evidence result changed".to_owned());
    }
    for field in [
        "interpretation",
        "correctness",
        "claim_boundary",
        "small_payload_result",
        "next_evidence",
    ] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!(
                "W5 HC/2 outbound byte-admission evidence {field} is missing"
            ));
        }
    }
    if text(value, "decision") != Some("retain-byte-admission-for-integrated-w5-w2-campaign") {
        problems.push("W5 HC/2 outbound byte-admission evidence decision changed".to_owned());
    }
    problems
}

pub fn check_w6_management_overhead_profile_contract(
    value: &TomlValue,
    release: &str,
) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "contract_id") != Some("w6-management-overhead-profile-073-v1")
        || text(value, "state") != Some("preregistered-before-profile-tooling")
        || text(value, "parent_contract") != Some(W1_OWNER_CLASSIFICATION_CONTRACT)
        || text(value, "source_parent") != Some("bd3d3100a12fd9f9b33ea37e59ff9bbc8c6d1427")
        || text(value, "profile_tool") != Some("tools/management-overhead-profile-073")
        || text(value, "ownership_scope")
            != Some("single-process management router and request-owned work")
    {
        problems.push("W6 management overhead profile identity changed".to_owned());
    }
    for field in [
        "production_counter_addition",
        "product_mutation_allowed",
        "candidate_data_allowed",
        "dedicated_host_run_allowed",
        "promotable",
        "release_numerical_claim_allowed",
        "fixed_task_cost_claim_allowed",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!(
                "W6 management overhead profile {field} must be false"
            ));
        }
    }
    for field in [
        "independent_process_pairs_required",
        "prebuilt_release_binary_required",
        "no_background_collector_expected",
        "history_request_scoped_expected",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!(
                "W6 management overhead profile {field} must be true"
            ));
        }
    }
    if integer(value, "polls_per_second") != Some(1)
        || integer(value, "polling_requests") != Some(60)
        || integer(value, "idle_observation_milliseconds") != Some(2_000)
        || integer(value, "cursor_records") != Some(1_024)
        || integer(value, "idle_pairs") != Some(5)
        || integer(value, "repeats_per_request_scenario") != Some(5)
        || string_array(value.get("counterbalanced_order"))
            != ["off/on", "on/off", "off/on", "on/off", "off/on"]
    {
        problems.push("W6 management overhead profile sampling matrix changed".to_owned());
    }
    if string_array(value.get("scenarios"))
        != [
            "management-off-idle",
            "management-on-idle",
            "dashboard-poll",
            "aggregate-cold",
            "aggregate-cache-hit",
            "cursor-saturation",
            "history-disabled",
        ]
        || string_array(value.get("metrics"))
            != [
                "gross_allocated_bytes",
                "live_allocated_bytes",
                "working_set_bytes",
                "pagefile_bytes",
                "serialized_response_bytes",
                "request_elapsed_nanoseconds",
                "aggregate_transport_calls",
                "retained_cursor_records",
            ]
    {
        problems.push("W6 management overhead profile scenario or metric set changed".to_owned());
    }
    for field in [
        "falsifier",
        "interpretation_limit",
        "decision_rule",
        "next_decision",
    ] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!("W6 management overhead profile {field} is missing"));
        }
    }
    problems
}

pub fn check_w6_management_overhead_profile_evidence(
    value: &TomlValue,
    release: &str,
) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_id") != Some("w6-management-overhead-316961e0-v1")
        || text(value, "state") != Some("local-profile-passed-measured-no-win")
        || text(value, "contract") != Some(W6_MANAGEMENT_OVERHEAD_PROFILE_CONTRACT)
        || text(value, "source_commit") != Some("316961e0148a814020db179c543c3e7e01af21b0")
        || text(value, "profile_tool") != Some("tools/management-overhead-profile-073")
    {
        problems.push("W6 management overhead evidence identity changed".to_owned());
    }
    for field in [
        "binary_sha256",
        "contract_sha256",
        "tool_source_sha256",
        "tool_lock_sha256",
    ] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!(
                "W6 management overhead evidence {field} is not SHA-256"
            ));
        }
    }
    let raw_results = string_array(value.get("raw_results"));
    if raw_results.len() != 35
        || raw_results.iter().any(|entry| {
            entry
                .rsplit_once(':')
                .is_none_or(|(_, digest)| !sha256(digest))
        })
    {
        problems.push("W6 management overhead raw result manifest changed".to_owned());
    }
    for field in [
        "production_counter_addition",
        "product_mutation_present",
        "candidate_data_present",
        "dedicated_host_run_allowed",
        "promotable",
        "release_numerical_claim_allowed",
        "fixed_task_cost_claim_allowed",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!(
                "W6 management overhead evidence {field} must be false"
            ));
        }
    }
    for field in [
        "independent_processes",
        "counterbalanced_idle_order",
        "stderr_empty",
        "all_invariants_passed",
        "cursor_oldest_eviction_passed",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!(
                "W6 management overhead evidence {field} must be true"
            ));
        }
    }
    if integer(value, "attempts") != Some(35)
        || integer(value, "failed_attempts") != Some(0)
        || integer(value, "idle_pairs") != Some(5)
        || integer(value, "request_repeats_per_scenario") != Some(5)
        || integer(value, "polling_requests") != Some(60)
        || integer(value, "polls_per_second") != Some(1)
        || integer(value, "cursor_issue_requests") != Some(1_025)
        || integer(value, "management_source_spawn_sites") != Some(0)
        || integer(value, "idle_transport_calls") != Some(0)
    {
        problems.push("W6 management overhead evidence volume changed".to_owned());
    }
    if integer(value, "idle_median_paired_gross_delta_bytes") != Some(95_261)
        || integer(value, "idle_median_paired_live_delta_bytes") != Some(21_797)
        || integer(value, "idle_median_paired_working_set_delta_bytes") != Some(143_360)
        || integer(value, "idle_median_paired_pagefile_delta_bytes") != Some(65_536)
        || integer(value, "dashboard_median_gross_allocated_bytes") != Some(927_564)
        || float(value, "dashboard_gross_allocated_bytes_per_read") != Some(15_459.4)
        || integer(value, "dashboard_serialized_bytes_per_read") != Some(1_901)
        || integer(value, "history_disabled_median_gross_allocated_bytes") != Some(536_520)
        || float(value, "history_disabled_gross_allocated_bytes_per_read") != Some(8_942.0)
        || integer(value, "history_disabled_serialized_bytes_per_read") != Some(351)
        || integer(value, "aggregate_cold_median_gross_allocated_bytes") != Some(599_684)
        || integer(value, "aggregate_cache_hit_median_gross_allocated_bytes") != Some(362_949)
        || integer(value, "aggregate_cold_transport_calls") != Some(60)
        || integer(value, "aggregate_cache_hit_transport_calls") != Some(1)
        || integer(value, "cursor_median_gross_allocated_bytes") != Some(53_105_466)
        || integer(value, "cursor_retained_records") != Some(1_024)
    {
        problems.push("W6 management overhead evidence result changed".to_owned());
    }
    for field in [
        "idle_working_set_outlier_retained",
        "source_audit",
        "allocation_interpretation",
        "memory_interpretation",
        "next_evidence",
    ] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!(
                "W6 management overhead evidence {field} is missing"
            ));
        }
    }
    if text(value, "decision") != Some("close-w6-local-measured-no-win") {
        problems.push("W6 management overhead evidence decision changed".to_owned());
    }
    problems
}

pub fn check_w3_tag_index_profile_contract(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "contract_id") != Some("w3-tag-index-profile-073-v1")
        || text(value, "state") != Some("preregistered-before-profile-tooling")
        || text(value, "parent_contract") != Some(W1_OWNER_CLASSIFICATION_CONTRACT)
        || text(value, "source_parent") != Some("188aeda1512ca8e9afe60ff03797aa3616aa9f22")
        || text(value, "profile_tool") != Some("tools/tag-index-profile-073")
    {
        problems.push("W3 tag-index profile identity changed".to_owned());
    }
    for field in [
        "production_counter_addition",
        "product_mutation_allowed",
        "candidate_data_allowed",
        "dedicated_host_run_allowed",
        "promotable",
        "release_numerical_claim_allowed",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!("W3 tag-index profile {field} must be false"));
        }
    }
    for field in [
        "independent_processes_required",
        "prebuilt_release_binary_required",
        "exact_memory_reconciliation_required",
        "event_content_equality_required",
        "generation_fence_required",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!("W3 tag-index profile {field} must be true"));
        }
    }
    if integer(value, "entry_count") != Some(256)
        || integer(value, "key_bytes") != Some(32)
        || integer(value, "tag_bytes") != Some(32)
        || integer(value, "repeats_per_scenario") != Some(5)
        || integer_array(value.get("tag_cardinalities")) != [0, 1, 4, 16, 64]
        || integer_array(value.get("event_subscriber_counts")) != [0, 1, 8]
        || integer_array(value.get("invalidation_fanouts")) != [1, 64, 1_024]
        || string_array(value.get("tag_topologies")) != ["shared-tag-set", "unique-tag-set"]
    {
        problems.push("W3 tag-index profile sampling matrix changed".to_owned());
    }
    if string_array(value.get("metrics"))
        != [
            "gross_allocated_bytes",
            "live_allocated_bytes",
            "estimated_tag_retained_bytes",
            "tag_memberships",
            "event_tag_payload_bytes",
            "event_delivery_count",
            "invalidation_removed_keys",
            "elapsed_nanoseconds",
        ]
    {
        problems.push("W3 tag-index profile metric set changed".to_owned());
    }
    for field in [
        "ownership_scope",
        "falsifier",
        "interpretation_limit",
        "decision_rule",
        "next_decision",
    ] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!("W3 tag-index profile {field} is missing"));
        }
    }
    problems
}

pub fn check_w3_tag_index_profile_evidence(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_id") != Some("w3-tag-index-profile-f8b90ef0-v1")
        || text(value, "state") != Some("local-profile-passed-candidate-owner-found")
        || text(value, "contract") != Some(W3_TAG_INDEX_PROFILE_CONTRACT)
        || text(value, "source_commit") != Some("f8b90ef0f6c9d7bfc42dbd367c87cce4a2ef2cf8")
        || text(value, "profile_tool") != Some("tools/tag-index-profile-073")
    {
        problems.push("W3 tag-index profile evidence identity changed".to_owned());
    }
    for field in [
        "binary_sha256",
        "contract_sha256",
        "tool_source_sha256",
        "tool_lock_sha256",
        "raw_manifest_sha256",
    ] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!(
                "W3 tag-index profile evidence {field} is not SHA-256"
            ));
        }
    }
    for field in [
        "production_counter_addition",
        "product_mutation_present",
        "candidate_data_present",
        "dedicated_host_run_allowed",
        "promotable",
        "release_numerical_claim_allowed",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!(
                "W3 tag-index profile evidence {field} must be false"
            ));
        }
    }
    for field in [
        "independent_processes",
        "prebuilt_release_binary",
        "exact_memory_reconciliation_passed",
        "event_content_equality_passed",
        "generation_fence_passed",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!(
                "W3 tag-index profile evidence {field} must be true"
            ));
        }
    }
    if integer(value, "attempts") != Some(215)
        || integer(value, "raw_stdout_results") != Some(215)
        || integer(value, "raw_stderr_results") != Some(215)
        || integer(value, "raw_exit_results") != Some(215)
        || integer(value, "failed_attempts") != Some(0)
        || integer(value, "nonempty_stderr") != Some(0)
        || integer(value, "invariant_failures") != Some(0)
        || integer(value, "repeats_per_scenario") != Some(5)
        || integer(value, "index_scenarios") != Some(10)
        || integer(value, "event_scenarios") != Some(30)
        || integer(value, "invalidation_scenarios") != Some(3)
    {
        problems.push("W3 tag-index profile evidence volume changed".to_owned());
    }
    if integer_array(value.get("tag_cardinalities")) != [0, 1, 4, 16, 64]
        || integer_array(value.get("estimated_tag_retained_bytes"))
            != [0, 47_104, 188_416, 753_664, 3_014_656]
        || integer_array(value.get("index_shared_median_gross_allocated_bytes"))
            != [183_424, 308_944, 684_924, 2_191_876, 8_213_340]
        || integer_array(value.get("index_unique_median_gross_allocated_bytes"))
            != [185_520, 388_228, 999_452, 3_449_996, 13_241_308]
        || integer_array(value.get("event_shared_s8_median_gross_allocated_bytes"))
            != [256_992, 498_232, 1_218_436, 4_100_548, 15_628_164]
        || integer_array(value.get("event_unique_s8_median_gross_allocated_bytes"))
            != [259_152, 576_708, 1_532_836, 5_357_572, 20_656_068]
        || integer_array(value.get("invalidation_fanouts")) != [1, 64, 1_024]
        || integer_array(value.get("invalidation_removed_keys")) != [1, 64, 1_024]
        || integer_array(value.get("invalidation_post_tag_memberships")) != [0, 0, 0]
        || integer_array(value.get("invalidation_post_generation_records")) != [1, 1, 1]
    {
        problems.push("W3 tag-index profile evidence result vectors changed".to_owned());
    }
    if float(value, "event_s1_tag_churn_bytes_per_delivery_tag") != Some(55.8701)
        || float(value, "event_s8_tag_churn_bytes_per_delivery_tag") != Some(56.0002)
        || integer(value, "event_shared_64_s8_payload_bytes") != Some(4_194_304)
        || integer(value, "event_shared_64_s8_gross_increment_bytes") != Some(7_412_728)
        || integer(value, "event_shared_64_s8_tag_only_gross_increment_bytes") != Some(7_340_064)
    {
        problems.push("W3 tag-index profile event owner result changed".to_owned());
    }
    for field in [
        "raw_manifest_canonicalization",
        "source_audit",
        "estimator_interpretation",
        "index_interpretation",
        "event_interpretation",
        "invalidation_interpretation",
        "next_evidence",
    ] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!("W3 tag-index profile evidence {field} is missing"));
        }
    }
    if text(value, "decision") != Some("preregister-shared-cache-event-tags-candidate") {
        problems.push("W3 tag-index profile evidence decision changed".to_owned());
    }
    problems
}

pub fn check_w3_shared_event_tags_contract(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "contract_id") != Some("w3-shared-event-tags-073-v1")
        || text(value, "state") != Some("preregistered-before-product-mutation")
        || text(value, "parent_contract") != Some(W3_TAG_INDEX_PROFILE_CONTRACT)
        || text(value, "baseline_evidence") != Some(W3_TAG_INDEX_PROFILE_EVIDENCE)
        || text(value, "source_parent") != Some("c213bc1bdf19fb29562241d71ea6ef4a28acd055")
    {
        problems.push("W3 shared event tags contract identity changed".to_owned());
    }
    for field in [
        "independent_processes_required",
        "prebuilt_release_binary_required",
        "exact_memory_reconciliation_required",
        "event_content_equality_required",
        "generation_fence_required",
        "public_tags_accessor_unchanged",
        "event_equality_semantics_unchanged",
        "product_mutation_allowed",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!(
                "W3 shared event tags contract {field} must be true"
            ));
        }
    }
    for field in [
        "production_counter_addition",
        "dedicated_host_run_allowed",
        "promotable",
        "release_numerical_claim_allowed",
        "elapsed_time_is_acceptance_metric",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!(
                "W3 shared event tags contract {field} must be false"
            ));
        }
    }
    if integer(value, "entry_count") != Some(256)
        || integer(value, "key_bytes") != Some(32)
        || integer(value, "tag_bytes") != Some(32)
        || integer(value, "repeats_per_scenario") != Some(5)
        || integer(value, "candidate_attempts") != Some(215)
        || integer_array(value.get("tag_cardinalities")) != [0, 1, 4, 16, 64]
        || string_array(value.get("tag_topologies")) != ["shared-tag-set", "unique-tag-set"]
        || integer_array(value.get("event_subscriber_counts")) != [0, 1, 8]
        || integer_array(value.get("invalidation_fanouts")) != [1, 64, 1_024]
    {
        problems.push("W3 shared event tags contract sampling matrix changed".to_owned());
    }
    if string_array(value.get("candidate_files")) != ["crates/hydracache-core/src/events.rs"]
        || text(value, "primary_cell") != Some("event/shared-tag-set/64-tags/8-subscribers")
        || integer(value, "baseline_primary_median_gross_allocated_bytes") != Some(15_628_164)
        || float(value, "minimum_primary_total_gross_reduction_fraction") != Some(0.30)
        || integer(value, "baseline_primary_tag_only_gross_increment_bytes") != Some(7_340_064)
        || float(value, "maximum_candidate_tag_churn_bytes_per_delivery_tag") != Some(12.0)
        || float(value, "maximum_no_subscriber_gross_regression_fraction") != Some(0.05)
        || float(value, "maximum_index_live_delta_regression_fraction") != Some(0.10)
        || float(value, "maximum_invalidation_gross_regression_fraction") != Some(0.10)
    {
        problems.push("W3 shared event tags contract thresholds changed".to_owned());
    }
    for field in [
        "candidate_scope",
        "falsifier",
        "interpretation_limit",
        "decision_rule",
        "next_decision",
    ] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!("W3 shared event tags contract {field} is missing"));
        }
    }
    problems
}

pub fn check_w3_shared_event_tags_evidence(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1)
        || text(value, "release") != Some(release)
        || text(value, "evidence_id") != Some("w3-shared-event-tags-accepted-82f46245-v1")
        || text(value, "state") != Some("local-candidate-accepted-for-integration")
        || text(value, "contract") != Some(W3_SHARED_EVENT_TAGS_CONTRACT)
        || text(value, "baseline_evidence") != Some(W3_TAG_INDEX_PROFILE_EVIDENCE)
        || text(value, "source_commit") != Some("82f46245af235536a6dac93a8fbc59ecaeb6cc7d")
    {
        problems.push("W3 shared event tags evidence identity changed".to_owned());
    }
    for field in [
        "binary_sha256",
        "contract_sha256",
        "product_source_sha256",
        "tool_source_sha256",
        "tool_lock_sha256",
        "raw_manifest_sha256",
    ] {
        if text(value, field).is_none_or(|digest| !sha256(digest)) {
            problems.push(format!(
                "W3 shared event tags evidence {field} is not SHA-256"
            ));
        }
    }
    for field in [
        "independent_processes",
        "prebuilt_release_binary",
        "exact_memory_reconciliation_passed",
        "event_content_equality_passed",
        "generation_fence_passed",
        "public_tags_accessor_unchanged",
        "event_equality_semantics_unchanged",
        "product_mutation_present",
        "all_thresholds_passed",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!(
                "W3 shared event tags evidence {field} must be true"
            ));
        }
    }
    for field in [
        "production_counter_addition",
        "dedicated_host_run_allowed",
        "promotable",
        "release_numerical_claim_allowed",
        "elapsed_time_used_for_acceptance",
    ] {
        if boolean(value, field) != Some(false) {
            problems.push(format!(
                "W3 shared event tags evidence {field} must be false"
            ));
        }
    }
    if integer(value, "attempts") != Some(215)
        || integer(value, "failed_attempts") != Some(0)
        || integer(value, "nonempty_stderr") != Some(0)
        || integer(value, "invariant_failures") != Some(0)
        || integer(value, "repeats_per_scenario") != Some(5)
        || integer_array(value.get("tag_cardinalities")) != [0, 1, 4, 16, 64]
    {
        problems.push("W3 shared event tags evidence volume changed".to_owned());
    }
    if integer(value, "baseline_primary_median_gross_allocated_bytes") != Some(15_628_164)
        || integer(value, "candidate_primary_median_gross_allocated_bytes") != Some(8_686_476)
        || float(value, "primary_total_gross_reduction_fraction") != Some(0.444178)
        || float(value, "minimum_primary_total_gross_reduction_fraction") != Some(0.30)
        || float(value, "baseline_s8_tag_churn_bytes_per_delivery_tag") != Some(56.0002)
        || float(value, "candidate_s8_tag_churn_bytes_per_delivery_tag") != Some(3.0084)
        || float(value, "maximum_candidate_tag_churn_bytes_per_delivery_tag") != Some(12.0)
    {
        problems.push("W3 shared event tags evidence primary result changed".to_owned());
    }
    if float(
        value,
        "maximum_observed_no_subscriber_gross_regression_fraction",
    ) != Some(0.011079)
        || float(
            value,
            "maximum_observed_index_live_delta_regression_fraction",
        ) != Some(0.014056)
        || float(
            value,
            "maximum_observed_invalidation_gross_regression_fraction",
        ) != Some(0.0)
        || integer_array(value.get("invalidation_removed_keys")) != [1, 64, 1_024]
        || integer_array(value.get("invalidation_post_tag_memberships")) != [0, 0, 0]
        || integer_array(value.get("invalidation_post_generation_records")) != [1, 1, 1]
    {
        problems.push("W3 shared event tags evidence control result changed".to_owned());
    }
    for field in [
        "raw_manifest_canonicalization",
        "ownership_interpretation",
        "control_interpretation",
        "next_evidence",
    ] {
        if text(value, field).is_none_or(str::is_empty) {
            problems.push(format!("W3 shared event tags evidence {field} is missing"));
        }
    }
    if text(value, "decision")
        != Some("retain-shared-cache-event-tags-for-integrated-w3-w5-confirmation")
    {
        problems.push("W3 shared event tags evidence decision changed".to_owned());
    }
    problems
}

pub fn check_instrumentation_overhead(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1) || text(value, "release") != Some(release) {
        problems.push("instrumentation overhead schema/release mismatch".to_owned());
        return problems;
    }
    if text(value, "contract_id") != Some("instrumentation-overhead-073-v1")
        || text(value, "measurement_profile") != Some("instrumentation-overhead-073-v1")
        || text(value, "resource_phase_schema")
            != Some("docs/testing/performance/0.73/resource-phase-v1.schema.json")
        || text(value, "state") != Some("pilot")
        || boolean(value, "i73_freeze_allowed") != Some(false)
        || boolean(value, "candidate_data_allowed") != Some(false)
    {
        problems.push("instrumentation overhead must remain a candidate-blocking pilot".to_owned());
    }
    if text(value, "comparison") != Some("off_vs_production")
        || text(value, "classification_only_mode") != Some("profile")
    {
        problems.push(
            "instrumentation overhead must compare off/production and isolate profile".to_owned(),
        );
    }
    for field in [
        "same_source_required",
        "same_binary_required",
        "same_toolchain_required",
        "same_host_required",
        "same_scenario_required",
        "same_trace_required",
        "counterbalanced_order_required",
        "independent_processes_required",
        "complete_outcome_accounting_required",
        "raw_series_required",
        "failed_attempts_preserved",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!("instrumentation overhead requires {field}=true"));
        }
    }
    if integer(value, "minimum_screening_pairs").is_none_or(|count| count < 3)
        || integer(value, "minimum_qualification_pairs").is_none_or(|count| count < 5)
        || float(value, "throughput_regression_limit") != Some(0.02)
        || float(value, "cpu_per_request_regression_limit") != Some(0.03)
        || float(value, "p99_regression_limit") != Some(0.03)
        || integer(value, "unexpected_failures_allowed") != Some(0)
    {
        problems.push("instrumentation overhead weakens inherited regression limits".to_owned());
    }
    if text(value, "allocation_regression_limit_state") != Some("frozen-single-maintainer-review")
        || text(value, "rss_delta_limit_state") != Some("frozen-single-maintainer-review")
    {
        problems.push(
            "allocation/RSS limits must remain bound to the single-maintainer threshold review"
                .to_owned(),
        );
    }
    let metrics: BTreeSet<_> = string_array(value.get("required_metrics"))
        .into_iter()
        .collect();
    for metric in [
        "goodput",
        "cpu_per_request",
        "p99",
        "allocations_per_request",
        "allocated_bytes_per_request",
        "rss_delta_bytes",
    ] {
        if !metrics.contains(metric) {
            problems.push(format!(
                "instrumentation overhead omits required metric {metric}"
            ));
        }
    }
    let workloads: BTreeSet<_> = string_array(value.get("required_workloads"))
        .into_iter()
        .collect();
    for workload in ["cold", "small-hot", "tag-heavy", "hc2-1000", "reset"] {
        if !workloads.contains(workload) {
            problems.push(format!(
                "instrumentation overhead omits workload {workload}"
            ));
        }
    }
    if string_array(value.get("blocking_before_i73")).len() < 5 {
        problems.push("instrumentation overhead does not enumerate all I73 blockers".to_owned());
    }
    problems
}

pub fn check_scenario_matrix(value: &TomlValue, release: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1) || text(value, "release") != Some(release) {
        problems.push("scenario matrix schema/release mismatch".to_owned());
        return problems;
    }
    if text(value, "matrix_id") != Some("performance-073-v1")
        || text(value, "state") != Some("i73-frozen")
        || boolean(value, "candidate_measurement_allowed") != Some(true)
    {
        problems.push("scenario matrix must retain the admitted I73 freeze".to_owned());
    }
    if text(value, "baseline_identity") != Some("I73")
        || text(value, "baseline_source_sha") != Some("e757556d3a31d565f52a9561d6d4e555bb1cc373")
        || text(value, "baseline_freeze_evidence") != Some(BASELINE_PILOT_V2_FREEZE_EVIDENCE)
        || text(value, "external_baseline_identity") != Some("B72")
    {
        problems
            .push("scenario matrix must bind frozen I73 separately from external B72".to_owned());
    }
    if text(value, "calibration_state") != Some("admitted-pre-and-post")
        || text(value, "measurement_windows_state") != Some("frozen-10-seconds")
        || text(value, "stable_rates_state") != Some("frozen")
        || integer_array(value.get("stable_rates_per_second")) != [2_500, 5_000, 10_000, 20_000]
        || integer(value, "selected_knee_rate_per_second") != Some(20_000)
        || integer_array(value.get("frozen_d3_rates_per_second")) != [5_000, 12_000, 17_000]
    {
        problems
            .push("scenario matrix changed the admitted calibration, window, or rates".to_owned());
    }
    if integer_array(value.get("concurrency_lanes")) != [1, 8, 32, 128]
        || float_array(value.get("offered_load_fractions")) != [0.25, 0.60, 0.85]
    {
        problems.push("scenario matrix changed the frozen concurrency/load lanes".to_owned());
    }
    if float(value, "maximum_goodput_regression") != Some(0.02)
        || integer(value, "unexpected_failures_allowed") != Some(0)
        || integer(value, "minimum_screening_pairs").is_none_or(|value| value < 3)
        || integer(value, "minimum_claim_pairs").is_none_or(|value| value < 5)
    {
        problems.push("scenario matrix weakened comparison admission".to_owned());
    }
    let expected_modes = ["off", "production", "profile"];
    if string_array(value.get("instrumentation_modes")) != expected_modes
        || text(value, "candidate_comparison_instrumentation_mode") != Some("production")
        || boolean(value, "profile_samples_classification_only") != Some(true)
    {
        problems.push("scenario matrix conflates production and profile identities".to_owned());
    }
    for field in [
        "counterbalanced_pair_order_required",
        "raw_series_required",
        "complete_outcome_accounting_required",
    ] {
        if boolean(value, field) != Some(true) {
            problems.push(format!("scenario matrix requires {field}=true"));
        }
    }
    let trace = value.get("trace").unwrap_or(&TomlValue::Boolean(false));
    if text(trace, "format") != Some("length-framed-v1") {
        problems.push("scenario matrix requires the length-framed-v1 request trace".to_owned());
    }
    for field in [
        "byte_identical_between_roles",
        "seed_frozen_before_candidate",
        "pacing_frozen_before_candidate",
        "warmup_frozen_before_candidate",
        "duration_frozen_before_candidate",
        "shutdown_and_drain_frozen_before_candidate",
    ] {
        if boolean(trace, field) != Some(true) {
            problems.push(format!("scenario matrix trace requires {field}=true"));
        }
    }
    let baseline_freeze = value
        .get("baseline_freeze")
        .unwrap_or(&TomlValue::Boolean(false));
    if text(baseline_freeze, "evidence_class") != Some("dedicated_host_baseline_only")
        || boolean(baseline_freeze, "promotable") != Some(false)
        || boolean(baseline_freeze, "numerical_claim_eligible") != Some(false)
        || boolean(baseline_freeze, "candidate_data_present") != Some(false)
    {
        problems.push(
            "scenario matrix baseline freeze must remain baseline-only and non-promotable"
                .to_owned(),
        );
    }
    let mut surfaces = BTreeMap::new();
    for surface in value
        .get("surfaces")
        .and_then(TomlValue::as_array)
        .into_iter()
        .flatten()
    {
        let work_item = text(surface, "work_item").unwrap_or_default();
        if surfaces.insert(work_item, surface).is_some() {
            problems.push(format!("scenario matrix duplicates {work_item}"));
        }
        if boolean(surface, "d1_analysis_required") != Some(true)
            || boolean(surface, "local_screening_required") != Some(true)
            || string_array(surface.get("protocols")).is_empty()
            || string_array(surface.get("persistence_modes")).is_empty()
            || string_array(surface.get("primary_observations")).is_empty()
        {
            problems.push(format!(
                "scenario matrix has incomplete surface {work_item}"
            ));
        }
    }
    let expected_surfaces = [
        ("W2", "shared-store-expiry", "A"),
        ("W3", "tag-index", "B"),
        ("W4", "resp-translation", "B"),
        ("W5", "hc2-connection-state", "A"),
        ("W6", "management-service-overhead", "A"),
        ("W7", "durability-page-cache", "B"),
        ("W8", "allocator", "B"),
        ("W9", "retained-byte-admission", "C"),
    ];
    for (work_item, id, wave) in expected_surfaces {
        match surfaces.get(work_item) {
            Some(surface)
                if text(surface, "id") == Some(id) && text(surface, "wave") == Some(wave) => {}
            Some(_) => problems.push(format!(
                "scenario matrix changes {work_item} identity or wave"
            )),
            None => problems.push(format!("scenario matrix omits mandatory {work_item}")),
        }
    }
    if surfaces.get("W7").is_none_or(|surface| {
        !string_array(surface.get("persistence_modes")).contains(&"all-supported-separate")
    }) || surfaces.get("W9").is_none_or(|surface| {
        !string_array(surface.get("persistence_modes")).contains(&"all-supported-separate")
    }) {
        problems.push("W7/W9 must keep supported persistence modes in separate cells".to_owned());
    }
    let mixed = value
        .get("mixed_runtime")
        .unwrap_or(&TomlValue::Boolean(false));
    let weights = mixed
        .get("weights_percent")
        .unwrap_or(&TomlValue::Boolean(false));
    let expected_weights = [
        ("hc2_key_and_subscription", 35),
        ("resp", 30),
        ("hc1_native_client", 15),
        ("direct_local_client_surface", 10),
        ("tag_invalidation", 5),
        ("ttl_expire_refill", 5),
    ];
    let weight_total: i64 = expected_weights
        .iter()
        .map(|(field, expected)| {
            let actual = integer(weights, field).unwrap_or_default();
            if actual != *expected {
                problems.push(format!("mixed-runtime weight {field} must be {expected}%"));
            }
            actual
        })
        .sum();
    if text(mixed, "id") != Some("mixed-runtime-073-v1")
        || text(mixed, "state") != Some("unmeasured")
        || integer(mixed, "management_reads_per_second") != Some(1)
        || boolean(mixed, "persistence_modes_separate") != Some(true)
        || boolean(mixed, "aggregate_cannot_hide_surface_regression") != Some(true)
        || weight_total != 100
    {
        problems.push(
            "mixed-runtime pilot contract is incomplete or weights do not total 100%".to_owned(),
        );
    }
    problems
}

pub fn check_baseline_identities(
    root: &Path,
    value: &TomlValue,
    release: &str,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1) || text(value, "release") != Some(release) {
        problems.push("baseline identity schema/release mismatch".to_owned());
        return Ok(problems);
    }
    let baseline = value
        .get("published_baseline")
        .unwrap_or(&TomlValue::Boolean(false));
    let branch_root = value
        .get("branch_root")
        .unwrap_or(&TomlValue::Boolean(false));
    let instrumented = value
        .get("instrumented_baseline")
        .unwrap_or(&TomlValue::Boolean(false));
    let tag = text(baseline, "tag").unwrap_or_default();
    let tag_object = text(baseline, "tag_object_sha").unwrap_or_default();
    let peeled = text(baseline, "peeled_commit_sha").unwrap_or_default();
    let root_sha = text(branch_root, "commit_sha").unwrap_or_default();
    for (label, value) in [
        ("tag_object_sha", tag_object),
        ("peeled_commit_sha", peeled),
        ("branch_root commit_sha", root_sha),
    ] {
        if !full_sha(value) {
            problems.push(format!(
                "baseline {label} must be a full lowercase commit SHA"
            ));
        }
    }
    if tag.is_empty() || git(root, &["cat-file", "-t", tag]).ok().as_deref() != Some("tag") {
        problems.push("B72 must resolve through an annotated tag object".to_owned());
    }
    if !tag.is_empty() && git(root, &["rev-parse", tag]).ok().as_deref() != Some(tag_object) {
        problems.push("B72 tag object SHA does not match the repository".to_owned());
    }
    let peeled_ref = format!("{tag}^{{}}");
    if !tag.is_empty() && git(root, &["rev-parse", &peeled_ref]).ok().as_deref() != Some(peeled) {
        problems.push("B72 peeled commit SHA does not match the repository".to_owned());
    }
    if !root_sha.is_empty() && git(root, &["rev-parse", root_sha]).ok().as_deref() != Some(root_sha)
    {
        problems.push("R73 branch root is absent from the repository".to_owned());
    }
    if text(instrumented, "state") != Some("frozen")
        || text(instrumented, "source_sha") != Some("e757556d3a31d565f52a9561d6d4e555bb1cc373")
        || text(instrumented, "binary_sha256").is_none_or(|digest| !sha256(digest))
        || text(instrumented, "host_fingerprint")
            .is_none_or(|digest| !digest.strip_prefix("sha256:").is_some_and(sha256))
        || text(instrumented, "freeze_evidence") != Some(BASELINE_PILOT_V2_FREEZE_EVIDENCE)
        || integer_array(instrumented.get("stable_rates_per_second"))
            != [2_500, 5_000, 10_000, 20_000]
        || integer(instrumented, "selected_knee_rate_per_second") != Some(20_000)
        || integer_array(instrumented.get("frozen_d3_rates_per_second")) != [5_000, 12_000, 17_000]
    {
        problems.push("I73 identity must retain the admitted exact-source freeze".to_owned());
    }
    Ok(problems)
}

pub fn check_post_tag_delta(
    root: &Path,
    value: &TomlValue,
    release: &str,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut problems = Vec::new();
    if integer(value, "schema_version") != Some(1) || text(value, "release") != Some(release) {
        problems.push("post-tag delta schema/release mismatch".to_owned());
        return Ok(problems);
    }
    let from = text(value, "from").unwrap_or_default();
    let to = text(value, "to").unwrap_or_default();
    let allowed: BTreeSet<_> = string_array(value.get("allowed_classifications"))
        .into_iter()
        .collect();
    let mut declared = BTreeMap::new();
    for item in value
        .get("paths")
        .and_then(TomlValue::as_array)
        .into_iter()
        .flatten()
    {
        let path = text(item, "path").unwrap_or_default();
        let status = text(item, "status").unwrap_or_default();
        let classification = text(item, "classification").unwrap_or_default();
        if path.is_empty() || !allowed.contains(classification) {
            problems.push(format!(
                "post-tag delta has invalid path/classification for {path}"
            ));
            continue;
        }
        if declared
            .insert(
                path.to_owned(),
                (status.to_owned(), classification.to_owned()),
            )
            .is_some()
        {
            problems.push(format!("post-tag delta declares {path} more than once"));
        }
    }
    let range = format!("{from}..{to}");
    let observed_text = git(root, &["diff", "--name-status", &range])?;
    let observed: BTreeMap<_, _> = observed_text
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            Some((fields.next()?.to_owned(), fields.next()?.to_owned()))
        })
        .map(|(status, path)| (path, status))
        .collect();
    for (path, status) in &observed {
        match declared.get(path) {
            Some((declared_status, _)) if declared_status == status => {}
            Some((declared_status, _)) => problems.push(format!(
                "post-tag delta status mismatch for {path}: declared {declared_status}, observed {status}"
            )),
            None => problems.push(format!("post-tag delta leaves {path} unclassified")),
        }
    }
    for path in declared.keys() {
        if !observed.contains_key(path) {
            problems.push(format!("post-tag delta declares stale path {path}"));
        }
    }
    let runtime: BTreeSet<_> = string_array(value.get("runtime_paths"))
        .into_iter()
        .collect();
    let instrumentation: BTreeSet<_> = string_array(value.get("instrumentation_paths"))
        .into_iter()
        .collect();
    let declared_runtime: BTreeSet<_> = declared
        .iter()
        .filter(|(_, (_, class))| class == "runtime")
        .map(|(path, _)| path.as_str())
        .collect();
    let declared_instrumentation: BTreeSet<_> = declared
        .iter()
        .filter(|(_, (_, class))| class == "instrumentation")
        .map(|(path, _)| path.as_str())
        .collect();
    if runtime != declared_runtime || instrumentation != declared_instrumentation {
        problems.push(
            "post-tag runtime/instrumentation summaries do not match path classifications"
                .to_owned(),
        );
    }
    Ok(problems)
}

fn git(root: &Path, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

pub fn check_receipt(value: &JsonValue, contract: &TomlValue) -> Vec<String> {
    let mut problems = Vec::new();
    for (pointer, expected) in [
        ("/release", RELEASE),
        ("/profile_id", PROFILE),
        ("/environment_class", ENVIRONMENT_CLASS),
    ] {
        if value.pointer(pointer).and_then(JsonValue::as_str) != Some(expected) {
            problems.push(format!("receipt {pointer} must be {expected}"));
        }
    }
    if value.get("promotable").and_then(JsonValue::as_bool) != Some(false) {
        problems.push("local receipt must never be promotable".to_owned());
    }
    if value
        .get("numerical_claim_eligible")
        .and_then(JsonValue::as_bool)
        != Some(false)
    {
        problems.push("local receipt must not be numerical-claim eligible".to_owned());
    }
    if value
        .get("non_promotion_reason")
        .and_then(JsonValue::as_str)
        .is_none_or(str::is_empty)
    {
        problems.push("local receipt requires a non_promotion_reason".to_owned());
    }
    if value
        .get("source_sha")
        .and_then(JsonValue::as_str)
        .is_none_or(|value| !full_sha(value))
    {
        problems.push("local receipt source_sha must be a full lowercase commit SHA".to_owned());
    }
    for field in [
        "binary_sha256",
        "scenario_sha256",
        "host_fingerprint_sha256",
        "raw_series_sha256",
    ] {
        if value
            .get(field)
            .and_then(JsonValue::as_str)
            .is_none_or(|value| !sha256(value))
        {
            problems.push(format!("local receipt {field} must be lowercase SHA-256"));
        }
    }
    let modes = string_array(contract.get("allowed_instrumentation_modes"));
    if value
        .get("instrumentation_mode")
        .and_then(JsonValue::as_str)
        .is_none_or(|mode| !modes.contains(&mode))
    {
        problems.push("local receipt has an unsupported instrumentation_mode".to_owned());
    }
    if value.get("pair_index").and_then(JsonValue::as_u64) == Some(0) {
        problems.push("local receipt pair_index must be positive".to_owned());
    }
    let attempted = value
        .pointer("/outcomes/attempted")
        .and_then(JsonValue::as_u64);
    let accounted = ["success", "rejected", "timeout", "late", "incomplete"]
        .into_iter()
        .map(|field| {
            value
                .pointer(&format!("/outcomes/{field}"))
                .and_then(JsonValue::as_u64)
        })
        .try_fold(0_u64, |total, count| {
            count.map(|count| total.saturating_add(count))
        });
    if attempted.is_none() || attempted != accounted {
        problems
            .push("local receipt outcomes must account for every attempted operation".to_owned());
    }
    problems
}

fn check_schema(schema: &JsonValue, value: &JsonValue, label: &str) -> Vec<String> {
    let validator = match jsonschema::validator_for(schema) {
        Ok(validator) => validator,
        Err(error) => return vec![format!("cannot compile local receipt schema: {error}")],
    };
    validator
        .iter_errors(value)
        .map(|error| format!("{label} schema violation: {error}"))
        .collect()
}

fn full_sha(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn text<'a>(root: &'a TomlValue, field: &str) -> Option<&'a str> {
    root.get(field).and_then(TomlValue::as_str)
}

fn integer(root: &TomlValue, field: &str) -> Option<i64> {
    root.get(field).and_then(TomlValue::as_integer)
}

fn float(root: &TomlValue, field: &str) -> Option<f64> {
    root.get(field).and_then(TomlValue::as_float)
}

fn boolean(root: &TomlValue, field: &str) -> Option<bool> {
    root.get(field).and_then(TomlValue::as_bool)
}

fn string_array(value: Option<&TomlValue>) -> Vec<&str> {
    value
        .and_then(TomlValue::as_array)
        .into_iter()
        .flatten()
        .filter_map(TomlValue::as_str)
        .collect()
}

fn integer_array(value: Option<&TomlValue>) -> Vec<i64> {
    value
        .and_then(TomlValue::as_array)
        .into_iter()
        .flatten()
        .filter_map(TomlValue::as_integer)
        .collect()
}

fn float_array(value: Option<&TomlValue>) -> Vec<f64> {
    value
        .and_then(TomlValue::as_array)
        .into_iter()
        .flatten()
        .filter_map(|value| {
            value
                .as_float()
                .or_else(|| value.as_integer().map(|v| v as f64))
        })
        .collect()
}

fn finish(label: &str, release: &str, problems: Vec<String>) -> Result<(), Box<dyn Error>> {
    if problems.is_empty() {
        println!("{label}: OK (release {release}, local screening only)");
        Ok(())
    } else {
        Err(format!("{label} failed:\n- {}", problems.join("\n- ")).into())
    }
}

struct Options {
    root: PathBuf,
    release: String,
    receipt: Option<PathBuf>,
    require_ship: bool,
}

impl Options {
    fn parse(args: Vec<String>) -> Result<Self, Box<dyn Error>> {
        let mut root = crate::doc_check::find_repo_root()?;
        let mut release = None;
        let mut receipt = None;
        let mut require_ship = false;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--root" => root = PathBuf::from(args.next().ok_or("--root requires a path")?),
                "--release" => release = Some(args.next().ok_or("--release requires a value")?),
                "--receipt" => {
                    receipt = Some(PathBuf::from(
                        args.next().ok_or("--receipt requires a path")?,
                    ))
                }
                "--require-ship" => require_ship = true,
                other => {
                    return Err(
                        format!("unsupported performance contract argument: {other}").into(),
                    )
                }
            }
        }
        Ok(Self {
            root,
            release: release.ok_or("performance-contract-check requires --release")?,
            receipt,
            require_ship,
        })
    }
}
