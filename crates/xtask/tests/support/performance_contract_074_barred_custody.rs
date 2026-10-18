//! Original-only pending custody; no old predicate or activated engine changes.
const GATES: &str = r###"
# BEGIN ORIGINAL BARRED CUSTODY GATES
[[gate]]
id = "tool.hydracache-long-run-supervisor-074.barred-custody-screen"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_barred_custody_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_barred_custody_screen"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Private autonomous original pending custody; no collector or authenticated start"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::original_barred_custody_screen","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.barred-custody-healthy"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_barred_custody_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_barred_custody_healthy"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Private autonomous original pending custody; no collector or authenticated start"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::original_barred_custody_healthy","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.barred-custody-refusals"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_barred_custody_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_barred_custody_refusals"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Private autonomous original pending custody; no collector or authenticated start"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::original_barred_custody_refusals","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.barred-custody-process-matrix"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_barred_custody_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_barred_custody_process_matrix"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Private autonomous original pending custody; no collector or authenticated start"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::original_barred_custody_process_matrix","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

# END ORIGINAL BARRED CUSTODY GATES
"###;
const COMPAT: &str = r###"
## Private original barred custody notices

The separate original-barred-custody-notice-074-v1 protocol uses one inherited
SOCK_SEQPACKET within the same executable image, with no external producer,
listener, CLI or attach API. Each notice is exactly one byte: ASCII P means
pending isolation only, E original-alarm expiry, F isolation/observation failure,
and L unexpected peer activity or loss. P is accepted exactly once during initial
pending construction; duplicate, unknown, empty, oversized or truncated notices
refuse. A later notice, missing notice, disconnect or terminal child cannot
authorize readiness, execution, collector release or recovered acknowledgement.

The child accepts no command and never consumes/rearms the original alarm. Its
failure notices are best effort; expiry or peer loss retains the fence regardless
of delivery. Cancellation uses the atomically created child pidfd while the
parent retains its exclusive original-owner borrow, not a protocol release or
completion command. Deploy only the same-image matched parent/child; old guardian
notice readers are separate and must not reinterpret this vocabulary. No durable
artifact, migration, public wire protocol or production start admission is added.
"###;
const SELF_REG: &str =
    "#[path = \"performance_contract_074_barred_custody.rs\"]\nmod barred_custody;\n";
const ADAPTER: &str =
    "    let current = barred_custody::normalize(path, raw);\n    let raw = current.as_str();\n";
const RUNTIME_REG: &str =
    "#[path = \"diagnostic_original_barred_custody.rs\"]\nmod barred_custody;\n";
const TEST_REG: &str =
    "#[path = \"diagnostic_original_barred_custody_tests.rs\"]\nmod barred_custody_tests;\n";
