//! The new fork-ordering proof changes no existing runtime or raw child body.
const GATES: &str = r###"
# BEGIN ORIGINAL BARRIER VALIDATION GATES
[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-barrier-screen"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_descriptor_barrier_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_barrier_validation_screen"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Full original validation behind unchanged test-only holder barrier; no production execution"
tier = "manual"
required_features = []
required_env = ["HC074_START_MATERIAL_BUNDLE"]
required_tools = ["cargo"]
timeout_seconds = 180
owner_release = "0.74.0"
ship_mandatory = false
artifacts = []

[gate.ci]
workflow = ".github/workflows/ci.yml"
job = "gated-proof-registry"
step = "Run registered gated proofs"

[gate.command]
program = "cargo"
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::holder::barrier::original_barrier_validation_screen","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-barrier-healthy"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_descriptor_barrier_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_barrier_validation_healthy"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Full original validation behind unchanged test-only holder barrier; no production execution"
tier = "manual"
required_features = []
required_env = ["HC074_START_MATERIAL_BUNDLE"]
required_tools = ["cargo"]
timeout_seconds = 900
owner_release = "0.74.0"
ship_mandatory = false
artifacts = []

[gate.ci]
workflow = ".github/workflows/ci.yml"
job = "gated-proof-registry"
step = "Run registered gated proofs"

[gate.command]
program = "cargo"
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::holder::barrier::original_barrier_validation_healthy","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-barrier-refusals"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_descriptor_barrier_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_barrier_validation_refusals"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Full original validation behind unchanged test-only holder barrier; no production execution"
tier = "manual"
required_features = []
required_env = ["HC074_START_MATERIAL_BUNDLE"]
required_tools = ["cargo"]
timeout_seconds = 1200
owner_release = "0.74.0"
ship_mandatory = false
artifacts = []

[gate.ci]
workflow = ".github/workflows/ci.yml"
job = "gated-proof-registry"
step = "Run registered gated proofs"

[gate.command]
program = "cargo"
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::holder::barrier::original_barrier_validation_refusals","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-barrier-sigkill"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_descriptor_barrier_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_barrier_validation_sigkill"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Full original validation behind unchanged test-only holder barrier; no production execution"
tier = "manual"
required_features = []
required_env = ["HC074_START_MATERIAL_BUNDLE"]
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::holder::barrier::original_barrier_validation_sigkill","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

# END ORIGINAL BARRIER VALIDATION GATES
"###;
const SELF_REG: &str =
    "#[path = \"performance_contract_074_original_barrier.rs\"]\nmod original_barrier;\n";
const ADAPTER: &str =
    "    let current = original_barrier::normalize(path, raw);\n    let raw = current.as_str();\n";
const TEST_REG: &str =
    "#[path = \"diagnostic_original_descriptor_barrier_tests.rs\"]\nmod barrier;\n";
fn once(raw: &str, from: &str) -> String {
    assert_eq!(raw.matches(from).count(), 1, "barrier exception drift");
    raw.replacen(from, "", 1)
}
pub(super) fn normalize(path: &str, raw: &str) -> String {
    match path {
        "tools/long-run-supervisor-074/src/diagnostic_original_descriptor_plan_tests.rs" => {
            once(raw, "pub(super) use original_owner;\n")
        }
        "tools/long-run-supervisor-074/src/diagnostic_original_descriptor_holder_tests.rs" => {
            once(raw, TEST_REG)
        }
        "crates/xtask/tests/support/performance_contract_074_original_descriptors.rs" => {
            once(&once(raw, SELF_REG), ADAPTER)
        }
        "docs/testing/gated-test-registry.toml" => once(raw, GATES),
        _ => raw.to_owned(),
    }
}
fn read(path: &str) -> String {
    std::fs::read_to_string(super::root().join(path))
        .unwrap()
        .replace('\r', "")
}
fn frozen(path: &str) -> String {
    let out = std::process::Command::new("git")
        .args(["show", &format!("3e4be719:{path}")])
        .current_dir(super::root())
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap().replace('\r', "")
}
#[test]
fn original_barrier_freezes_all_original_runtime_and_existing_holder_bodies() {
    for path in [
        "tools/long-run-supervisor-074/src/diagnostic_original_descriptor_holder_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_descriptor_plan_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_descriptor_plan.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_custody_join.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_collector_custody_storage.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_grouped_preparation.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_collector_sinks.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_collector_pipes.rs",
        "tools/long-run-supervisor-074/src/diagnostic_autonomous_watchdog.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_fence.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs",
        "tools/long-run-supervisor-074/src/diagnostic_lease.rs",
        "docs/testing/performance/0.74/qualification-manifest.toml",
        "crates/xtask/tests/support/performance_contract_074_original_descriptors.rs",
        "docs/testing/gated-test-registry.toml",
    ] {
        assert_eq!(normalize(path, &read(path)), frozen(path), "{path}");
    }
    let path = "tools/long-run-supervisor-074/src/diagnostic_original_descriptor_holder_tests.rs";
    let raw = read(path);
    for (from, to) in [
        ("byte != b'C'", "false"),
        ("n != 0", "false"),
        ("libc::SYS_fork", "libc::SYS_vfork"),
        ("libc::fsync(*fd)", "0"),
    ] {
        assert_ne!(normalize(path, &raw.replace(from, to)), frozen(path));
    }
    assert!(std::panic::catch_unwind(|| normalize(path, &raw.replace(TEST_REG, ""))).is_err());
    assert_ne!(
        normalize(path, &format!("{raw}\nfn bypass() {{}}\n")),
        frozen(path)
    );
}
fn ordered(raw: &str, needles: &[&str]) -> bool {
    let mut rest = raw;
    for needle in needles {
        let Some(at) = rest.find(needle) else {
            return false;
        };
        rest = &rest[at + needle.len()..];
    }
    true
}
fn healthy_order(raw: &str) -> bool {
    ordered(
        raw,
        &[
            "Holder::spawn(",
            "barred(&child);",
            "plan.revalidate_for_test().unwrap();",
            "call_count(&report, Stage::Checks);",
            "barred(&child);",
            "drop(held);",
            "aliases_closed(fds);",
            "busy(root);",
            "child.observe_eof(false)",
            "busy(root);",
            "child.finish(false);",
        ],
    )
}
#[test]
fn original_barrier_requires_full_frame_before_parent_close_and_eof_command() {
    let raw =
        read("tools/long-run-supervisor-074/src/diagnostic_original_descriptor_barrier_tests.rs");
    let healthy = raw
        .split("fn healthy(")
        .nth(1)
        .unwrap()
        .split("#[test]")
        .next()
        .unwrap();
    assert!(healthy_order(healthy));
    for needle in [
        "plan.revalidate_for_test().unwrap();",
        "barred(&child);",
        "aliases_closed(fds);",
        "child.observe_eof(false)",
    ] {
        assert!(!healthy_order(&healthy.replace(needle, "")), "{needle}");
    }
    assert!(raw.contains("assert_eq!(&after[..4], &origins[..4]);"));
    assert!(raw.contains("after[4] >= origins[4] && after[5] >= after[4]"));
    assert!(raw.contains("healthy(3, true)"));
    assert!(raw.contains("0x0740_b018_u64"));
    for denied in [
        "Command::",
        "SYS_fork",
        "timerfd_create",
        "thread::sleep",
        "pub fn",
        "Connection::",
        "libc::write",
    ] {
        assert!(!raw.contains(denied), "{denied}");
    }
}
#[test]
fn original_barrier_refusals_never_issue_eof_and_real_cuts_remain_distinct() {
    let raw =
        read("tools/long-run-supervisor-074/src/diagnostic_original_descriptor_barrier_tests.rs");
    let refusals = raw
        .split("fn original_barrier_validation_refusals()")
        .nth(1)
        .unwrap()
        .split("#[test]")
        .next()
        .unwrap();
    assert!(!refusals.contains("observe_eof("));
    assert!(ordered(
        refusals,
        &[
            "Holder::spawn(",
            "match case",
            "plan.revalidate_for_test().unwrap_err()",
            "OriginalCustodyError::Refused",
            "barred(&child);",
            "drop(held);",
            "barred(&child);",
            "busy(root);",
            "child.finish(true);",
            "retained_context_refuses_for_test(root)"
        ]
    ));
    for case in [
        "trust",
        "output",
        "writer",
        "message",
        "alarm",
        "custody-flags",
        "custody-object",
        "sink-offset",
        "final-expiry",
    ] {
        assert!(refusals.contains(&format!("\"{case}\"")), "{case}");
    }
    assert!(refusals.contains("!reason.contains(\"Time\")"));
    assert!(refusals.contains("Original(Expired)"));
    assert!(raw.contains("repeat in 1..=3"));
    let compact = raw.split_whitespace().collect::<String>();
    assert!(compact.contains(
        "forcutin[\"before-frame\",\"after-frame\",\"after-alias-close\",\"after-eof\",]"
    ));
    assert!(raw.contains("child.finish(true);\n                    assert_eq!(readable(child.pidfd.as_ref().unwrap(), 0), 1);"));
    assert!(raw.contains("0x0740_b118_u64"));
    assert!(raw.contains("0x0740_b218_u64"));
}
