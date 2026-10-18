//! Only the declared borrowed inventory exception; original runtime remains exact.
const GATES: &str = r###"
# BEGIN ORIGINAL DESCRIPTOR PLAN GATES
[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-descriptors-screen"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_descriptor_plan_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_descriptor_plan_screen"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Original descriptor inventory and test-only allowlist holder; no production execution"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::original_descriptor_plan_screen","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-descriptors-healthy"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_descriptor_plan_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_descriptor_plan_healthy"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Original descriptor inventory and test-only allowlist holder; no production execution"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::original_descriptor_plan_healthy","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-descriptors-refusals"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_descriptor_plan_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_descriptor_plan_refusals"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Original descriptor inventory and test-only allowlist holder; no production execution"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::original_descriptor_plan_refusals","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-descriptors-later-refusals"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_descriptor_plan_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_descriptor_plan_later_refusals"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Original descriptor inventory and test-only allowlist holder; no production execution"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::original_descriptor_plan_later_refusals","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-descriptors-holder-matrix"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_descriptor_plan_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_descriptor_holder_matrix"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Original descriptor inventory and test-only allowlist holder; no production execution"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::original_descriptor_holder_matrix","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

# END ORIGINAL DESCRIPTOR PLAN GATES
"###;
const SELF_REG: &str =
    "#[path = \"performance_contract_074_original_descriptors.rs\"]\nmod original_descriptors;\n";
const ADAPTER: &str = "    let current = original_descriptors::normalize(path, raw);\n    let raw = current.as_str();\n";
const REG: &str = "#[allow(dead_code)]\n#[path = \"diagnostic_original_descriptor_plan.rs\"]\nmod descriptor_plan;\n";
const TEST_REG: &str =
    "#[path = \"diagnostic_original_descriptor_plan_tests.rs\"]\nmod descriptor_plan_tests;\n";
const ACCESSOR: &str = r#"    // Borrow only this checked original description; never clone or extract it.
    pub(crate) fn original_descriptor(
        &self,
        storage: &CollectorReservationStorage<'_>,
    ) -> Result<BorrowedFd<'_>, StorageAttemptError> {
        self.inspect(storage)?;
        Ok(self.original._lock.as_fd())
    }
