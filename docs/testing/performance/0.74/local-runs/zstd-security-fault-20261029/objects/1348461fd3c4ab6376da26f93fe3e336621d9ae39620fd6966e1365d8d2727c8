//! Additive original collector lifecycle; old attempt/storage predicates stay exact.
use std::process::Command;
pub(super) const REGISTRY_ADDITION: &str = r#"
# BEGIN COLLECTOR RESERVED CRASH GATE
[[gate]]
id = "tool.hydracache-long-run-supervisor-074.collector-reserved-process-crash-all-stages-and-lost-ack"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_collector_reserved_crash_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "collector_reserved_process_crash_all_stages_and_lost_ack"
cfg = 'target_os = "linux"'
env = ""
reason = "Manual nonroot original collector Reserved SIGKILL and lost-ack proof; no execution or enrollment"
tier = "manual"
required_features = []
required_env = []
required_tools = ["cargo"]
timeout_seconds = 1800
owner_release = "0.74.0"
ship_mandatory = false
artifacts = []

[gate.ci]
workflow = ".github/workflows/ci.yml"
job = "gated-proof-registry"
step = "Run registered gated proofs"

[gate.command]
program = "cargo"
args = ["test", "-p", "hydracache-long-run-supervisor-074", "--lib", "--locked", "diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::crash_tests::collector_reserved_process_crash_all_stages_and_lost_ack", "--", "--ignored", "--exact", "--nocapture", "--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]
# END COLLECTOR RESERVED CRASH GATE
"#;

#[test]
fn collector_reserved_requires_original_owned_publication_not_recovered_start() {
    let c = super::contract("diagnostic-collector-reserved-contract.toml");
    for (field, expected) in [
        ("max_record_bytes", 16384),
        ("max_collector_bytes", 8192),
        ("kernel_packet_hold_ns", 60_000_000_000),
        ("write_stages", 9),
        ("sigkills_per_repeat", 10),
        ("controls_per_repeat", 1),
        ("fresh_inspections_per_repeat", 22),
        ("crash_repeats", 3),
    ] {
        assert_eq!(c[field].as_integer(), Some(expected), "{field}");
    }
    for field in [
        "observed_owner_consumed_with_original_authority_and_fence",
        "exact_original_request_and_new_record_inventory",
        "legacy_single_document_inspector_unchanged",
        "paired_byte_inspector_is_not_authentication",
        "live_original_signature_and_kernel_checks_at_each_boundary",
        "file_and_directory_sync_before_runtime_ack",
        "sticky_failure_retains_owner_and_context",
        "before_create_retains_original_request",
        "no_recovered_acknowledgement_or_replay",
        "initial_collector_reserved_publication",
    ] {
        assert_eq!(c[field].as_bool(), Some(true), "{field}");
    }
    for field in [
        "starting_or_later_cells",
        "root_outputs_or_authenticated_start",
        "watchdog_or_all_writer_cessation",
        "power_loss_or_reboot_proof",
        "production_enrollment_or_server_admission",
        "qualification_or_performance_claim",
    ] {
        assert_eq!(c[field].as_bool(), Some(false), "{field}");
    }
    let read = |p: &str| {
        let raw = crate::runtime_io::read_to_string(super::root().join(p))
            .unwrap()
            .replace('\r', "");
        super::original_start_consent::normalize(p, &raw)
    };
    let historical = |p: &str| {
        let o = Command::new("git")
            .args([
                "show",
                &format!("13dac632d91bb029821b9f3a2678e13694222130:{p}"),
            ])
            .current_dir(super::root())
            .output()
            .unwrap();
        assert!(o.status.success());
        String::from_utf8(o.stdout).unwrap().replace('\r', "")
    };
    let path = "tools/long-run-supervisor-074/src/diagnostic_early_attempt_storage.rs";
    let registration = "#[path = \"diagnostic_collector_reservation_storage.rs\"]\npub(crate) mod collector_reservation_storage;\n";
    let old = read(path);
    assert_eq!(old.matches(registration).count(), 1);
    assert_eq!(
        old.replacen(registration, "", 1),
        historical(path),
        "legacy storage bodies must not change"
    );
    for p in [
        "tools/long-run-supervisor-074/src/diagnostic_controller_request_attempt.rs",
        "tools/long-run-supervisor-074/src/diagnostic_reservation_publication_storage.rs",
        "tools/long-run-supervisor-074/src/diagnostic_starting_intent.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector.rs",
        "tools/long-run-supervisor-074/src/diagnostic_controller_attempt_crash_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_collector_inputs.rs",
        "tools/long-run-supervisor-074/src/diagnostic_worker_kernel_binding.rs",
        "Cargo.lock",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ] {
        let current = super::collector_progress::without_registration(p, &read(p));
        assert_eq!(current, historical(p), "out-of-scope change: {p}");
    }
    let registry = read("docs/testing/gated-test-registry.toml");
    let registry = super::collector_causal::without_registry_addition(&registry);
    assert_eq!(registry.matches(REGISTRY_ADDITION).count(), 1);
    assert_eq!(
        registry.replacen(REGISTRY_ADDITION, "", 1),
        historical("docs/testing/gated-test-registry.toml")
    );
    let source = read("tools/long-run-supervisor-074/src/diagnostic_collector_reserved.rs");
    let storage =
        read("tools/long-run-supervisor-074/src/diagnostic_collector_reservation_storage.rs");
    let runtime = source
        .lines()
        .filter(|l| !l.trim_start().starts_with("///"))
        .collect::<Vec<_>>()
        .join("\n");
    for forbidden in [
        "pub fn start",
        "pub fn prepare",
        "pub fn recover",
        "pub fn refresh",
        "pub fn into_",
        "impl Clone",
        "std::process",
        "DiagnosticBackend",
        "FixedStartingIntentRead",
        "FixedAuthorityIntentRead",
        "&mut DiagnosticState",
        "O_TRUNC",
        "remove_file",
        "fs::rename",
    ] {
        assert!(
            !runtime.contains(forbidden) && !storage.contains(forbidden),
            "{forbidden}"
        );
    }
    let compact = runtime.split_whitespace().collect::<String>();
    for marker in [
        "pub fn publish_reserved(",
        "self.check_collector(bytes)",
        "Held::Observed",
        "Held::Publication",
        "CollectorReservationStorage::from_original(self.attempt.storage)",
        "capture_request(original)? != request_raw",
        "original_reservation_time(Some(last))?",
        "verify_original_collector_requirements(",
        "self.original.original.original.latch_refusal()",
        "self.storage.revalidate()?",
        "self.live()?",
        "controller_attempt_sha256 != sha256_hex(request_raw)",
        "state.reserved_monotonic_ns != state.controller_monotonic_ns",
        "body.request_id != request.request_id",
    ] {
        assert!(
            compact.contains(&marker.split_whitespace().collect::<String>()),
            "{marker}"
        );
    }
    for marker in [
        "diagnostic-collector-reserved-074.json",
        "libc::O_EXCL",
        "[true, self.document.is_some()]",
        "self.original.ancestry()?",
        "original.inspect(leaf)?",
        "doc.inspect(leaf)?",
        "owned_write(&leaf, bytes, seal)?",
        "CollectorReservedStage::Acknowledging",
        "self.acknowledged = true",
    ] {
        assert!(storage.contains(marker), "{marker}");
    }
    assert!(
        storage
            .find("self.boundary(CollectorReservedStage::Acknowledging")
            .unwrap()
            < storage.find("self.acknowledged = true").unwrap()
    );
    let crash =
        read("tools/long-run-supervisor-074/src/diagnostic_collector_reserved_crash_tests.rs");
    for marker in [
        "libc::SYS_pidfd_send_signal",
        "libc::SIGKILL",
        "Some(libc::SIGKILL)",
        "for repeat in 1..=3",
        "fresh_inspections=22",
        "sigkills=10",
        "empty=false unchanged=true",
        "snapshot(&s.origin())",
        "snapshot(s.1.path())",
        "inspect_empty_fence_at_for_test(&root.join(\"origin\"), false)",
    ] {
        assert!(crash.contains(marker), "{marker}");
    }
    assert!(read("docs/COMPAT.md").contains("diagnostic-collector-reserved-074-v1"));
    println!("collector_reserved_contract original_fence=true initial_reserved=true starting=false root_start=false");
}
