//! Additive fixed-origin preparation; all prior validators remain frozen.
#[path = "performance_contract_074_fence_custody.rs"]
mod fence_custody;
const GUARDIAN_COMPAT: &str = r###"
## Private diagnostic guardian notices

The internal 0.74 guardian fork notice protocol, guardian-fork-notice-074-v1,
uses one inherited SOCK_SEQPACKET connection within the same executable image.
A packet is exactly one byte: 1 is readiness; 2 isolation failure; 3 observation
failure; 4 revocation failure; 5 read failure; 6 write failure; 7 overflow;
8 fsync failure; 9 close failure; 10 expiry. Unknown, truncated or oversized packets
are not readiness. The private parent must not interpret disconnect or a missing
notice as successful closure, start authority, retry or recovered acknowledgement.

There is no named listener, external producer, CLI or independently versioned
reader. No migration or durable artifact is introduced. Failure notification is
best effort after receiver loss; the guardian retains the fence regardless.
This protocol does not replace any signed lifecycle record or manager ACK.
"###;
const GATES: &str = r###"
# BEGIN FIXED GROUPED PREPARATION GATES
[[gate]]
id = "cfg.hydracache-long-run-supervisor-074.fixed-grouped-preparation-api"
kind = "cfg_test_target"
source = "tools/long-run-supervisor-074/tests/diagnostic_fixed_grouped_preparation.rs"
package = "hydracache-long-run-supervisor-074"
target = "diagnostic_fixed_grouped_preparation"
test = ""
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Nonexecuting fixed-origin grouped owner; no send, guardian or execution admission"
tier = "fast"
required_features = []
required_env = []
required_tools = ["cargo"]
timeout_seconds = 120
owner_release = "0.74.0"
ship_mandatory = false
artifacts = []

[gate.ci]
workflow = ".github/workflows/ci.yml"
job = "rust"
step = "Release 0.74 Linux supervisor contracts"

[gate.command]
program = "cargo"
args = ["test","-p","hydracache-long-run-supervisor-074","--test","diagnostic_fixed_grouped_preparation","--locked"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.fixed-grouped-preparation-screen"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_fixed_grouped_preparation_join_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "fixed_grouped_preparation_screen"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Nonexecuting fixed-origin grouped owner; no send, guardian or execution admission"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::fixed_grouped_preparation_screen","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.fixed-grouped-preparation-healthy"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_fixed_grouped_preparation_join_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "fixed_grouped_preparation_healthy"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Nonexecuting fixed-origin grouped owner; no send, guardian or execution admission"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::fixed_grouped_preparation_healthy","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.fixed-grouped-preparation-refusals"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_fixed_grouped_preparation_join_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "fixed_grouped_preparation_refusals"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Nonexecuting fixed-origin grouped owner; no send, guardian or execution admission"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::fixed_grouped_preparation_refusals","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.fixed-grouped-preparation-fixture-origin"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_fixed_grouped_preparation_join_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "fixed_grouped_preparation_fixture_cannot_enter_production"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Nonexecuting fixed-origin grouped owner; no send, guardian or execution admission"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::fixed_grouped_preparation_fixture_cannot_enter_production","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.fixed-grouped-process-screen"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_fixed_grouped_preparation_crash_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_fixed_grouped_process_screen"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Shared fixed grouped owner, owned SIGKILL and private return ACK loss; no delivery or execution"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::crash_tests::fixed_grouped_process_tests::original_fixed_grouped_process_screen","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.fixed-grouped-process-matrix"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_fixed_grouped_preparation_crash_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_fixed_grouped_process_matrix"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Shared fixed grouped owner, owned SIGKILL and private return ACK loss; no delivery or execution"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::crash_tests::fixed_grouped_process_tests::original_fixed_grouped_process_matrix","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.autonomous-writer-fence"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_autonomous_watchdog_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "autonomous_writer_fence_matrix"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Manual isolated nonroot kernel guardian and writer-fence faults; no production execution enrollment"
tier = "manual"
required_features = []
required_env = []
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","watchdog::autonomous::tests::autonomous_writer_fence_matrix","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]
# END FIXED GROUPED PREPARATION GATES
"###;

