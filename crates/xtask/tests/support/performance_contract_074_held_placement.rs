//! Exact additive data-only boundary; no original live predicate is normalized.
#[path = "performance_contract_074_owned_placement.rs"]
mod owned_placement;
const BASE: &str = "b23a503b";
const PREFIX: &str = "tools/long-run-supervisor-074/src/";
const HOOKS: &[(&str, &str)] = &[
    (
        "diagnostic_two_phase_launch.rs",
        "\n#[path = \"diagnostic_held_placement.rs\"]\npub mod held_placement;\n",
    ),
    (
        "diagnostic_two_phase_launch_tests.rs",
        "\n#[path = \"diagnostic_held_placement_tests.rs\"]\nmod held_placement;\n",
    ),
];
const COMPAT: &str = r##"
## Signed expected held placement v3

The additive diagnostic-held-placement-requirements-074-v3,
diagnostic-held-placement-074-v3 and diagnostic-signed-held-placement-074-v3
schemas bind complete historical v2 launch claims and the exact lowercase hex
encoding of the 104-byte HCRG0742 Enroll packet. Its attempt digest must equal
the unchanged hc074-original-root-attempt-v1 serde JSON identity tuple digest.
Armed, Refused and Revoking packets are not placement requests. No v1/v2 reader,
signature, packet or requirement changes; no upgrade or fallback exists.

Signing bytes are hydracache-diagnostic-held-placement-074-v3, one NUL byte and
the canonical newline-terminated body. The body action is project-held-cell-one.
The requirements digest covers canonical newline-terminated requirements JSON;
its launch_signing_message_sha256 covers the complete existing v2 signing
message, including its domain and NUL. Bodies/envelopes retain 16384/32768-byte
limits. Unknown/duplicate fields, noncanonical JSON, wrong domains/keys, changed
bindings and non-lowercase hex refuse. Inspection uses only a supplied key.

Expected placement derives system-hc074 followed by the full attempt digest and
.slice directly beneath system.slice, with the existing fixed cell-one unit as
its child. No caller supplies arbitrary paths, commands or a renewed deadline.
These are data-only expectations, not a live root, trusted issuer, held unit,
systemd request or original-owner capability. Actual placement, complete live
frame, issuer enrollment, barrier, writer cessation and completion stay separate.
All six runtime admission fields and renewal are false. No send/start, host
creation, completion or fence release follows from this new signature.
"##;
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let mut text = owned_placement::normalize(path, raw);
    for (file, hook) in HOOKS {
        if path == format!("{PREFIX}{file}") {
            assert_eq!(
                text.matches(hook).count(),
                1,
                "exact placement hook: {path}"
            );
            text = text.replacen(hook, "", 1);
        }
    }
    if path == "docs/COMPAT.md" {
        assert_eq!(
            text.matches(COMPAT).count(),
            1,
            "exact placement wire scope"
        );
        text = text.replacen(COMPAT, "", 1);
    }
    text
}
fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &str) -> String {
    crate::runtime_io::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "")
}
fn git(args: &[&str]) -> String {
    let value = std::process::Command::new("git")
        .args(args)
        .current_dir(root())
        .output()
        .unwrap();
    assert!(value.status.success());
    String::from_utf8(value.stdout).unwrap().replace('\r', "")
}
#[test]
fn held_placement_preserves_all_423_sources_and_external_fault_harness() {
    let files = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        BASE,
        "--",
        "tools/long-run-supervisor-074/src",
    ]);
    assert_eq!(files.lines().count(), 423);
    for path in files.lines().chain([
        "docs/COMPAT.md",
        "docs/testing/gated-test-registry.toml",
        "tools/long-run-supervisor-074/tests/owned_root_creator_sigkill.py",
    ]) {
        assert_eq!(
            normalize(path, &read(path)),
            git(&["show", &format!("{BASE}:{path}")]),
            "{path}"
        );
    }
}
#[test]
fn held_placement_adapter_rejects_missing_duplicate_and_unrelated_drift() {
    for (file, hook) in HOOKS
        .iter()
        .map(|(file, hook)| (format!("{PREFIX}{file}"), *hook))
        .chain([("docs/COMPAT.md".into(), COMPAT)])
    {
        let raw = read(&file);
        for bad in [raw.replacen(hook, "", 1), format!("{raw}{hook}")] {
            assert!(std::panic::catch_unwind(|| normalize(&file, &bad)).is_err());
        }
        let drift = raw.replacen("\n", "\n// unrelated drift\n", 1);
        assert_ne!(
            normalize(&file, &drift),
            git(&["show", &format!("{BASE}:{file}")])
        );
    }
}
#[test]
fn held_placement_is_bounded_versioned_and_has_no_runtime_capability() {
    let source = read(&format!("{PREFIX}diagnostic_held_placement.rs"));
    for required in [
        "diagnostic-held-placement-requirements-074-v3",
        "diagnostic-held-placement-074-v3",
        "diagnostic-signed-held-placement-074-v3",
        "hydracache-diagnostic-held-placement-074-v3",
        "hc074-original-root-attempt-v1",
        "root.attempt_sha256 != attempt",
        "packet.kind != Kind::Enroll",
        "MAX_BODY_BYTES: usize = 16384",
        "MAX_ENVELOPE_BYTES: usize = 32768",
        "line(&doc)? != raw",
        "verify_strict",
        "deny_unknown_fields",
        "system-hc074{}.slice",
        "\"renewal\": false",
        "\"original_send_allowed\": false",
        "\"fence_release_allowed\": false",
    ] {
        assert!(source.contains(required), "{required}");
    }
    for forbidden in [
        "unsafe",
        "File::",
        "Command::",
        "clock_gettime",
        "thread_local!",
        "static ",
        "sendmsg",
        "StartTransientUnit",
        "LOCK_UN",
        "TransientUnitSpec",
        "pub fn start",
        "pub fn release",
        "pub fn enroll",
        "impl PlacementBody",
    ] {
        assert!(!source.contains(forbidden), "{forbidden}");
    }
    let tests = read(&format!("{PREFIX}diagnostic_held_placement_tests.rs"));
    for required in [
        "0x074e_5a1c",
        "0..128",
        "0..PACKET_BYTES",
        "Kind::Armed",
        "Kind::Revoking",
        "Kind::Refused",
    ] {
        assert!(tests.contains(required), "{required}");
    }
}
