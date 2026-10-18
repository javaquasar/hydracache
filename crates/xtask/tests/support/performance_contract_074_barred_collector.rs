//! Original barred I/O enrollment must not weaken old frames or deadlines.
const GATES: &str = r###"
# BEGIN ORIGINAL BARRED COLLECTOR GATES
[[gate]]
id = "tool.hydracache-long-run-supervisor-074.barred-collector-screen"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_barred_collector_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_barred_collector_screen"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Private barred original collector and autonomous I/O quarantine; no authenticated start"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::collector_tests::original_barred_collector_screen","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.barred-collector-healthy"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_barred_collector_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_barred_collector_healthy"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Private barred original collector and autonomous I/O quarantine; no authenticated start"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::collector_tests::original_barred_collector_healthy","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.barred-collector-refusals"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_barred_collector_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_barred_collector_refusals"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Private barred original collector and autonomous I/O quarantine; no authenticated start"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::collector_tests::original_barred_collector_refusals","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.barred-collector-process-matrix"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_barred_collector_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_barred_collector_process_matrix"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Private barred original collector and autonomous I/O quarantine; no authenticated start"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::collector_tests::original_barred_collector_process_matrix","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

# END ORIGINAL BARRED COLLECTOR GATES
"###;
const COMPAT: &str = r###"
## Private original barred collector notices

The original-barred-collector-notice-074-v1 protocol is a separate, same-image
inherited SOCK_SEQPACKET, not the custodian or generic guardian vocabulary.
Exactly one byte B denotes barred original descriptor isolation only; E denotes
original-alarm expiry, L peer activity/loss and F isolation/observation/close
failure. Initial observation accepts B once; missing, duplicate, unknown, empty,
oversized or truncated notices and terminal children refuse. No notice grants
activation, original send/start, release, writer cessation or completion.

The child accepts no command and never reads/rearms the original alarm. Before
best-effort E/L/F it attempts to close both original readers and both original
O_RDWR sinks, then parks retaining its original fence alias. A failed close emits
F and cannot prove quiescence. The ordinary owner keeps its exclusive original
borrow through collector cancellation/reaping and subsequent custodian cleanup.
Matched image only; unknown notices fail closed. No public wire or durable format
is introduced, and the generic O_WRONLY engine remains unchanged.
"###;
const RUNTIME: &str =
    "\n#[path = \"diagnostic_original_barred_collector.rs\"]\nmod barred_collector;\n";
const TESTS: &str =
    "#[path = \"diagnostic_original_barred_collector_tests.rs\"]\nmod collector_tests;\n";