pub(super) fn normalize(path: &str, raw: &str) -> String {
    let current = fence_custody::normalize(path, raw);
    let raw = current.as_str();
    if path == "crates/xtask/tests/support/performance_contract_074_start_material.rs" {
        let addition = "super::fixed_grouped::normalize(p, &read(p)),";
        assert_eq!(raw.matches(addition).count(), 1);
        return raw.replacen(addition, "read(p),", 1);
    }
    let addition = match path {
        "docs/COMPAT.md" => GUARDIAN_COMPAT,
        "tools/long-run-supervisor-074/src/watchdog.rs" =>
            "\n// Private mechanics only. Original admission/descriptor transfer is not enrolled.\n#[cfg(all(target_os = \"linux\", target_arch = \"x86_64\"))]\n#[allow(dead_code)]\n#[path = \"diagnostic_autonomous_watchdog.rs\"]\nmod autonomous;\n",
        "tools/long-run-supervisor-074/src/diagnostic_original_output_join_crash_tests.rs" =>
            "#[path = \"diagnostic_fixed_grouped_preparation_crash_tests.rs\"]\nmod fixed_grouped_process_tests;\n",
        "tools/long-run-supervisor-074/src/diagnostic_original_pipe_transport.rs" =>
            "#[path = \"diagnostic_original_grouped_preparation.rs\"]\npub mod grouped_preparation;\n",
        "tools/long-run-supervisor-074/src/diagnostic_original_safety_alarm_join_tests.rs" =>
            "#[path = \"diagnostic_fixed_grouped_preparation_join_tests.rs\"]\nmod fixed_grouped_tests;\n",
        "crates/xtask/tests/performance_contract_074.rs" =>
            "\n#[path = \"support/performance_contract_074_fixed_grouped.rs\"]\nmod fixed_grouped;\n",
        "docs/testing/gated-test-registry.toml" => GATES,
        _ => return raw.to_owned(),
    };
    assert_eq!(raw.matches(addition).count(), 1, "{path}");
    raw.replacen(addition, "", 1)
}
fn read(path: &str) -> String {
    crate::runtime_io::read_to_string(super::root().join(path))
        .unwrap()
        .replace('\r', "")
}
fn frozen(path: &str) -> String {
    let out = std::process::Command::new("git")
        .args(["show", &format!("8193e642:{path}")])
        .current_dir(super::root())
        .output()
        .unwrap();
    assert!(out.status.success(), "{path}");
    String::from_utf8(out.stdout).unwrap().replace('\r', "")
}
fn slice<'a>(raw: &'a str, start: &str, end: &str) -> &'a str {
    let tail = &raw[raw.find(start).unwrap()..];
    &tail[..tail.find(end).unwrap()]
}
#[test]
fn fixed_grouped_registration_does_not_normalize_away_old_behavior_mutations() {
    for path in [
        "tools/long-run-supervisor-074/src/diagnostic_original_pipe_transport.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_safety_alarm_join_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_safety_alarm.rs",
        "tools/long-run-supervisor-074/src/diagnostic_grouped_preparation_candidate.rs",
        "tools/long-run-supervisor-074/src/diagnostic_grouped_preparation_join_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_grouped_preparation_crash_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_validation_frame.rs",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ] {
        assert_eq!(normalize(path, &read(path)), frozen(path), "{path}");
    }
    let path = "tools/long-run-supervisor-074/src/diagnostic_original_pipe_transport.rs";
    let raw = read(path);
    assert_ne!(
        normalize(
            path,
            &raw.replace("self.validate(false, false)", "self.validate(true, false)")
        ),
        frozen(path)
    );
    assert!(std::panic::catch_unwind(|| normalize(
        path,
        &raw.replace(
            "pub mod grouped_preparation;",
            "pub(crate) mod grouped_preparation;"
        )
    ))
    .is_err());
}
#[test]
fn fixed_grouped_alarm_and_closing_round_match_the_frozen_reference_exactly() {
    let original = frozen("tools/long-run-supervisor-074/src/diagnostic_original_safety_alarm.rs");
    let new = read("tools/long-run-supervisor-074/src/diagnostic_original_grouped_preparation.rs")
        .replace("OriginalGroupedPrepError", "OriginalSafetyAlarmError");
    assert_eq!(
        slice(&new, "use super::*;", "// No production path"),
        slice(&original, "use super::*;", "/// Retained one-shot alarm"),
    );
    assert_eq!(
        slice(&new, "    fn original_round(", "    fn validate("),
        slice(&original, "    fn original_round(", "    fn validate("),
    );
    let mutant = new.replace("received.boottime_ns(),", "now.boottime_ns(),");
    assert_ne!(
        slice(&mutant, "    fn original_round(", "    fn validate("),
        slice(&original, "    fn original_round(", "    fn validate(")
    );
    let mutant = new.replace("now >= bounds.first()", "now > bounds.first()");
    assert_ne!(
        slice(&mutant, "use super::*;", "// No production path"),
        slice(&original, "use super::*;", "/// Retained one-shot alarm")
    );
}
#[test]
fn fixed_grouped_production_surface_has_no_replacement_or_execution_input() {
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_original_grouped_preparation.rs");
    let constructor = slice(
        &raw,
        "    pub fn prepare_original_grouped(",
        "    #[cfg(test)]\n    pub(crate) fn prepare_fixed_grouped_fixture_for_test",
    );
    assert!(constructor.starts_with("    pub fn prepare_original_grouped(\n        self,\n"));
    assert!(constructor.contains("None,"));
    assert!(raw.contains("inspect_fixed_install(identity, policy)"));
    assert!(raw.contains("#[cfg(test)]\n    if let Some(path) = fixture"));
    assert!(raw.contains("held.original_round(allow_fixture)? != bounds"));
    assert!(raw.contains("held.transport.material.ready = true"));
    assert_eq!(raw.matches("Alarm::arm(bounds.first())").count(), 1);
    for forbidden in [
        "Connection",
        "Proxy",
        "Command::",
        "SystemTime",
        "CLOCK_REALTIME",
        "pub fn start",
        "pub fn send",
        "pub fn rearm",
        "pub fn into_",
    ] {
        assert!(!raw.contains(forbidden), "{forbidden}");
    }
}

