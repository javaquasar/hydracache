//! Exact original custody/sink join; no live predicate or deadline is normalized.
#[path = "performance_contract_074_original_descriptors.rs"]
mod original_descriptors;
const GATES: &str = r###"
# BEGIN ORIGINAL CUSTODY JOIN GATES
[[gate]]
id = "tool.hydracache-long-run-supervisor-074.custody-join-screen"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_custody_join_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_custody_join_screen"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Original custody/sink full-frame ownership, nonexecuting fixtures only; no production start"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::original_custody_join_screen","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.custody-join-healthy"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_custody_join_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_custody_join_healthy"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Original custody/sink full-frame ownership, nonexecuting fixtures only; no production start"
tier = "manual"
required_features = []
required_env = ["HC074_START_MATERIAL_BUNDLE"]
required_tools = ["cargo"]
timeout_seconds = 600
owner_release = "0.74.0"
ship_mandatory = false
artifacts = []

[gate.ci]
workflow = ".github/workflows/ci.yml"
job = "gated-proof-registry"
step = "Run registered gated proofs"

[gate.command]
program = "cargo"
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::original_custody_join_healthy","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.custody-join-refusals"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_custody_join_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_custody_join_refusals"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Original custody/sink full-frame ownership, nonexecuting fixtures only; no production start"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::original_custody_join_refusals","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.custody-join-later-refusals"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_custody_join_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_custody_join_later_refusals"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Original custody/sink full-frame ownership, nonexecuting fixtures only; no production start"
tier = "manual"
required_features = []
required_env = ["HC074_START_MATERIAL_BUNDLE"]
required_tools = ["cargo"]
timeout_seconds = 600
owner_release = "0.74.0"
ship_mandatory = false
artifacts = []

[gate.ci]
workflow = ".github/workflows/ci.yml"
job = "gated-proof-registry"
step = "Run registered gated proofs"

[gate.command]
program = "cargo"
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::original_custody_join_later_refusals","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.custody-join-process-screen"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_custody_join_process_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_custody_join_process_screen"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Original custody/sink full-frame ownership, nonexecuting fixtures only; no production start"
tier = "manual"
required_features = []
required_env = ["HC074_START_MATERIAL_BUNDLE"]
required_tools = ["cargo"]
timeout_seconds = 600
owner_release = "0.74.0"
ship_mandatory = false
artifacts = []

[gate.ci]
workflow = ".github/workflows/ci.yml"
job = "gated-proof-registry"
step = "Run registered gated proofs"

[gate.command]
program = "cargo"
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::crash_tests::fixed_grouped_process_tests::custody_process_tests::original_custody_join_process_screen","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.custody-join-process-matrix"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_custody_join_process_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_custody_join_process_matrix"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Original custody/sink full-frame ownership, nonexecuting fixtures only; no production start"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::crash_tests::fixed_grouped_process_tests::custody_process_tests::original_custody_join_process_matrix","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]
# END ORIGINAL CUSTODY JOIN GATES
"###;
const GROUP_REG: &str = "#[path = \"diagnostic_original_custody_join.rs\"]\npub mod custody;\n";
const STORAGE_REG: &str = "#[cfg(all(target_os = \"linux\", target_arch = \"x86_64\"))]\n#[path = \"diagnostic_original_collector_custody_storage.rs\"]\npub(crate) mod custody_storage;\n";
const STORAGE_TEST_REG: &str = "    #[cfg(all(target_os = \"linux\", target_arch = \"x86_64\"))]\n    mod custody_tests {\n        include!(\"diagnostic_original_collector_custody_storage_tests.rs\");\n    }\n";
const JOIN_REG: &str =
    "#[path = \"diagnostic_original_custody_join_tests.rs\"]\nmod custody_tests;\n";
const PROCESS_REG: &str =
    "#[path = \"diagnostic_original_custody_join_process_tests.rs\"]\nmod custody_process_tests;\n";
