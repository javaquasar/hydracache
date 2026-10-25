//! Test-only custody loss cuts; no ordinary pending implementation changes.
#[path = "performance_contract_074_barred_collector.rs"]
mod barred_collector;
const GATES: &str = r###"
# BEGIN ORIGINAL CUSTODY LOSS GATES
[[gate]]
id = "tool.hydracache-long-run-supervisor-074.custody-lost-pending"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_custody_loss_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_custody_lost_pending_notice"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Private pending custody notice and owner loss; no output or start"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::loss_tests::original_custody_lost_pending_notice","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.custody-owner-victim"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_custody_loss_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_custody_owner_victim"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Private pending custody notice and owner loss; no output or start"
tier = "manual"
required_features = []
required_env = ["HC074_START_MATERIAL_BUNDLE","HC074_PRIVATE_CUSTODY_LOSS_ROOT","HC074_PRIVATE_CUSTODY_LOSS_CUT","HC074_PRIVATE_CUSTODY_LOSS_AUTHORITY"]
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::loss_tests::original_custody_owner_victim","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

# END ORIGINAL CUSTODY LOSS GATES
"###;
const COMPAT: &str = r###"
## Private original custody owner loss fixture

The hc074-pending-owner-test-v1 packet exists only in the opt-in Linux libtest
fault fixture, never an ordinary-build listener or installed command. A private
owned Unix datagram carries bounded JSON (schema, owner, cut, deadline, origins,
closing_authentication) and exactly one SCM_RIGHTS atomically created custodian
pidfd. The test driver requires exact fields, the expected subprocess PID and
kernel SCM_CREDENTIALS UID/GID/PID, and rejects truncated or ambiguous ancillary
data. It never receives the original fence or private custodian notice peer.
Only matched fixture and driver versions are supported; unknown fields/versions
fail the test. This packet is neither a start permit nor a durable receipt.
"###;
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let current = barred_collector::normalize(path, raw);
    let raw = current.as_str();
    match path {
        "docs/testing/gated-test-registry.toml" => strip(raw, GATES),
        "docs/COMPAT.md" => strip(raw, COMPAT),
        _ => raw.to_owned(),
    }
}
fn strip(raw: &str, allowed: &str) -> String {
    assert_eq!(
        raw.matches(allowed).count(),
        1,
        "custody loss exception drift"
    );
    raw.replacen(allowed, "", 1)
}
fn read(path: &str) -> String {
    std::fs::read_to_string(super::root().join(path))
        .unwrap()
        .replace('\r', "")
}
#[test]
fn custody_loss_preserves_the_entire_ordinary_pending_implementation() {
    let path = "tools/long-run-supervisor-074/src/diagnostic_original_barred_custody.rs";
    let out = std::process::Command::new("git")
        .args(["show", &format!("069a1dab:{path}")])
        .current_dir(super::root())
        .output()
        .unwrap();
    assert!(out.status.success());
    let frozen = String::from_utf8(out.stdout).unwrap().replace('\r', "");
    let raw = barred_collector::normalize(path, &read(path));
    let start = raw
        .find("            #[cfg(test)]\n            if fault == 3 {")
        .unwrap();
    let end = start + raw[start..].find("\n            held.kernel\n").unwrap() + 1;
    let seam = &raw[start..end];
    assert!(seam.contains(".observe(inventory.fds[1], true)?;"));
    assert_eq!(strip(&raw, seam), frozen);
    for needed in [
        "child.alive()?;",
        "self.original.validate(self.allow_fixture)?;",
        "self.original.inspect_deadline()?;",
    ] {
        assert_ne!(strip(&raw.replace(needed, ""), seam), frozen);
    }
}
#[test]
fn custody_loss_fixture_keeps_actual_notice_and_exact_original_frame_cuts() {
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_original_custody_loss_tests.rs");
    for needed in [
        "prepare_barred_fixture_for_test(3,",
        "failure.reason_for_test(), \"Expired\"",
        "failure.origins_for_test()[..4]",
        "mask(), parent_mask",
        "failure.retry_for_test()",
        "failure.assert_lost_pending_for_test()",
        "records(leaf), before",
        "retained_context_refuses_for_test(root)",
        "barred.assert_original_for_test()",
        "libc::SCM_RIGHTS",
        "barred.pinned_for_test()",
        "call_count(&report, Stage::Bind)",
        "\"before-frame\"",
        "\"after-frame\"",
    ] {
        assert!(raw.contains(needed), "{needed}");
    }
    assert!(!raw.contains("detach_for_test"));
    let api =
        read("tools/long-run-supervisor-074/src/diagnostic_original_barred_custody_test_api.rs");
    let api = api
        .split("pub(crate) fn assert_lost_pending_for_test")
        .nth(1)
        .unwrap()
        .split("pub(crate) fn origins_for_test")
        .next()
        .unwrap();
    assert!(api.contains("assert_eq!(byte, b'E');"));
    assert!(api.contains("libc::MSG_TRUNC"));
    for needed in [
        "assert_eq!(byte, b'E');",
        "assert_eq!(poll[1].revents, 0);",
        "kernel.alive().unwrap();",
    ] {
        let proof = |text: &str| {
            text.contains("assert_eq!(byte, b'E');")
                && text.contains("assert_eq!(poll[1].revents, 0);")
                && text.contains("kernel.alive().unwrap();")
        };
        assert!(proof(api));
        assert!(!proof(&api.replace(needed, "")));
    }
    assert!(read("docs/COMPAT.md").contains(COMPAT));
}
fn driver_closed(raw: &str) -> bool {
    [
        "signal.pidfd_send_signal(owner, signal.SIGKILL)",
        "signal.pidfd_send_signal(custody, signal.SIGKILL)",
        "credentials == [(child.pid, os.getuid(), os.getgid())]",
        "len(received) == 1",
        "flags & ~socket.MSG_CMSG_CLOEXEC == 0",
        "parent_pid(guardian) == child.pid",
        "parent_pid(guardian) == os.getpid()",
        "os.waitid(os.P_PIDFD, fd, os.WEXITED)",
        "snapshot(root / \"origin\") == before",
        "closing_authentication",
        "time.CLOCK_BOOTTIME",
        "locked(lock, True)",
        "replay_refused=true",
    ]
    .iter()
    .all(|needed| raw.contains(needed))
}
#[test]
fn custody_loss_driver_pins_owned_processes_and_holds_no_original_aliases() {
    let raw = read("tools/long-run-supervisor-074/tests/original_custody_owner_loss.py");
    assert!(driver_closed(&raw));
    for needed in [
        "signal.pidfd_send_signal(owner, signal.SIGKILL)",
        "credentials == [(child.pid, os.getuid(), os.getgid())]",
        "os.waitid(os.P_PIDFD, fd, os.WEXITED)",
        "locked(lock, True)",
    ] {
        assert!(!driver_closed(&raw.replace(needed, "")));
    }
    for denied in [
        "os.kill(",
        "os.killpg(",
        "child.kill(",
        "systemctl",
        "sudo",
        "CLOCK_REALTIME",
        "timerfd_settime",
        "peer_for_test",
    ] {
        assert!(!raw.contains(denied), "{denied}");
    }
}