#[test]
fn fixed_grouped_process_matrix_preserves_runtime_and_exact_owned_harness() {
    let at_baseline = |path: &str| {
        let out = std::process::Command::new("git")
            .args(["show", &format!("49db0f7f:{path}")])
            .current_dir(super::root())
            .output()
            .unwrap();
        assert!(out.status.success(), "{path}");
        String::from_utf8(out.stdout).unwrap().replace('\r', "")
    };
    for path in [
        "tools/long-run-supervisor-074/src/diagnostic_original_grouped_preparation.rs",
        "tools/long-run-supervisor-074/src/diagnostic_fixed_grouped_preparation_join_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_fixed_grouped_preparation_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_output_join_crash_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_grouped_preparation_crash_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_grouped_preparation_candidate.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_safety_alarm.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_output_join.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_pipe_transport.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_start_material.rs",
        "tools/long-run-supervisor-074/src/diagnostic_validation_frame.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_progress.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs",
        "tools/long-run-supervisor-074/src/diagnostic_lease.rs",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ] {
        let current = if path.ends_with("diagnostic_original_output_join_crash_tests.rs") {
            normalize(path, &read(path))
        } else {
            fence_custody::restore_join(path, &read(path))
        };
        assert_eq!(current, at_baseline(path), "{path}");
    }
    let original = at_baseline(
        "tools/long-run-supervisor-074/src/diagnostic_grouped_preparation_crash_tests.rs",
    );
    let expected = original
        .replace("grouped_", "fixed_grouped_")
        .replace("HC074-GROUPED", "HC074-FIXED-GROUPED")
        .replace("HC074_PRIVATE_GROUPED", "HC074_PRIVATE_FIXED_GROUPED")
        // Exactly one rustfmt fold caused by the longer private log namespace.
        .replace(
            "    println!(\"HC074-FIXED-GROUPED-PROCESS-SCREEN sigkills=1 controls=1 inspections=4 completed=true\");",
            "    println!(\n        \"HC074-FIXED-GROUPED-PROCESS-SCREEN sigkills=1 controls=1 inspections=4 completed=true\"\n    );",
        );
    let path =
        "tools/long-run-supervisor-074/src/diagnostic_fixed_grouped_preparation_crash_tests.rs";
    let current = fence_custody::restore_join(path, &read(path));
    assert_eq!(current, expected);
    for (from, to) in [
        ("libc::SIGKILL", "libc::SIGTERM"),
        (
            "prepare_fixed_grouped_fixture_for_test",
            "prepare_grouped_fixture_for_test",
        ),
        ("inspection in 1..=2", "inspection in 1..=1"),
        ("repeat in 1..=3", "repeat in 1..=1"),
    ] {
        assert_ne!(current.replace(from, to), expected, "{from}");
    }
}

