//! Approved frequency change only; old validation and raw child bodies freeze.
#[path = "performance_contract_074_dispatch_exchange.rs"]
mod dispatch_exchange;
const GATES: &str = r###"
# BEGIN ORIGINAL ENROLLMENT FRAME GATES
[[gate]]
id = "tool.hydracache-long-run-supervisor-074.enrollment-frame-screen"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_enrollment_frame_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_enrollment_frame_screen"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Private combined custody and barred enrollment frame; no execution"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::collector_tests::enrollment_tests::original_enrollment_frame_screen","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.enrollment-frame-healthy"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_enrollment_frame_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_enrollment_frame_healthy"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Private combined custody and barred enrollment frame; no execution"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::collector_tests::enrollment_tests::original_enrollment_frame_healthy","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.enrollment-frame-refusals"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_enrollment_frame_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_enrollment_frame_refusals"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Private combined custody and barred enrollment frame; no execution"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::collector_tests::enrollment_tests::original_enrollment_frame_refusals","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.enrollment-frame-notice-loss"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_enrollment_fault_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_enrollment_lost_b_notice"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Private combined custody and barred enrollment frame; no execution"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::collector_tests::enrollment_tests::faults::original_enrollment_lost_b_notice","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.enrollment-frame-owner-victim"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_enrollment_fault_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_enrollment_owner_victim"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Private combined custody and barred enrollment frame; no execution"
tier = "manual"
required_features = []
required_env = ["HC074_START_MATERIAL_BUNDLE","HC074_PRIVATE_ENROLLMENT_LOSS_ROOT","HC074_PRIVATE_ENROLLMENT_LOSS_CUT","HC074_PRIVATE_ENROLLMENT_LOSS_AUTHORITY"]
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::collector_tests::enrollment_tests::faults::original_enrollment_owner_victim","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

# END ORIGINAL ENROLLMENT FRAME GATES
"###;
const COMPAT: &str = r###"
## Private combined original enrollment owner fixture