fn strip(raw: &str, allowed: &str) -> String {
    assert_eq!(
        raw.matches(allowed).count(),
        1,
        "barred collector exception drift"
    );
    raw.replacen(allowed, "", 1)
}
pub(super) fn normalize(path: &str, raw: &str) -> String {
    match path {
        "docs/testing/gated-test-registry.toml" => strip(raw, GATES),
        "docs/COMPAT.md" => strip(raw, COMPAT),
        "tools/long-run-supervisor-074/src/diagnostic_original_barred_custody.rs" => {
            strip(raw, RUNTIME)
        }
        "tools/long-run-supervisor-074/src/diagnostic_original_barred_custody_tests.rs" => {
            strip(raw, TESTS)
        }
        _ => raw.to_owned(),
    }
}
fn read(name: &str) -> String {
    std::fs::read_to_string(super::super::root().join(name))
        .unwrap()
        .replace('\r', "")
}
fn closed(raw: &str) -> bool {
    [
        "child.alive()?;",
        "child.observe(alarm, false)?;",
        "self.custody.validate()?;",
        "self.custody.original.inspect_deadline()?;",
        "held.validate()?;",
        "held.custody.original.refuse();",
        "held.ready = true;",
    ]
    .iter()
    .all(|s| raw.contains(s))
}
#[test]
fn barred_collector_joint_frame_is_complete_and_not_recursive() {
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_original_barred_collector.rs");
    assert!(closed(&raw));
    for predicate in [
        "self.custody.validate()?;",
        "child.alive()?;",
        "child.observe(alarm, false)?;",
        "self.custody.original.inspect_deadline()?;",
    ] {
        assert!(!closed(&raw.replace(predicate, "")), "{predicate}");
    }
    for denied in [
        "prepare_barred_with(",
        "plan_descriptors_with(",
        ".send(",
        ".write(",
        "timerfd_settime",
        "pub fn",
        "Deserialize",
        "Clone",
    ] {
        assert!(!raw.contains(denied), "{denied}");
    }
    assert!(
        raw.find("collector: Option<KernelCollector>").unwrap()
            < raw.find("custody: OriginalBarredCustody").unwrap()
    );
    assert!(raw.find("held.validate()?;").unwrap() < raw.find("held.ready = true;").unwrap());
}
#[test]
fn barred_collector_raw_child_closes_io_before_notice_and_retains_fence() {
    let raw =
        read("tools/long-run-supervisor-074/src/diagnostic_original_barred_collector_kernel.rs");
    let child = raw
        .split("fn quarantine(")
        .nth(1)
        .unwrap()
        .split("pub(crate) struct KernelCollector")
        .next()
        .unwrap();
    assert!(child.find("libc::close(fd)").unwrap() < child.find("notice(peer, if closed").unwrap());
    assert!(child.contains("park();"));
    assert!(child.contains("keep: [RawFd; 7]"));
    for denied in [
        "libc::read",
        "libc::write",
        "timerfd_settime",
        "flock",
        "File::",
        "Vec::",
        "exit(",
        "recv(",
    ] {
        assert!(!child.contains(denied), "{denied}");
    }
    for predicate in [
        "libc::close(fd)",
        "quarantine(io, peer, b'L')",
        "libc::SYS_close_range",
        "if n == 1 && initial && byte == b'B'",
    ] {
        assert!(raw.contains(predicate), "{predicate}");
    }
    assert!(raw.contains("let arguments = clone_arguments(&mut pinned);"));
    assert!(
        raw.find("let owned = (pid > 0)").unwrap() < raw.find("let restored = unsafe").unwrap()
    );
    let old = read("tools/long-run-supervisor-074/src/diagnostic_original_barred_custody.rs");
    let stop = |s: &str| {
        s.split("fn stop(")
            .nth(1)
            .unwrap()
            .split("impl Drop for")
            .next()
            .unwrap()
            .to_owned()
    };
    assert_eq!(stop(&raw), stop(&old));
    let observation = |s: &str| {
        s.split("fn alive(")
            .nth(1)
            .unwrap()
            .split("fn stop(")
            .next()
            .unwrap()
            .replace("pub(super) ", "")
            .replace("b'P'", "b'B'")
    };
    assert_eq!(observation(&raw), observation(&old));
    for denied in ["SYS_pidfd_open", "libc::kill(", "SIGTERM", "CLOCK_REALTIME"] {
        assert!(!raw.contains(denied));
    }
}
#[test]
fn barred_collector_selected_fault_oracles_keep_full_frames_and_busy_fence() {
    let raw =
        read("tools/long-run-supervisor-074/src/diagnostic_original_barred_collector_tests.rs");
    for needed in [
        "call_count(&report, Stage::Bind)",
        "call_count(&report, Stage::Checks)",
        "assert_eq!(mask(), parent_mask)",
        "origins[..4]",
        "records(leaf), before",
        "retained_context_refuses_for_test(root)",
        "children.kill_custody_for_test();\n                busy(root);",
        "children.finish_for_test()",
        "failure.retry_for_test()",
        "reason.contains(\"Time\")",
    ] {
        assert!(raw.contains(needed), "{needed}");
    }
    let api =
        read("tools/long-run-supervisor-074/src/diagnostic_original_barred_collector_test_api.rs");
    assert!(api.contains("if count == 3"));
    assert!(api.contains("libc::O_RDWR"));
    assert!(read("docs/COMPAT.md").contains(COMPAT));
}