const SELF_REG: &str =
    "#[path = \"performance_contract_074_custody_join.rs\"]\nmod custody_join;\n";
const ADAPTER: &str =
    "    let current = custody_join::normalize(path, raw);\n    let raw = current.as_str();\n";
const BRIDGE: &str = "pub(super) fn restore_join(path: &str, raw: &str) -> String {\n    custody_join::normalize(path, raw)\n}\n";
const CALLER: &str = "            .retain_custody(parent(original.storage)?)";
const OLD_CALLER: &str = "        assert!(!std::fs::read_to_string(file)\n            .unwrap()\n            .contains(\"retain_custody(\"));";
const NEW_CALLER: &str = "        let raw = std::fs::read_to_string(&file).unwrap();\n        assert!(!custody_join::approved_caller(name, &raw).contains(\"retain_custody(\"));";
fn once(raw: &str, new: &str, old: &str) -> String {
    assert_eq!(
        raw.matches(new).count(),
        1,
        "original custody join exception differs"
    );
    raw.replacen(new, old, 1)
}
pub(super) fn approved_caller(name: &str, raw: &str) -> String {
    if name == "diagnostic_original_collector_custody_storage.rs" {
        once(raw, CALLER, "            .approved_original_custody_call()")
    } else {
        raw.to_owned()
    }
}
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let current = original_descriptors::normalize(path, raw);
    let raw = current.as_str();
    match path {
        "tools/long-run-supervisor-074/src/diagnostic_original_grouped_preparation.rs" => {
            once(raw, GROUP_REG, "")
        }
        "tools/long-run-supervisor-074/src/diagnostic_collector_starting_storage.rs" => {
            once(&once(raw, STORAGE_REG, ""), STORAGE_TEST_REG, "")
        }
        "tools/long-run-supervisor-074/src/diagnostic_fixed_grouped_preparation_join_tests.rs" => {
            once(raw, JOIN_REG, "")
        }
        "tools/long-run-supervisor-074/src/diagnostic_fixed_grouped_preparation_crash_tests.rs" => {
            once(raw, PROCESS_REG, "")
        }
        "crates/xtask/tests/support/performance_contract_074_fence_custody.rs" => {
            let raw = once(&once(&once(raw, SELF_REG, ""), ADAPTER, ""), BRIDGE, "");
            once(&raw, NEW_CALLER, OLD_CALLER)
        }
        "crates/xtask/tests/support/performance_contract_074_fixed_grouped.rs" => {
            let raw = once(
                raw,
                "            fence_custody::restore_join(path, &read(path))",
                "            read(path)",
            );
            once(&raw,
                "    let path =\n        \"tools/long-run-supervisor-074/src/diagnostic_fixed_grouped_preparation_crash_tests.rs\";\n    let current = fence_custody::restore_join(path, &read(path));",
                "    let current = read(\n        \"tools/long-run-supervisor-074/src/diagnostic_fixed_grouped_preparation_crash_tests.rs\",\n    );")
        }
        "docs/testing/gated-test-registry.toml" => once(raw, GATES, ""),
        _ => raw.to_owned(),
    }
}
fn root() -> std::path::PathBuf {
    super::super::super::root()
}
fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "")
}
fn frozen(path: &str, commit: &str) -> String {
    let out = std::process::Command::new("git")
        .args(["show", &format!("{commit}:{path}")])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(out.status.success(), "{path}");
    String::from_utf8(out.stdout).unwrap().replace('\r', "")
}
#[test]
fn custody_join_preserves_every_old_runtime_body_and_guard() {
    for path in [
        "tools/long-run-supervisor-074/src/diagnostic_original_grouped_preparation.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_starting_storage.rs",
        "tools/long-run-supervisor-074/src/diagnostic_fixed_grouped_preparation_join_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_fixed_grouped_preparation_crash_tests.rs",
        "crates/xtask/tests/support/performance_contract_074_fence_custody.rs",
        "crates/xtask/tests/support/performance_contract_074_fixed_grouped.rs",
        "docs/testing/gated-test-registry.toml",
        "tools/long-run-supervisor-074/src/diagnostic_original_fence.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_collector_sinks.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_start_material.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_output_join.rs",
        "tools/long-run-supervisor-074/src/diagnostic_validation_frame.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_progress.rs",
        "tools/long-run-supervisor-074/src/diagnostic_autonomous_watchdog.rs",
        "tools/long-run-supervisor-074/src/diagnostic_lease.rs",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ] {
        assert_eq!(
            normalize(path, &read(path)),
            frozen(path, "4f5c8a6f"),
            "{path}"
        );
    }
    let path = "tools/long-run-supervisor-074/src/diagnostic_original_grouped_preparation.rs";
    let raw = read(path);
    for (from, to) in [
        ("received.boottime_ns(),", "now.boottime_ns(),"),
        ("self.original_round(allow_fixture)? != bounds", "false"),
        ("now >= bounds.first()", "now > bounds.first()"),
    ] {
        assert_ne!(
            normalize(path, &raw.replace(from, to)),
            frozen(path, "4f5c8a6f")
        );
    }
    assert_ne!(
        normalize(path, &format!("{raw}\nfn unapproved_tail() {{}}\n")),
        frozen(path, "4f5c8a6f")
    );
    let bridge =
        read("tools/long-run-supervisor-074/src/diagnostic_original_collector_custody_storage.rs");
    assert!(
        !approved_caller("diagnostic_original_collector_custody_storage.rs", &bridge)
            .contains("retain_custody(")
    );
    assert!(approved_caller("foreign.rs", &bridge).contains("retain_custody("));
    assert!(std::panic::catch_unwind(|| approved_caller(
        "diagnostic_original_collector_custody_storage.rs",
        &bridge.replace(CALLER, "")
    ))
    .is_err());
}
const VALIDATE: &str = r#"        let result = (|| {
            self.inspect_custody()?;
            self.original.validate(allow_fixture)?;
            self.inspect_custody()?;
            self.inspect_deadline()
        })();
        if result.is_err() {
            self.refuse();
        }
        result
