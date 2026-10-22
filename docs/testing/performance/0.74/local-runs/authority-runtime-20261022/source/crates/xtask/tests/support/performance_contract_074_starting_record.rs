//! One byte-only artifact; original storage and runtime predicates stay exact.
const REGISTRATION: &str =
    "#[path = \"diagnostic_collector_starting_record.rs\"]\npub mod starting_record;\n";
const GATE: &str =
    "\n#[path = \"support/performance_contract_074_starting_record.rs\"]\nmod starting_record;\n";
const ADAPTER: &str = "    let current = super::starting_record::normalize(path, raw);\n    let raw = current.as_str();\n";
const COMPAT_ROW: &str = "| Separate 0.74 collector Starting byte record | `diagnostic-collector-starting-074-v1` | opt-in Linux byte encoder only; no publisher or installed consumer | Canonical LF JSON, at most 16384 bytes; eventual fixed name `diagnostic-collector-starting-074.json`. Binds exact controller-attempt/collector-Reserved LF digests, canonical signed-consent LF digest and the identical signed Starting state; reuses strict prior-history and adjacent-transition grammar. | Unknown/future/duplicate/partial/noncanonical/oversized/mixed bindings refuse. Byte consistency does not verify signatures, inspect actual clocks/credentials or recover acknowledgement, replay ownership or start rights. Old two-file storage and legacy readers remain unchanged and reject a third record. No migration, overwrite, repair, publication, output, execution or cleanup authority. |\n";
const OLD_ASSERT: &str = "        assert_eq!(\n            read(path),\n            old(path),\n            \"original leaf/workload changed: {path}\"\n        );\n";
const NEW_ASSERT: &str = "        assert_eq!(\n            super::starting_record::normalize(path, &read(path)),\n            old(path),\n            \"original leaf/workload changed: {path}\"\n        );\n";
const OLD_ORIGINAL_ASSERT: &str = "        assert_eq!(read(p), old(p), \"out-of-scope: {p}\");\n";
const NEW_ORIGINAL_ASSERT: &str = "        assert_eq!(\n            super::starting_record::normalize(p, &read(p)),\n            old(p),\n            \"out-of-scope: {p}\"\n        );\n";
const OLD_BUDGET_READ: &str = "    let path = \"crates/xtask/tests/support/performance_contract_074_original_start_consent.rs\";\n    let raw = read(path);\n";
const NEW_BUDGET_READ: &str = "    let path = \"crates/xtask/tests/support/performance_contract_074_original_start_consent.rs\";\n    let raw = super::starting_record::normalize(path, &read(path));\n";
const REGISTRY: &str = "\n# BEGIN COLLECTOR STARTING RECORD GATE\n[[gate]]\nid = \"cfg.hydracache-long-run-supervisor-074.diagnostic-collector-starting-record\"\nkind = \"cfg_test_target\"\nsource = \"tools/long-run-supervisor-074/tests/diagnostic_collector_starting_record.rs\"\npackage = \"hydracache-long-run-supervisor-074\"\ntarget = \"diagnostic_collector_starting_record\"\ntest = \"\"\ncfg = 'target_os = \"linux\"'\nenv = \"\"\nreason = \"Linux byte-only Starting evidence, no publication, recovered authority or execution\"\ntier = \"fast\"\nrequired_features = []\nrequired_env = []\nrequired_tools = [\"cargo\"]\ntimeout_seconds = 120\nowner_release = \"0.74.0\"\nship_mandatory = false\nartifacts = []\n\n[gate.ci]\nworkflow = \".github/workflows/ci.yml\"\njob = \"rust\"\nstep = \"Release 0.74 Linux supervisor contracts\"\n\n[gate.command]\nprogram = \"cargo\"\nargs = [\"test\", \"-p\", \"hydracache-long-run-supervisor-074\", \"--test\", \"diagnostic_collector_starting_record\", \"--locked\"]\ncwd = \".\"\nplatform = \"linux\"\n\n[gate.command.env]\n# END COLLECTOR STARTING RECORD GATE\n";
fn once(raw: &str, new: &str, old: &str, path: &str) -> String {
    assert_eq!(
        raw.matches(new).count(),
        1,
        "unapproved Starting record adapter: {path}"
    );
    raw.replacen(new, old, 1)
}
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let current = super::preconsent_frame::normalize(path, raw);
    let raw = current.as_str();
    match path {
        "tools/long-run-supervisor-074/src/diagnostic_collector_reserved.rs" => {
            once(raw, REGISTRATION, "", path)
        }
        "docs/COMPAT.md" => once(raw, COMPAT_ROW, "", path),
        "docs/testing/gated-test-registry.toml" => once(raw, REGISTRY, "", path),
        "crates/xtask/tests/performance_contract_074.rs" => once(raw, GATE, "", path),
        "crates/xtask/tests/support/performance_contract_074_original_start_consent.rs" => {
            once(raw, NEW_ORIGINAL_ASSERT, OLD_ORIGINAL_ASSERT, path)
        }
        "crates/xtask/tests/support/performance_contract_074_consent_budget.rs" => {
            once(raw, NEW_BUDGET_READ, OLD_BUDGET_READ, path)
        }
        "crates/xtask/tests/support/performance_contract_074_validation_frame.rs" => {
            let raw = once(raw, ADAPTER, "", path);
            once(&raw, NEW_ASSERT, OLD_ASSERT, path)
        }
        _ => raw.into(),
    }
}
#[test]
fn collector_starting_record_preserves_original_runtime_and_two_file_storage() {
    let read = |p: &str| {
        crate::runtime_io::read_to_string(super::root().join(p))
            .unwrap()
            .replace('\r', "")
    };
    let old = |p: &str| {
        let out = std::process::Command::new("git")
            .args(["show", &format!("7b799270:{p}")])
            .current_dir(super::root())
            .output()
            .unwrap();
        assert!(out.status.success());
        String::from_utf8(out.stdout).unwrap().replace('\r', "")
    };
    for p in [
        "tools/long-run-supervisor-074/src/diagnostic_collector_reserved.rs",
        "docs/COMPAT.md",
        "crates/xtask/tests/performance_contract_074.rs",
        "crates/xtask/tests/support/performance_contract_074_validation_frame.rs",
        "crates/xtask/tests/support/performance_contract_074_original_start_consent.rs",
        "crates/xtask/tests/support/performance_contract_074_consent_budget.rs",
    ] {
        let raw = read(p);
        assert_eq!(
            normalize(p, &raw),
            old(p),
            "original predicate changed: {p}"
        );
        assert_ne!(normalize(p, &format!("{raw}\n// unapproved\n")), old(p));
    }
    let p = "tools/long-run-supervisor-074/src/diagnostic_collector_reserved.rs";
    assert!(std::panic::catch_unwind(|| normalize(
        p,
        &read(p).replace("pub mod starting_record;", "pub mod unchecked;")
    ))
    .is_err());
    for p in [
        "Cargo.toml",
        "Cargo.lock",
        "tools/long-run-supervisor-074/Cargo.toml",
        "tools/long-run-supervisor-074/src/lib.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_reservation_storage.rs",
        "tools/long-run-supervisor-074/src/diagnostic_controller_request_attempt.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_causal.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_progress.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_start_consent.rs",
        "tools/long-run-supervisor-074/src/diagnostic_validation_frame.rs",
        "tools/long-run-supervisor-074/src/diagnostic_validation_frame_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_validation_frame_budget_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_start_consent.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_start_consent.rs",
        "tools/long-run-supervisor-074/src/diagnostic_worker_start_consent.rs",
        "tools/long-run-supervisor-074/src/diagnostic_ipc.rs",
        "tools/long-run-supervisor-074/src/diagnostic_lease.rs",
        "tools/long-run-supervisor-074/src/diagnostic_transition.rs",
        "tools/long-run-supervisor-074/src/host_execution.rs",
        "tools/long-run-supervisor-074/src/server.rs",
        "docs/testing/gated-test-registry.toml",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ] {
        assert_eq!(
            normalize(p, &read(p)),
            old(p),
            "byte codec changed runtime/input: {p}"
        );
    }
    let source = read("tools/long-run-supervisor-074/src/diagnostic_collector_starting_record.rs");
    let runtime = source
        .lines()
        .filter(|l| !l.trim_start().starts_with("///") && !l.trim_start().starts_with("//!"))
        .collect::<Vec<_>>()
        .join("\n");
    for denied in [
        "std::fs",
        "std::process",
        "KernelPacket",
        "SystemTime",
        "SigningKey",
        "FixedAuthority",
        "pub fn publish",
        "pub fn recover",
        "pub fn start_once",
        "pub fn into_",
    ] {
        assert!(!runtime.contains(denied), "{denied}");
    }
    for required in [
        "validate_retained_collector_reserved",
        "validate_transition",
        "deny_unknown_fields",
        "MAX_ENVELOPE_BYTES",
        "16_384",
        "consent_line(&signed)? != consent_raw",
        "line(&record)? != raw",
        "canonical_json(&body.starting_state)",
        "signed_consent_sha256",
    ] {
        assert!(runtime.contains(required), "{required}");
    }
    let path = "tools/long-run-supervisor-074/src/diagnostic_collector_reservation_storage.rs";
    let storage = super::starting_publication::normalize(path, &read(path));
    assert!(storage.contains("let mut seen = [false; 2]"));
    assert!(!storage.contains("diagnostic-collector-starting-074"));
}