"#;
fn once(raw: &str, new: &str, old: &str) -> String {
    assert_eq!(
        raw.matches(new).count(),
        1,
        "original descriptor exception differs: {new:?}"
    );
    raw.replacen(new, old, 1)
}
pub(super) fn normalize(path: &str, raw: &str) -> String {
    match path {
        "tools/long-run-supervisor-074/src/diagnostic_original_custody_join.rs" => {
            once(raw, REG, "")
        }
        "tools/long-run-supervisor-074/src/diagnostic_original_custody_join_tests.rs" => {
            once(raw, TEST_REG, "")
        }
        "tools/long-run-supervisor-074/src/diagnostic_original_collector_custody_storage.rs" => {
            once(
                &once(raw, ACCESSOR, ""),
                "use std::os::fd::{AsFd, AsRawFd, BorrowedFd};",
                "use std::os::fd::AsRawFd;",
            )
        }
        "crates/xtask/tests/support/performance_contract_074_custody_join.rs" => {
            once(&once(raw, SELF_REG, ""), ADAPTER, "")
        }
        "docs/testing/gated-test-registry.toml" => once(raw, GATES, ""),
        _ => raw.to_owned(),
    }
}
fn root() -> std::path::PathBuf {
    super::super::super::super::root()
}
fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "")
}
fn frozen(path: &str) -> String {
    let out = std::process::Command::new("git")
        .args(["show", &format!("422aa5f2:{path}")])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap().replace('\r', "")
}
#[test]
fn original_descriptor_plan_preserves_all_old_bodies_and_generic_guardian() {
    for path in [
        "tools/long-run-supervisor-074/src/diagnostic_original_custody_join.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_custody_join_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_collector_custody_storage.rs",
        "crates/xtask/tests/support/performance_contract_074_custody_join.rs",
        "docs/testing/gated-test-registry.toml",
        "tools/long-run-supervisor-074/src/diagnostic_original_collector_sinks.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_collector_pipes.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_grouped_preparation.rs",
        "tools/long-run-supervisor-074/src/diagnostic_autonomous_watchdog.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_fence.rs",
        "tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs",
        "tools/long-run-supervisor-074/src/diagnostic_lease.rs",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ] {
        assert_eq!(normalize(path, &read(path)), frozen(path), "{path}");
    }
    let path = "tools/long-run-supervisor-074/src/diagnostic_original_custody_join.rs";
    let raw = read(path);
    for (from, to) in [
        (
            "self.original.validate(allow_fixture)?;",
            "let _ = allow_fixture;",
        ),
        ("self.inspect_deadline()", "Ok(())"),
        (
            "held.original.original_round(allow_fixture)? != bounds",
            "false",
        ),
        ("clock()?", "0"),
    ] {
        assert_ne!(normalize(path, &raw.replace(from, to)), frozen(path));
    }
    assert_ne!(
        normalize(path, &format!("{raw}\nfn unapproved() {{}}\n")),
        frozen(path)
    );
    let path = "tools/long-run-supervisor-074/src/diagnostic_original_collector_custody_storage.rs";
    assert!(
        std::panic::catch_unwind(|| normalize(path, &read(path).replace(ACCESSOR, ""))).is_err()
    );
    assert_ne!(
        normalize(
            path,
            &read(path).replace("self.revalidate()?;", "let _ = ();")
        ),
        frozen(path)
    );
}
fn enrolled(raw: &str) -> bool {
    raw.contains("if position != 0 {\n            return Err(OriginalCustodyError::Binding);")
        && raw.contains("original:\n        &'held mut OriginalCustodiedPreparation")
        && raw.contains("let inventory = capture(self)?;")
        && raw.contains("self.validate(allow_fixture)?;\n            if capture(self)? != inventory")
        && raw.contains("self.inspect_deadline()?;\n            Ok(inventory)")
        && raw.contains("self.original.validate(self.allow_fixture)?;\n            if capture(self.original)? != self.inventory")
        && raw.contains("self.original.inspect_deadline()")
        && raw.contains("self.plan_descriptors_with(\n            false,")
        && raw.matches("self.refuse();").count() == 1
        && raw.matches("self.original.refuse();").count() == 1
}
#[test]
fn original_descriptor_plan_borrows_exact_objects_and_closes_with_full_validation() {
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_original_descriptor_plan.rs");
    assert!(enrolled(&raw));
    for (from, to) in [
        ("if position != 0", "if false"),
        ("self.validate(allow_fixture)?;", "let _ = allow_fixture;"),
        ("if capture(self)? != inventory", "if false"),
        ("self.inspect_deadline()?;", "let _ = ();"),
        (
            "self.original.validate(self.allow_fixture)?;",
            "let _ = ();",
        ),
        ("self.original.inspect_deadline()", "Ok(())"),
    ] {
        assert!(!enrolled(&raw.replace(from, to)), "{from}");
    }
    for required in [
        "custody.original_descriptor(outputs.reservation())?",
        "message.data()",
        "aliases.len() != 2",
        "libc::F_GETFD",
        "libc::F_GETFL",
        "libc::fstat",
        "libc::SEEK_CUR",
    ] {
        assert!(raw.contains(required), "{required}");
    }
    for denied in [
        "pub fn ",
        "Command::",
        "libc::write",
        "libc::fork",
        "timerfd_create",
        "File::open",
        "CLOCK_REALTIME",
        "impl Clone",
        "impl Deserialize",
        "impl AsRawFd",
    ] {
        assert!(!raw.contains(denied), "{denied}");
    }
    assert_eq!(raw.matches("/// \x60\x60\x60compile_fail").count(), 2);
    assert_eq!(raw.matches(".try_clone()").count(), 1);
    assert!(raw.contains("#[cfg(test)]\n    pub(crate) fn hidden_writer_for_test"));
    let bridge =
        read("tools/long-run-supervisor-074/src/diagnostic_original_collector_custody_storage.rs");
    assert_eq!(bridge.matches(ACCESSOR).count(), 1);
    let test =
        read("tools/long-run-supervisor-074/src/diagnostic_original_descriptor_plan_tests.rs");
    assert!(test.contains("authentication_calls, 34"));
    assert!(test.contains("signature_calls, 34"));
    assert!(test.contains("!reason.contains(\"Time\")"));
    assert!(test.contains("Original(Expired)"));
}
#[test]
fn original_descriptor_holder_remains_a_bounded_nonexecuting_owned_fixture() {
    let raw =
        read("tools/long-run-supervisor-074/src/diagnostic_original_descriptor_holder_tests.rs");
    let child = raw
        .split("fn child(")
        .nth(1)
        .unwrap()
        .split("impl Holder")
        .next()
        .unwrap();
    for denied in [
        "assert!",
        "unwrap(",
        "File::",
        "std::io",
        "Command::",
        "Vec::",
        "format!",
        "println!",
        "libc::write",
    ] {
        assert!(!child.contains(denied), "{denied}");
    }
    for required in [
        "SYS_close_range",
        "&fds[6..10]",
        "libc::EBADF",
        "&fds[2..4]",
        "n != 0",
        "b'E'",
        "libc::fsync",
        "libc::close",
        "libc::poll(&mut timer, 1, 0)",
        "b'T'",
    ] {
        assert!(child.contains(required), "{required}");
    }
    assert!(raw.find("libc::SIG_SETMASK,").unwrap() < raw.find("libc::SYS_fork").unwrap());
    assert!(raw.contains("Some(libc::EAGAIN)"));
    assert!(raw.contains("libc::SYS_pidfd_open"));
    assert!(raw.contains("libc::SYS_pidfd_send_signal"));
    assert!(raw.contains("libc::WTERMSIG(status), libc::SIGKILL"));
    let cases =
        read("tools/long-run-supervisor-074/src/diagnostic_original_descriptor_plan_tests.rs");
    assert!(cases.contains("repeat in 1..=3"));
    assert!(cases.contains("[\"normal\", \"ack-lost\", \"holder-kill\", \"hidden-writer\"]"));
    assert!(cases.find("Holder::spawn(").unwrap() < cases.rfind("drop(held);").unwrap());
    assert!(cases.contains("child.assert_retained();"));
    assert!(cases.contains("retained_context_refuses_for_test(root)"));
    for (from, to) in [
        ("repeat in 1..=3", "repeat in 1..=1"),
        ("libc::SIGKILL", "libc::SIGTERM"),
        ("n != 0", "false"),
    ] {
        assert_ne!(
            raw.replace(from, to) + &cases.replace(from, to),
            raw.clone() + &cases
        );
    }
}
