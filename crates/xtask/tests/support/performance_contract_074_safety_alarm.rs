//! Test-only alarm and leaf attribution; original runtime stays exact.
const GATES: &str = r###"
# BEGIN ORIGINAL SAFETY ALARM GATES
[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-safety-alarm-cost-attribution"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_safety_alarm_join_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_safety_alarm_cost_attribution"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Cost diagnostic only; refusal is not successful preparation or admission"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::original_safety_alarm_cost_attribution","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]
[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-validation-leaf-cost-attribution"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_validation_leaf_cost_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_validation_leaf_cost_attribution"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Existing opening callback only; full closing round retained in explicit final bucket"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::leaf_cost_tests::original_validation_leaf_cost_attribution","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-controller-component-cost-controls"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_controller_component_cost_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_controller_component_cost_controls"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Separate original-object component controls, not a ReservedLive decomposition or new admission"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::leaf_cost_tests::component_cost_tests::original_controller_component_cost_controls","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-grouped-preparation-screen"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_grouped_preparation_join_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_grouped_preparation_screen"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Test-only grouped nonexecuting schedule; no start, send or production admission"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::grouped_tests::original_grouped_preparation_screen","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-grouped-preparation-healthy"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_grouped_preparation_join_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_grouped_preparation_healthy"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Test-only grouped nonexecuting schedule; no start, send or production admission"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::grouped_tests::original_grouped_preparation_healthy","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-grouped-preparation-refusals"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_grouped_preparation_join_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_grouped_preparation_refusals"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Test-only grouped nonexecuting schedule; no start, send or production admission"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::grouped_tests::original_grouped_preparation_refusals","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-grouped-process-screen"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_grouped_preparation_crash_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_grouped_process_screen"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Owned nonexecuting SIGKILL and synthetic return ACK loss; no systemd delivery or admission"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::crash_tests::grouped_process_tests::original_grouped_process_screen","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-grouped-process-matrix"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_grouped_preparation_crash_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_grouped_process_matrix"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Owned nonexecuting SIGKILL and synthetic return ACK loss; no systemd delivery or admission"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::crash_tests::grouped_process_tests::original_grouped_process_matrix","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

# END ORIGINAL SAFETY ALARM GATES
"###;
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let current = super::fixed_grouped::normalize(path, raw);
    let raw = current.as_str();
    if path == "tools/long-run-supervisor-074/src/diagnostic_original_output_join_crash_tests.rs" {
        let registration = "#[path = \"diagnostic_grouped_preparation_crash_tests.rs\"]\nmod grouped_process_tests;\n";
        assert_eq!(raw.matches(registration).count(), 1);
        return raw.replacen(registration, "", 1);
    }
    if path == "crates/xtask/tests/support/performance_contract_074_start_material.rs" {
        let adapter = "super::safety_alarm::normalize(p, &read(p)),";
        assert_eq!(raw.matches(adapter).count(), 1);
        return raw.replacen(adapter, "read(p),", 1);
    }
    let addition = match path {
        "tools/long-run-supervisor-074/src/diagnostic_original_pipe_transport.rs" =>
            "#[cfg(test)]\n#[path = \"diagnostic_original_safety_alarm.rs\"]\npub(crate) mod safety_alarm;\n",
        "tools/long-run-supervisor-074/src/diagnostic_original_pipe_transport_join_tests.rs" =>
            "#[path = \"diagnostic_original_safety_alarm_join_tests.rs\"]\nmod safety_alarm_tests;\n",
        "crates/xtask/tests/performance_contract_074.rs" =>
            "\n#[path = \"support/performance_contract_074_safety_alarm.rs\"]\nmod safety_alarm;\n",
        "crates/xtask/tests/support/performance_contract_074_pipe_transport.rs" =>
            "    let current = super::safety_alarm::normalize(path, raw);\n    let raw = current.as_str();\n",
        "docs/testing/gated-test-registry.toml" => GATES,
        _ => return raw.to_owned(),
    };
    assert_eq!(
        raw.matches(addition).count(),
        1,
        "unexpected safety alarm registration: {path}"
    );
    let result = raw.replacen(addition, "", 1);
    if path == "tools/long-run-supervisor-074/src/diagnostic_original_pipe_transport.rs" {
        let boundary = "/// The rejected alarm composition is test-only; it grants no production API.\n/// ```compile_fail\n/// use hydracache_long_run_supervisor_074::diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::starting::pipes::sinks::start_material::pipe_transport::OriginalCollectorPipeTransport;\n/// fn alarm(p: OriginalCollectorPipeTransport<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>) { p.prepare_original_safety_alarm(); }\n/// ```\n";
        assert_eq!(result.matches(boundary).count(), 1);
        result.replacen(boundary, "", 1)
    } else {
        result
    }
}
#[test]
fn rejected_original_alarm_stays_test_only_with_exact_original_guards() {
    let read = |p: &str| {
        std::fs::read_to_string(super::root().join(p))
            .unwrap()
            .replace('\r', "")
    };
    for p in [
        "tools/long-run-supervisor-074/src/diagnostic_original_pipe_transport.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_pipe_transport_join_tests.rs",
        "crates/xtask/tests/performance_contract_074.rs",
        "crates/xtask/tests/support/performance_contract_074_pipe_transport.rs",
        "docs/testing/gated-test-registry.toml",
        "tools/long-run-supervisor-074/src/diagnostic_original_start_material.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_output_join.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_progress.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs",
        "tools/long-run-supervisor-074/src/diagnostic_lease.rs",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ] {
        let out = std::process::Command::new("git")
            .args(["show", &format!("4c33c5ca:{p}")])
            .current_dir(super::root())
            .output()
            .unwrap();
        assert!(out.status.success());
        assert_eq!(
            normalize(p, &read(p)),
            String::from_utf8(out.stdout).unwrap().replace('\r', ""),
            "{p}"
        );
    }
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_original_safety_alarm.rs");
    let runtime = raw
        .lines()
        .filter(|l| !l.trim_start().starts_with("///"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!runtime.contains("pub fn prepare_original_safety_alarm("));
    assert!(!runtime.contains("pub fn revalidate("));
    assert_eq!(runtime.matches("libc::timerfd_settime(").count(), 1);
    for marker in [
        "libc::CLOCK_BOOTTIME",
        "libc::TFD_TIMER_ABSTIME",
        "libc::timerfd_gettime",
        "received_time().clone()",
        "original_reservation_time(Some(previous))",
        "now >= bounds.first()",
        "self.original_round(allow_fixture)? != bounds",
        "alarm.inspect()?",
        "held: Box::new(held)",
    ] {
        assert!(runtime.contains(marker), "{marker}");
    }
    for denied in [
        "pub fn rearm",
        "pub fn into_",
        "pub fn start",
        "Connection",
        "Proxy",
        "Command::",
        "libc::kill(",
        "libc::read(",
        "SystemTime",
        "CLOCK_REALTIME",
    ] {
        assert!(!runtime.contains(denied), "{denied}");
    }
    assert!(std::panic::catch_unwind(|| normalize(
        "tools/long-run-supervisor-074/src/diagnostic_original_pipe_transport.rs",
        &read("tools/long-run-supervisor-074/src/diagnostic_original_pipe_transport.rs")
            .replace("#[cfg(test)]", "#[cfg(all())]")
    ))
    .is_err());
}

#[test]
fn validation_leaf_attribution_keeps_all_runtime_and_observation_points_exact() {
    for path in [
        "tools/long-run-supervisor-074/src/diagnostic_validation_frame.rs",
        "tools/long-run-supervisor-074/src/diagnostic_preconsent_frame.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_output_join.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_start_material.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_pipe_transport.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_safety_alarm.rs",
        "tools/long-run-supervisor-074/src/diagnostic_validation_trace.rs",
        "tools/long-run-supervisor-074/src/diagnostic_lease.rs",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ] {
        let before = std::process::Command::new("git")
            .args(["show", &format!("48aa96b7:{path}")])
            .current_dir(super::root())
            .output()
            .unwrap();
        assert!(before.status.success());
        let current = std::fs::read_to_string(super::root().join(path)).unwrap();
        assert_eq!(
            super::fixed_grouped::normalize(path, &current.replace('\r', "")),
            String::from_utf8(before.stdout).unwrap().replace('\r', ""),
            "{path}"
        );
    }
}

#[test]
fn component_cost_controls_leave_all_original_validator_bodies_exact() {
    for path in [
        "tools/long-run-supervisor-074/src/diagnostic_collector_reserved.rs",
        "tools/long-run-supervisor-074/src/diagnostic_controller_request_attempt.rs",
        "tools/long-run-supervisor-074/src/diagnostic_controller_build_input.rs",
        "tools/long-run-supervisor-074/src/diagnostic_oidc_input.rs",
        "tools/long-run-supervisor-074/src/diagnostic_oidc_profile.rs",
        "tools/long-run-supervisor-074/src/diagnostic_controller_trust_input.rs",
        "tools/long-run-supervisor-074/src/diagnostic_worker_kernel_binding.rs",
    ] {
        let before = std::process::Command::new("git")
            .args(["show", &format!("bb2c843f:{path}")])
            .current_dir(super::root())
            .output()
            .unwrap();
        assert!(before.status.success());
        let current = std::fs::read_to_string(super::root().join(path)).unwrap();
        assert_eq!(
            crate::live_walk::normalize(path, &current.replace('\r', "")),
            String::from_utf8(before.stdout).unwrap().replace('\r', ""),
            "{path}"
        );
    }
}

#[test]
fn grouped_preparation_candidate_is_test_only_and_preserves_all_runtime_guards() {
    for path in [
        "tools/long-run-supervisor-074/src/diagnostic_original_start_material.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_pipe_transport.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_safety_alarm.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_collector_sinks.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_output_join.rs",
        "tools/long-run-supervisor-074/src/diagnostic_validation_frame.rs",
        "tools/long-run-supervisor-074/src/diagnostic_worker_kernel_binding.rs",
        "tools/long-run-supervisor-074/src/diagnostic_lease.rs",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ] {
        let before = std::process::Command::new("git")
            .args(["show", &format!("67419018:{path}")])
            .current_dir(super::root())
            .output()
            .unwrap();
        assert!(before.status.success());
        let current = std::fs::read_to_string(super::root().join(path)).unwrap();
        assert_eq!(
            super::fixed_grouped::normalize(path, &current.replace('\r', "")),
            String::from_utf8(before.stdout).unwrap().replace('\r', ""),
            "{path}"
        );
    }
    let candidate = std::fs::read_to_string(
        super::root()
            .join("tools/long-run-supervisor-074/src/diagnostic_grouped_preparation_candidate.rs"),
    )
    .unwrap();
    for marker in [
        "material_round()",
        "held.original_round(true)? != bounds",
        "Alarm::arm(bounds.first())",
        "alarm.inspect()?",
        "held.transport.material.outputs.refuse()",
        "held: Box::new(held)",
        "held.transport.material.ready = true",
    ] {
        assert!(candidate.contains(marker), "{marker}");
    }
    for forbidden in [
        "pub fn ",
        "Connection",
        "Proxy",
        "Command::",
        "publish_starting",
        "prepare_original_outputs",
        "SystemTime",
        "CLOCK_REALTIME",
    ] {
        assert!(!candidate.contains(forbidden), "{forbidden}");
    }
    assert_eq!(candidate.matches("Alarm::arm(").count(), 1);
    let registered = std::fs::read_to_string(
        super::root()
            .join("tools/long-run-supervisor-074/src/diagnostic_original_safety_alarm_tests.rs"),
    )
    .unwrap();
    assert!(registered.contains("#[path = \"diagnostic_grouped_preparation_candidate.rs\"]"));
}

#[test]
fn grouped_process_loss_preserves_original_candidate_and_owned_crash_harness() {
    let path = "tools/long-run-supervisor-074/src/diagnostic_original_output_join_crash_tests.rs";
    let current = std::fs::read_to_string(super::root().join(path))
        .unwrap()
        .replace('\r', "");
    let before = std::process::Command::new("git")
        .args(["show", &format!("674ea586:{path}")])
        .current_dir(super::root())
        .output()
        .unwrap();
    assert!(before.status.success());
    let frozen = String::from_utf8(before.stdout).unwrap().replace('\r', "");
    assert_eq!(normalize(path, &current), frozen);
    assert_ne!(
        normalize(path, &current.replace("libc::SIGKILL", "libc::SIGTERM")),
        frozen
    );
    for path in [
        "tools/long-run-supervisor-074/src/diagnostic_grouped_preparation_candidate.rs",
        "tools/long-run-supervisor-074/src/diagnostic_grouped_preparation_join_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_safety_alarm.rs",
        "tools/long-run-supervisor-074/src/diagnostic_validation_frame.rs",
        "tools/long-run-supervisor-074/src/diagnostic_lease.rs",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ] {
        let out = std::process::Command::new("git")
            .args(["show", &format!("674ea586:{path}")])
            .current_dir(super::root())
            .output()
            .unwrap();
        assert!(out.status.success());
        assert_eq!(
            std::fs::read_to_string(super::root().join(path))
                .unwrap()
                .replace('\r', ""),
            String::from_utf8(out.stdout).unwrap().replace('\r', ""),
            "{path}"
        );
    }
}
