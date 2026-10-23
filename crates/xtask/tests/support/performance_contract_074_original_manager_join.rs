//! Exact additive registration exceptions; original predicates remain frozen.
const GATES: &str = r###"
# BEGIN ORIGINAL MANAGER JOIN LOCAL GATES

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-manager-frame-controls"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_manager_frame_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_manager_join_frame_controls"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual nonroot original frame and distinct test channel controls"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::collector_tests::enrollment_tests::manager_tests::original_manager_join_frame_controls","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-manager-frame-refusals"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_manager_frame_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_manager_join_frame_refusals"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual nonroot original frame live refusals with a distinct test channel"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::collector_tests::enrollment_tests::manager_tests::original_manager_join_frame_refusals","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-manager-nonroot-fixed-refusal"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_manager_frame_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_manager_join_nonroot_fixed_refusal"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual nonroot consuming fixed channel refusal retains original owner"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::collector_tests::enrollment_tests::manager_tests::original_manager_join_nonroot_fixed_refusal","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-manager-root-readonly-context"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_manager_join_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_manager_join_root_readonly_receiver_context"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual local root read-only fixed channel and actual original receiver context"
tier = "manual"
required_features = []
required_env = ["HC074_LOCAL_MANAGER_READONLY"]
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::fenced_preflight::request_attempt::collector_preflight::observed_reservation::publication::progress::causal::start_consent::validation_frame::starting::pipes::sinks::start_material::pipe_transport::grouped_preparation::custody::descriptor_plan::barred_custody::barred_collector::enrollment_frame::manager_channel::original_join::tests::original_manager_join_root_readonly_receiver_context","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

# END ORIGINAL MANAGER JOIN LOCAL GATES
"###;
const COMPAT: &str = r###"
## Private original manager context join

The private consuming join retains the same original enrollment frame and one
fixed live manager channel. Actual retained receiver Origin and typed nsfs
user/mount/PID/time objects are compared with the channel view; caller context
claims cannot construct the production owner. Its light channel observations
enclose one unchanged complete frame, retaining every old live check, absolute
alarm and original deadline. Failure latches and owns the original frame,
children, writers and custody; no retry or former-owner getter exists.

Only new private runtime types are added, not a public/durable codec or start
operation. Separate test channels exercise shared frame ordering without
constructing a fixed owner. Actual read-only channel/context tests do not prove
production trust, message/FD delivery, start, all-writer cessation or completion.
"###;
const CHANNEL_REG: &str =
    "\n#[path = \"diagnostic_original_manager_join.rs\"]\nmod original_join;\n";
const PIN_REG: &str =
    "\n#[path = \"diagnostic_manager_receiver_pin.rs\"]\npub(super) mod receiver;\n";
const ORIGIN_REG: &str = "\n#[path = \"diagnostic_kernel_manager_binding.rs\"]\nmod manager_binding;\npub(crate) use manager_binding::OriginalReceiverBinding;\n";
const PACKET_REG: &str = "pub(crate) use origin::OriginalReceiverBinding;\n";
const TEST_REG: &str =
    "\n#[path = \"diagnostic_original_manager_frame_tests.rs\"]\nmod manager_tests;\n";
const ADAPTER: &str = r###"        // BEGIN ORIGINAL MANAGER KERNEL REFERENCE
        let current = std::fs::read_to_string(super::root().join(&path)).unwrap();
        let current = super::original_start_consent::normalize(&path, &current.replace('\r', ""));
        // END ORIGINAL MANAGER KERNEL REFERENCE
"###;
const OLD_READ: &str =
    "        let current = std::fs::read_to_string(super::root().join(path)).unwrap();\n";
fn strip(raw: &str, exact: &str, restored: &str) -> String {
    assert_eq!(
        raw.matches(exact).count(),
        1,
        "original manager registration drift"
    );
    raw.replacen(exact, restored, 1)
}
const OLD_REFERENCE: &str = r###"        // BEGIN FIXED MANAGER DEPENDENCY REFERENCE
        let current = read(p);
        let current = if matches!(p, "Cargo.lock" | "tools/long-run-supervisor-074/Cargo.toml") {
            super::original_start_consent::normalize(p, &current)
        } else {
            current
        };
        assert_eq!(current, old(p), "out of scope: {p}");
        // END FIXED MANAGER DEPENDENCY REFERENCE
"###;
const EXTENDED_REFERENCE: &str = r###"        // BEGIN FIXED MANAGER DEPENDENCY REFERENCE
        let current = read(p);
        let current = if matches!(p, "Cargo.lock" | "tools/long-run-supervisor-074/Cargo.toml" | "tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs" | "tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs") {
            super::original_start_consent::normalize(p, &current)
        } else {
            current
        };
        assert_eq!(current, old(p), "out of scope: {p}");
        // END FIXED MANAGER DEPENDENCY REFERENCE
