//! Exact additive boundary; existing guards, wire signatures and transport stay frozen.
const GATES: &str = r###"
# BEGIN ORIGINAL PIPE TRANSPORT GATES
[[gate]]
id = "cfg.hydracache-long-run-supervisor-074.original-pipe-transport-api"
kind = "cfg_test_target"
source = "tools/long-run-supervisor-074/tests/diagnostic_original_pipe_transport.rs"
package = "hydracache-long-run-supervisor-074"
target = "diagnostic_original_pipe_transport"
test = ""
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Original unsent pipe message only; no bus, watchdog admission or execution"
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
args = ["test","-p","hydracache-long-run-supervisor-074","--test","diagnostic_original_pipe_transport","--locked"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-pipe-transport-healthy"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_pipe_transport_join_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_pipe_transport_joined_healthy"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Original unsent pipe message only; no bus, watchdog admission or execution"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::original_pipe_transport_joined_healthy","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-pipe-transport-refusals"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_pipe_transport_join_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_pipe_transport_joined_refusals"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Original unsent pipe message only; no bus, watchdog admission or execution"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::original_pipe_transport_joined_refusals","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

# END ORIGINAL PIPE TRANSPORT GATES
"###;
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let current = super::safety_alarm::normalize(path, raw);
    let raw = current.as_str();
    let addition = match path {
        "tools/long-run-supervisor-074/src/diagnostic_original_start_material.rs" =>
            "#[path = \"diagnostic_original_pipe_transport.rs\"]\npub mod pipe_transport;\n",
        "tools/long-run-supervisor-074/src/diagnostic_original_start_material_join_tests.rs" =>
            "#[path = \"diagnostic_original_pipe_transport_join_tests.rs\"]\nmod pipe_transport_tests;\n",
        "crates/xtask/tests/performance_contract_074.rs" =>
            "\n#[path = \"support/performance_contract_074_pipe_transport.rs\"]\nmod pipe_transport;\n",
        "crates/xtask/tests/support/performance_contract_074_start_material.rs" =>
            "    let current = super::pipe_transport::normalize(path, raw);\n    let raw = current.as_str();\n",
        "docs/testing/gated-test-registry.toml" => GATES,
        _ => return raw.to_owned(),
    };
    assert_eq!(
        raw.matches(addition).count(),
        1,
        "unexpected pipe transport registration: {path}"
    );
    raw.replacen(addition, "", 1)
}

#[test]
fn original_pipe_transport_preserves_material_and_legacy_transport() {
    let read = |p: &str| {
        std::fs::read_to_string(super::root().join(p))
            .unwrap()
            .replace('\r', "")
    };
    for p in [
        "tools/long-run-supervisor-074/src/diagnostic_original_start_material.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_start_material_join_tests.rs",
        "crates/xtask/tests/performance_contract_074.rs",
        "crates/xtask/tests/support/performance_contract_074_start_material.rs",
        "docs/testing/gated-test-registry.toml",
        "tools/long-run-supervisor-074/src/systemd_unit.rs",
        "tools/long-run-supervisor-074/src/systemd_spawn.rs",
        "tools/long-run-supervisor-074/src/diagnostic_unit.rs",
        "tools/long-run-supervisor-074/src/diagnostic_collector_intent.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_collector_pipes.rs",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ] {
        let out = std::process::Command::new("git")
            .args(["show", &format!("fa680f3d:{p}")])
            .current_dir(super::root())
            .output()
            .unwrap();
        assert!(out.status.success(), "{p}");
        assert_eq!(
            normalize(p, &read(p)),
            String::from_utf8(out.stdout).unwrap().replace('\r', ""),
            "{p}"
        );
    }
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_original_pipe_transport.rs");
    let runtime = raw
        .lines()
        .filter(|l| !l.trim_start().starts_with("///"))
        .collect::<Vec<_>>()
        .join("\n");
    for denied in [
        "Connection",
        "Proxy",
        ".send(",
        "pub fn start",
        "pub fn into_",
        "impl Clone",
        "SystemTime",
        "sync_all(",
        "Command::",
        "try_clone(",
    ] {
        assert!(!runtime.contains(denied), "{denied}");
    }
    for marker in [
        "StandardOutputFileDescriptor",
        "StandardErrorFileDescriptor",
        "Value::from(Fd::from(stdout))",
        "self.material.bind(allow_fixture)?",
        "inspect_message(",
        "self.material.outputs.refuse()",
        "held: Box::new(held)",
    ] {
        assert!(runtime.contains(marker), "{marker}");
    }
    assert!(std::panic::catch_unwind(|| normalize(
        "tools/long-run-supervisor-074/src/diagnostic_original_start_material.rs",
        &read("tools/long-run-supervisor-074/src/diagnostic_original_start_material.rs")
            .replace("pub mod pipe_transport;", "pub mod unchecked;")
    ))
    .is_err());
}