fn once(raw: &str, from: &str) -> String {
    assert_eq!(
        raw.matches(from).count(),
        1,
        "barred custody exception drift"
    );
    raw.replacen(from, "", 1)
}
pub(super) fn normalize(path: &str, raw: &str) -> String {
    match path {
        "tools/long-run-supervisor-074/src/diagnostic_original_descriptor_plan.rs" => {
            once(raw, RUNTIME_REG)
        }
        "tools/long-run-supervisor-074/src/diagnostic_original_descriptor_plan_tests.rs" => {
            once(raw, TEST_REG)
        }
        "crates/xtask/tests/support/performance_contract_074_original_barrier.rs" => {
            once(&once(raw, SELF_REG), ADAPTER)
        }
        "docs/testing/gated-test-registry.toml" => once(raw, GATES),
        "docs/COMPAT.md" => once(raw, COMPAT),
        _ => raw.to_owned(),
    }
}
fn root() -> std::path::PathBuf {
    super::super::root()
}
fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "")
}
fn frozen(path: &str) -> String {
    let out = std::process::Command::new("git")
        .args(["show", &format!("7e2f5725:{path}")])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap().replace('\r', "")
}
#[test]
fn barred_custody_preserves_all_old_frames_timer_origins_and_engine_bodies() {
    for path in [
        "tools/long-run-supervisor-074/src/diagnostic_original_descriptor_plan.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_descriptor_plan_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_custody_join.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_collector_custody_storage.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_grouped_preparation.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_collector_sinks.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_collector_pipes.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_fence.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_descriptor_holder_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_autonomous_watchdog.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs",
        "tools/long-run-supervisor-074/src/diagnostic_lease.rs",
        "crates/xtask/tests/support/performance_contract_074_original_barrier.rs",
        "docs/COMPAT.md",
        "docs/testing/gated-test-registry.toml",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ] {
        assert_eq!(normalize(path, &read(path)), frozen(path), "{path}");
    }
    let path = "tools/long-run-supervisor-074/src/diagnostic_original_descriptor_plan.rs";
    let raw = read(path);
    for (a, b) in [
        (
            "self.original.validate(self.allow_fixture)?;",
            "let _ = ();",
        ),
        ("if capture(self.original)? != self.inventory", "if false"),
        ("self.original.inspect_deadline()", "Ok(())"),
    ] {
        assert_ne!(normalize(path, &raw.replace(a, b)), frozen(path));
    }
    assert!(std::panic::catch_unwind(|| normalize(path, &raw.replace(RUNTIME_REG, ""))).is_err());
    assert_ne!(
        normalize(path, &format!("{raw}\nfn bypass() {{}}\n")),
        frozen(path)
    );
}
fn protocol_closed(raw: &str) -> bool {
    raw.contains("self.original.validate(self.allow_fixture)?;")
        && raw.contains("self.original.inspect_deadline()?;")
        && raw.contains("child.alive()?;")
        && raw.contains("if result.is_err()")
        && raw.contains("self.original.refuse();")
        && raw.contains("kernel: Option<KernelCustodian>,\n    original:")
}
#[test]
fn barred_custody_borrows_original_owner_and_keeps_child_below_full_live_checks() {
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_original_barred_custody.rs");
    assert!(protocol_closed(&raw));
    for needle in [
        "self.original.validate(self.allow_fixture)?;",
        "self.original.inspect_deadline()?;",
        "child.alive()?;",
        "self.original.refuse();",
    ] {
        assert!(!protocol_closed(&raw.replace(needle, "")), "{needle}");
    }
    for needed in [
        "held.inventory = Some(capture(held.original)?);",
        "held.original.inspect_deadline()?;",
        "[inventory.fds[0], inventory.fds[1]]",
        "held.validate()?;",
        "held.ready = true;",
        "held: Box::new(held)",
        "if capture(self.original)? != *inventory",
        "self.prepare_barred_with(\n            false,",
    ] {
        assert!(raw.contains(needed), "{needed}");
    }
    for denied in [
        "pub fn ",
        "timerfd_create",
        "timerfd_settime",
        "libc::write",
        "libc::fsync",
        "libc::flock",
        "Command::",
        "Connection::",
        "impl Clone",
        "CLOCK_REALTIME",
        "SYS_kill",
        "SYS_pidfd_open",
        "plan_original_descriptors(",
        "plan_descriptors_with(",
    ] {
        assert!(!raw.contains(denied), "{denied}");
    }
    assert!(
        raw.find("kernel: Option<KernelCustodian>").unwrap() < raw.find("original:\n").unwrap()
    );
}
#[test]
fn barred_custody_raw_continuation_has_no_output_or_activation_and_pins_atomically() {
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_original_barred_custody.rs");
    let body = raw
        .split("fn child(")
        .nth(1)
        .unwrap()
        .split("fn clone_arguments")
        .next()
        .unwrap();
    for denied in [
        "File::",
        "std::io",
        "unwrap(",
        "assert!",
        "Vec::",
        "format!",
        "println!",
        "libc::read",
        "libc::recv",
        "libc::close(",
        "_exit",
        "libc::write",
        "timerfd_settime",
    ] {
        assert!(!body.contains(denied), "{denied}");
    }
    for needed in [
        "SYS_close_range",
        "notice(peer, b'P')",
        "poll[0].fd = -1",
        "poll[1].revents != 0",
        "b'E'",
        "b'F'",
        "b'L'",
    ] {
        assert!(body.contains(needed), "{needed}");
    }
    assert!(raw.find("libc::SIG_SETMASK,").unwrap() < raw.find("libc::SYS_clone3").unwrap());
    assert!(raw.find("let owned = (pid > 0)").unwrap() < raw.find("let restored =").unwrap());
    assert!(raw.contains("arguments[0] = libc::CLONE_PIDFD as u64"));
    assert!(raw.contains("arguments[4] = libc::SIGCHLD as u64"));
    assert!(raw.contains("libc::SYS_pidfd_send_signal"));
    assert!(raw.contains("libc::WTERMSIG(status) == libc::SIGKILL"));
    let tests =
        read("tools/long-run-supervisor-074/src/diagnostic_original_barred_custody_tests.rs");
    for needed in [
        "call_count(&report, Stage::Bind)",
        "call_count(&report, Stage::Checks)",
        "mask(), parent_mask",
        "!reason.contains(\"Time\")",
        "Original(Original(Expired))",
        "repeat in 1..=3",
        "kernel.wait_expired_for_test()",
        "kernel.disconnect_for_test()",
        "retained_context_refuses_for_test(root)",
    ] {
        assert!(tests.contains(needed), "{needed}");
    }
    assert!(read(
        "tools/long-run-supervisor-074/src/diagnostic_original_barred_custody_test_api.rs"
    )
    .contains("fn detach_for_test"));
    assert!(
        raw.contains("#[cfg(test)]\n#[path = \"diagnostic_original_barred_custody_test_api.rs\"]")
    );
    assert!(read("docs/COMPAT.md").contains(COMPAT));
}