"###;
fn restore_reference(raw: &str) -> String {
    let begin = "        // BEGIN FIXED MANAGER DEPENDENCY REFERENCE\n";
    let end = "        // END FIXED MANAGER DEPENDENCY REFERENCE\n";
    let start = raw.find(begin).unwrap();
    let finish = raw.find(end).unwrap() + end.len();
    assert_eq!(
        compact(&raw[start..finish]),
        compact(EXTENDED_REFERENCE),
        "unapproved original manager reference"
    );
    format!("{}{}{}", &raw[..start], OLD_REFERENCE, &raw[finish..])
}
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let exact = match path {
        "crates/xtask/tests/support/performance_contract_074_output_join.rs"
        | "crates/xtask/tests/support/performance_contract_074_starting_publication.rs" => {
            return restore_reference(raw)
        }
        "docs/COMPAT.md" => COMPAT,
        "docs/testing/gated-test-registry.toml" => GATES,
        "tools/long-run-supervisor-074/src/diagnostic_manager_channel.rs" => CHANNEL_REG,
        "tools/long-run-supervisor-074/src/diagnostic_manager_channel_kernel.rs" => PIN_REG,
        "tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs" => ORIGIN_REG,
        "tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs" => PACKET_REG,
        "tools/long-run-supervisor-074/src/diagnostic_original_enrollment_frame_tests.rs" => {
            TEST_REG
        }
        "crates/xtask/tests/support/performance_contract_074_kernel_refusal.rs" => {
            return strip(raw, ADAPTER, OLD_READ)
        }
        _ => return raw.to_owned(),
    };
    strip(raw, exact, "")
}
fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &str) -> String {
    let raw = crate::runtime_io::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "");
    crate::live_walk::normalize(path, &raw)
}
fn compact(raw: &str) -> String {
    raw.split_whitespace().collect()
}
#[test]
fn original_manager_join_preserves_every_original_predicate_and_execution_boundary() {
    for name in [
        "diagnostic_manager_channel.rs",
        "diagnostic_manager_channel_kernel.rs",
        "diagnostic_kernel_origin.rs",
        "diagnostic_kernel_admission.rs",
        "diagnostic_original_enrollment_frame_tests.rs",
        "diagnostic_original_enrollment_frame.rs",
        "diagnostic_original_enrollment_frame_build.rs",
        "diagnostic_original_custody_join.rs",
        "diagnostic_original_grouped_preparation.rs",
        "diagnostic_original_pipe_transport.rs",
        "diagnostic_original_barred_custody.rs",
        "diagnostic_original_barred_collector_kernel.rs",
        "diagnostic_dispatch_exchange.rs",
        "diagnostic_autonomous_watchdog.rs",
    ] {
        let path = format!("tools/long-run-supervisor-074/src/{name}");
        let old = std::process::Command::new("git")
            .args(["show", &format!("3e7a70a8:{path}")])
            .current_dir(root())
            .output()
            .unwrap();
        assert!(old.status.success());
        assert_eq!(
            normalize(&path, &read(&path)),
            String::from_utf8(old.stdout).unwrap().replace('\r', ""),
            "{path}"
        );
    }
    let path = "crates/xtask/tests/support/performance_contract_074_kernel_refusal.rs";
    let old = std::process::Command::new("git")
        .args(["show", &format!("3e7a70a8:{path}")])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(old.status.success());
    assert_eq!(
        normalize(path, &read(path)),
        String::from_utf8(old.stdout).unwrap().replace('\r', "")
    );
}
fn closed(raw: &str) -> bool {
    let s = compact(raw);
    ["original.observe()?;channel.check(receiver,pin)?;original.observe()?;original.revalidate()?;channel.check(receiver,pin)?;original.observe()?;",
     "original.original.refuse();", "FixedManagerChannel::open_fixed()", "retain_receiver(&mutheld.original)?",
     "held:Box::new(held)", "self.ready=false;", "if!original.ready||original.original.is_refused()"].iter().all(|p| s.contains(p))
}
#[test]
fn original_manager_join_full_frame_order_and_latches_have_canaries() {
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_original_manager_join.rs");
    assert!(closed(&raw));
    let raw = compact(&raw);
    for part in [
        "original.revalidate()?;",
        "channel.check(receiver,pin)?;",
        "original.original.refuse();",
        "held:Box::new(held)",
        "self.ready=false;",
    ] {
        assert!(!closed(&raw.replace(part, "")), "{part}");
    }
    for denied in [
        ".send(",
        "sendmsg(",
        "pub fn",
        "Serialize",
        "Deserialize",
        "derive(Clone",
        "into_original",
        "into_channel",
        "Connection::system",
    ] {
        assert!(!raw.contains(denied), "{denied}");
    }
}
#[test]
fn original_manager_join_receiver_uses_retained_origin_and_all_four_typed_objects() {
    let binding = compact(&read(
        "tools/long-run-supervisor-074/src/diagnostic_kernel_manager_binding.rs",
    ));
    for part in [
        "Arc::clone(&self.origin)",
        "self.sender.revalidate(&self.origin)?;",
        "valid_hold(&self.received,&self.origin.sample()?)?",
        "boot!=self.origin.boot",
        "require_namespace(file,*retained,kind)?;",
        "self.refused=true;",
        "libc::CLONE_NEWUSER",
        "libc::CLONE_NEWNS",
        "libc::CLONE_NEWPID",
        "libc::CLONE_NEWTIME",
    ] {
        assert!(binding.contains(part), "{part}");
    }
    let pin = compact(&read(
        "tools/long-run-supervisor-074/src/diagnostic_manager_receiver_pin.rs",
    ));
    assert!(pin.contains("[&ns[5],&self.mount,&ns[2],&ns[6]]"));
    assert!(pin.contains("stamp(&current.mount)"));
    assert!(pin.contains("channel.inspect()"));
    let tests = read("tools/long-run-supervisor-074/src/diagnostic_original_manager_join_tests.rs");
    assert!(!tests.contains("OriginalManagerJoin {"));
    assert!(tests.contains("struct TestChannel"));
    assert!(read("docs/COMPAT.md").contains(COMPAT));
}