The hc074-enrollment-owner-test-v1 datagram is cfg(test) only and matched-image
only. A private credential-checked Unix datagram carries exactly two SCM_RIGHTS
pidfds, in collector/custodian order, never original I/O, fence or notice peers.
Closed JSON fields are schema, owner, roles, cut, deadline, origins and
frame_closed. The before-frame cut is after real P/B isolation but before the
closing round; after-frame is after all final observations. frame_closed is a
boolean matching that cut, not an authentication count or execution permit.
Reject extra/missing/duplicate keys, wrong types, credentials, roles, truncation
or ambiguous rights. These test messages confer no start, release or completion
authority. The ordinary P/B child protocols and absolute deadlines are unchanged.
"###;
fn strip(raw: &str, allowed: &str) -> String {
    assert_eq!(
        raw.matches(allowed).count(),
        1,
        "enrollment exception drift"
    );
    raw.replacen(allowed, "", 1)
}
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let normalized = dispatch_exchange::normalize(path, raw);
    let raw = normalized.as_str();
    match path {
        "docs/COMPAT.md" => strip(raw, COMPAT),
        "docs/testing/gated-test-registry.toml" => strip(raw, GATES),
        "tools/long-run-supervisor-074/src/diagnostic_original_barred_collector.rs" => strip(
            raw,
            "\n#[path = \"diagnostic_original_enrollment_frame.rs\"]\nmod enrollment_frame;\n",
        ),
        "tools/long-run-supervisor-074/src/diagnostic_original_barred_collector_tests.rs" => strip(
            raw,
            "#[path = \"diagnostic_original_enrollment_frame_tests.rs\"]\nmod enrollment_tests;\n",
        ),
        _ => raw.to_owned(),
    }
}
fn read(path: &str) -> String {
    std::fs::read_to_string(super::root().join(path))
        .unwrap()
        .replace('\r', "")
}
#[test]
fn enrollment_frame_preserves_all_preexisting_original_code() {
    for name in [
        "diagnostic_original_custody_join.rs",
        "diagnostic_original_grouped_preparation.rs",
        "diagnostic_original_descriptor_plan.rs",
        "diagnostic_original_barred_custody.rs",
        "diagnostic_original_barred_collector.rs",
        "diagnostic_original_barred_collector_kernel.rs",
        "diagnostic_original_barred_collector_test_api.rs",
        "diagnostic_original_barred_collector_tests.rs",
        "diagnostic_autonomous_watchdog.rs",
    ] {
        let path = format!("tools/long-run-supervisor-074/src/{name}");
        let out = std::process::Command::new("git")
            .args(["show", &format!("27dbbac7:{path}")])
            .current_dir(super::root())
            .output()
            .unwrap();
        assert!(out.status.success());
        assert_eq!(
            normalize(&path, &read(&path)),
            String::from_utf8(out.stdout).unwrap().replace('\r', ""),
            "{path}"
        );
    }
}
fn builder_closed(raw: &str) -> bool {
    [
        ".material_round()",
        "requirements(",
        "load_bundle(",
        ".bind(allow_fixture)",
        "worker(&r.state, &material.projection)",
        "message(&spec, &stdout.writer, &stderr.writer)",
        "inspect_message(",
        "original_reservation_time(Some(previous))",
        "boundaries(",
        "Alarm::arm(bounds.first())",
        "retain_original_collector_custody()",
        "original.inspect_custody()?;",
        "held.enroll(",
        ".original_round(allow_fixture)",
        "if alarm.deadline != bounds.first()",
        "original.inspect_deadline()?;",
        "held.observe()?;",
        "held.original.refuse();",
        "held: Box::new(held)",
    ]
    .iter()
    .all(|s| raw.contains(s))
}
#[test]
fn enrollment_frame_closes_original_predicates_before_any_ready_flag() {
    let raw =
        read("tools/long-run-supervisor-074/src/diagnostic_original_enrollment_frame_build.rs");
    assert!(builder_closed(&raw));
    for predicate in [
        ".material_round()",
        "load_bundle(",
        "inspect_message(",
        "boundaries(",
        "retain_original_collector_custody()",
        "held.enroll(",
        ".original_round(allow_fixture)",
        "if alarm.deadline != bounds.first()",
        "original.inspect_deadline()?;",
        "held.observe()?;",
    ] {
        assert!(!builder_closed(&raw.replace(predicate, "")), "{predicate}");
    }
    assert!(raw.find(".material_round()").unwrap() < raw.find("held.enroll(").unwrap());
    assert!(
        raw.find("held.enroll(").unwrap() < raw.find(".original_round(allow_fixture)").unwrap()
    );
    assert!(raw.find("held.observe()?;").unwrap() < raw.find("held.ready = true;").unwrap());
    assert_eq!(raw.matches(".original_round(allow_fixture)").count(), 1);
    assert!(!raw.contains(".validate("));
}
#[test]
fn enrollment_frame_owns_children_before_original_and_rechecks_live_inventory() {
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_original_enrollment_frame.rs");
    assert!(
        raw.find("collector: Option<KernelCollector>").unwrap()
            < raw.find("custodian: Option<KernelCustodian>").unwrap()
    );
    assert!(
        raw.find("custodian: Option<KernelCustodian>").unwrap()
            < raw.find("original: OriginalCustodiedPreparation").unwrap()
    );
    for predicate in [
        "collector.alive()?;",
        "custodian.alive()?;",
        "collector.observe(inventory.fds[1], false)?;",
        "custodian.observe(inventory.fds[1], false)?;",
        "capture(&self.original)? != *inventory",
        "self.original.inspect_custody()?;",
        "self.original.inspect_deadline()?;",
        "self.original.validate(self.allow_fixture)?;",
        "self.original.refuse();",
    ] {
        assert!(raw.contains(predicate), "{predicate}");
    }
    for name in [
        "diagnostic_original_enrollment_frame.rs",
        "diagnostic_original_enrollment_frame_build.rs",
    ] {
        let raw = read(&format!("tools/long-run-supervisor-074/src/{name}"));
        for denied in [
            "pub fn",
            ".send(",
            ".write(",
            "timerfd_settime",
            "SYS_clone3",
            "Deserialize",
            "derive(Clone",
            "into_stdio",
            "start_once",
        ] {
            assert!(!raw.contains(denied), "{name}: {denied}");
        }
    }
    assert!(read("docs/COMPAT.md").contains(COMPAT));
}
#[test]
fn enrollment_owner_driver_retains_exact_pidfd_and_credential_guards() {
    let raw = read("tools/long-run-supervisor-074/tests/original_enrollment_owner_loss.py");
    for needed in [
        "credentials == [(owner, os.getuid(), os.getgid())]",
        "len(received) == 2 and len(set(received)) == 2",
        "socket.MSG_CMSG_CLOEXEC",
        "signal.pidfd_send_signal(owner, signal.SIGKILL)",
        "signal.pidfd_send_signal(received[first], signal.SIGKILL)",
        "signal.pidfd_send_signal(received[last], signal.SIGKILL)",
        "base.parent_pid(pid) == child.pid",
        "os.path.samefile",
        "base.snapshot(root / \"origin\") == before",
        "cleanup_adopted(binary)",
        "frame_closed",
        "time.CLOCK_BOOTTIME",
        "base.locked(lock, True)",
    ] {
        assert!(raw.contains(needed), "{needed}");
    }
    for denied in [
        "os.kill(",
        "os.killpg(",
        "child.kill(",
        "systemctl",
        "sudo",
        "CLOCK_REALTIME",
    ] {
        assert!(!raw.contains(denied), "{denied}");
    }
}