"#;
fn enrolled(raw: &str) -> bool {
    raw.matches(VALIDATE).count() == 1
        && raw.matches(".material_round()").count() == 1
        && raw.matches("held.original.original_round(allow_fixture)? != bounds").count() == 1
        && raw.matches("Alarm::arm(bounds.first())").count() == 1
        && raw.matches("held.inspect_custody()?;").count() == 2
        && raw.contains("held.custody = Some(storage.retain_original_collector_custody()?);")
        && raw.find("held.custody = Some(") < raw.rfind("held.original.original_round(allow_fixture)? != bounds")
        && raw.rfind("held.inspect_custody()?;") < raw.find("held.ready = true;")
        && raw.contains("held.inspect_deadline()?;\n            held.original.transport.material.ready = true;\n            held.ready = true;")
}
#[test]
fn custody_join_requires_original_frames_before_ready_and_preserves_closed_execution() {
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_original_custody_join.rs");
    assert!(enrolled(&raw));
    for (from, to) in [
        (
            "self.original.validate(allow_fixture)?;",
            "let _ = allow_fixture;",
        ),
        (
            "held.original.original_round(allow_fixture)? != bounds",
            "false",
        ),
        ("held.inspect_custody()?;", "let _ = ();"),
        ("held.inspect_deadline()?;", "let _ = ();"),
        (
            "held.custody = Some(storage.retain_original_collector_custody()?);",
            "let _ = storage;",
        ),
    ] {
        assert!(!enrolled(&raw.replace(from, to)), "{from}");
    }
    assert!(
        raw.find("    original: OriginalGroupedPreparation")
            < raw.find("    custody: Option<OriginalCollectorFenceCustody>")
    );
    assert!(raw.contains("let allow_fixture = false;"));
    assert!(raw.contains("Err(OriginalCustodyFailure {"));
    assert!(raw.contains("held: Box::new(held),"));
    assert_eq!(raw.matches("/// ```compile_fail").count(), 7);
    for forbidden in [
        "Command::",
        "Connection",
        "Proxy",
        "pub fn start",
        "pub fn send",
        "pub fn into_",
        "CLOCK_REALTIME",
        "rearm(",
    ] {
        assert!(!raw.contains(forbidden), "{forbidden}");
    }
    let bridge =
        read("tools/long-run-supervisor-074/src/diagnostic_original_collector_custody_storage.rs");
    assert_eq!(bridge.matches(".retain_custody(").count(), 1);
    assert!(bridge.contains("self.revalidate()?;"));
    assert!(bridge.contains("self.latch_refusal();"));
    assert!(bridge.contains("Stamp::of(&self.original._lock.metadata()?) != fence.lock_stamp"));
    assert!(bridge.contains("libc::FD_CLOEXEC"));
    assert!(!bridge.contains("pub fn"));
    assert!(!bridge.contains("-> File"));
    let tests = read("tools/long-run-supervisor-074/src/diagnostic_original_custody_join_tests.rs");
    assert_eq!(tests.matches("authentication_calls,").count(), 2);
    assert!(tests.contains("34"));
    assert!(!raw.contains("bind_original_custody"));
    assert!(raw.contains("pub fn prepare_original_custodied("));
    assert!(raw.contains("causal::age(prepared.original.issued.boottime_ns(), clock()?)?;"));
    assert!(tests.contains("drop_eof_for_test"));
    assert!(tests.contains("late time is not"));
}
#[test]
fn custody_join_process_cuts_are_the_same_owned_sigkill_harness() {
    let original = frozen(
        "tools/long-run-supervisor-074/src/diagnostic_fixed_grouped_preparation_crash_tests.rs",
        "3334ab74",
    );
    let mut expected = original
        .replace("fixed_grouped_", "custody_join_")
        .replace("FIXED_GROUPED", "CUSTODY_JOIN")
        .replace("HC074-FIXED-GROUPED", "HC074-CUSTODY-JOIN")
        .replace("0x0740_6c17", "0x0740_c618");
    expected = once(&expected,
        "const POINTS: [&str; 6] = [\n    \"opened\",\n    \"loaded\",\n    \"bound\",\n    \"message\",\n    \"armed\",\n    \"returned-unacknowledged\",\n];",
        "const POINTS: [&str; 3] = [\"opened\", \"retained\", \"returned-unacknowledged\"];");
    let start = expected
        .find("                let mut alarm = outputs")
        .unwrap();
    let end = expected[start..]
        .find("                assert_eq!(stage, \"returned-unacknowledged\");")
        .unwrap()
        + start;
    let new = r#"                let mut alarm = outputs
                    .prepare_custodied_fixture_for_test(&bundle, |at, _| {
                        visited.push(at);
                        if at == stage {
                            group_ready(at, true);
                        }
                    })
                    .unwrap();
                assert_eq!(visited, POINTS[..2]);
"#;
    expected.replace_range(start..end, new);
    expected = expected
        .replace("points=6 exact=true", "points=3 exact=true")
        .replace(
            "sigkills=6 controls=1 lost_acks=1 inspections=16 admission=false",
            "sigkills=3 controls=1 lost_acks=1 inspections=10 admission=false",
        );
    let current =
        read("tools/long-run-supervisor-074/src/diagnostic_original_custody_join_process_tests.rs");
    assert_eq!(current, expected);
    for (from, to) in [
        ("libc::SIGKILL", "libc::SIGTERM"),
        ("inspection in 1..=2", "inspection in 1..=1"),
        ("repeat in 1..=3", "repeat in 1..=1"),
    ] {
        assert_ne!(current.replace(from, to), expected);
    }
}