#[test]
fn autonomous_guardian_is_private_and_preserves_original_admission() {
    let path = "tools/long-run-supervisor-074/src/watchdog.rs";
    let addition = "\n// Private mechanics only. Original admission/descriptor transfer is not enrolled.\n#[cfg(all(target_os = \"linux\", target_arch = \"x86_64\"))]\n#[allow(dead_code)]\n#[path = \"diagnostic_autonomous_watchdog.rs\"]\nmod autonomous;\n";
    let current = read(path);
    assert_eq!(current.matches(addition).count(), 1);
    let old = std::process::Command::new("git")
        .args(["show", &format!("5621df7b:{path}")])
        .current_dir(super::root())
        .output()
        .unwrap();
    assert!(old.status.success());
    assert_eq!(
        current.replacen(addition, "", 1),
        String::from_utf8(old.stdout).unwrap().replace('\r', "")
    );
    let p = "crates/xtask/tests/support/performance_contract_074_start_material.rs";
    let old = std::process::Command::new("git")
        .args(["show", &format!("5621df7b:{p}")])
        .current_dir(super::root())
        .output()
        .unwrap();
    assert!(old.status.success());
    assert_eq!(
        normalize(p, &read(p)),
        String::from_utf8(old.stdout).unwrap().replace('\r', "")
    );
    let mutation = current.replace(
        "rejection_gap_seconds <= warning_gap_seconds",
        "rejection_gap_seconds < warning_gap_seconds",
    );
    assert_ne!(normalize(path, &mutation), normalize(path, &current));
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_autonomous_watchdog.rs");
    for forbidden in [
        "pub fn",
        "pub struct",
        "pub(crate)",
        "Command::",
        "LOCK_UN",
        "PR_SET_PDEATHSIG",
        "SystemTime",
        "CLOCK_REALTIME",
        "unlink",
        "rearm",
    ] {
        // The comment explaining why LOCK_UN is forbidden is not executable.
        assert!(
            !raw.lines()
                .filter(|l| !l.trim_start().starts_with("//"))
                .any(|l| l.contains(forbidden)),
            "{forbidden}"
        );
    }
    for required in [
        "libc::SYS_fork",
        "libc::SYS_close_range",
        "libc::SYS_pidfd_send_signal",
        "libc::TFD_TIMER_ABSTIME",
        "libc::CLOCK_BOOTTIME",
        "libc::MSG_NOSIGNAL",
        "writers_terminal && eof == [true, true] && !refused",
        "libc::fsync(fd)",
        "libc::close(raw.fence)",
    ] {
        assert!(raw.contains(required), "{required}");
    }
    let collector = slice(&raw, "fn collect(", "fn guardian_refused(");
    assert!(
        collector.find("libc::fsync(fd)").unwrap() < collector.rfind("libc::close(fd)").unwrap()
    );
    let guardian = slice(&raw, "fn run(", "fn spawn(");
    let spawn = &raw[raw.find("fn spawn_inner(").unwrap()..];
    assert!(spawn.find("&mut previous,").unwrap() < spawn.find("libc::SYS_fork").unwrap());
    assert!(spawn.find("&previous,").unwrap() < spawn.find("drop(input)").unwrap());
    assert!(spawn.contains("let fork_error = (pid < 0).then(io::Error::last_os_error)"));
    assert!(!guardian.contains("libc::write("));
    assert!(!guardian.contains("libc::fsync("));
    assert!(
        guardian.find("libc::waitpid(").unwrap() < guardian.find("libc::close(raw.fence)").unwrap()
    );
    assert!(
        guardian
            .find("releasable(writers_terminal, eof, false)")
            .unwrap()
            < guardian.find("libc::close(raw.fence)").unwrap()
    );
}
