//! Successor keeps the accepted output runtime and trust/time leaves exact.
const GATES: &str = r###"
# BEGIN ORIGINAL START MATERIAL GATES
[[gate]]
id = "cfg.hydracache-long-run-supervisor-074.diagnostic-original-start-material"
kind = "cfg_test_target"
source = "tools/long-run-supervisor-074/tests/diagnostic_original_start_material.rs"
package = "hydracache-long-run-supervisor-074"
target = "diagnostic_original_start_material"
test = ""
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Original installed start material only; no worker execution or deadline change"
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
args = ["test","-p","hydracache-long-run-supervisor-074","--test","diagnostic_original_start_material","--locked"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-start-material-joined-healthy"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_start_material_join_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_start_material_joined_healthy"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Original installed start material only; no worker execution or deadline change"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::original_start_material_joined_healthy","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-start-material-joined-refusals"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_start_material_join_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_start_material_joined_refusals"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Original installed start material only; no worker execution or deadline change"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::original_start_material_joined_refusals","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

# END ORIGINAL START MATERIAL GATES
"###;
const SINK_MODULE: &str =
    "#[path = \"diagnostic_original_start_material.rs\"]\npub mod start_material;\n";
const TEST_MODULE: &str =
    "#[path = \"diagnostic_original_start_material_join_tests.rs\"]\nmod start_material_tests;\n";
const CONTRACT_MODULE: &str =
    "#[path = \"support/performance_contract_074_start_material.rs\"]\nmod start_material;\n";
const NORMALIZER: &str = "    let current = super::start_material::normalize(path, raw);\n    let raw = current.as_str();\n";

pub(super) fn normalize(path: &str, raw: &str) -> String {
    let addition = match path {
        "tools/long-run-supervisor-074/src/diagnostic_original_collector_sinks.rs" => SINK_MODULE,
        "tools/long-run-supervisor-074/src/diagnostic_original_collector_sinks_tests.rs" => {
            TEST_MODULE
        }
        "crates/xtask/tests/performance_contract_074.rs" => CONTRACT_MODULE,
        "crates/xtask/tests/support/performance_contract_074_output_join.rs" => NORMALIZER,
        "docs/testing/gated-test-registry.toml" => GATES,
        _ => return raw.to_owned(),
    };
    assert_eq!(
        raw.matches(addition).count(),
        1,
        "unapproved start-material registration: {path}"
    );
    raw.replacen(addition, "", 1)
}
#[test]
fn original_start_material_preserves_output_runtime_and_closed_execution() {
    let read = |p: &str| {
        std::fs::read_to_string(super::root().join(p))
            .unwrap()
            .replace('\r', "")
    };
    for p in [
        "tools/long-run-supervisor-074/src/diagnostic_original_collector_sinks.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_collector_sinks_tests.rs",
        "crates/xtask/tests/performance_contract_074.rs",
        "crates/xtask/tests/support/performance_contract_074_output_join.rs",
        "docs/testing/gated-test-registry.toml",
    ] {
        let out = std::process::Command::new("git")
            .args(["show", &format!("2c63cac0:{p}")])
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
    let frozen: serde_json::Value = serde_json::from_str(&read("docs/testing/performance/0.74/local-runs/original-collector-sinks-join-20261017/unchanged-runtime.json")).unwrap();
    for row in frozen.as_array().unwrap() {
        let p = row["path"].as_str().unwrap();
        let out = std::process::Command::new("git")
            .args(["show", &format!("2c63cac0:{p}")])
            .current_dir(super::root())
            .output()
            .unwrap();
        assert!(out.status.success());
        assert_eq!(
            read(p),
            String::from_utf8(out.stdout).unwrap().replace('\r', ""),
            "frozen leaf: {p}"
        );
    }
    for p in [
        "tools/long-run-supervisor-074/src/diagnostic_original_output_join.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_collector_sink_storage.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_output_join_crash_tests.rs",
        "tools/long-run-supervisor-074/src/diagnostic_original_output_join_fault_tests.rs",
    ] {
        let out = std::process::Command::new("git")
            .args(["show", &format!("2c63cac0:{p}")])
            .current_dir(super::root())
            .output()
            .unwrap();
        assert!(out.status.success());
        assert_eq!(
            read(p),
            String::from_utf8(out.stdout).unwrap().replace('\r', ""),
            "output proof changed: {p}"
        );
    }
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_original_start_material.rs");
    let runtime = raw
        .lines()
        .filter(|l| !l.trim_start().starts_with("///"))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(runtime.matches(".outputs.material_round()?").count(), 4);
    let compact = runtime.split_whitespace().collect::<String>();
    assert!(
        compact.contains("self.inspect(true)?;self.pipes.join_revalidate()?;self.inspect(true)")
    );
    for marker in [
        "inspect_fixed_install(identity, policy)",
        "held.outputs.material_round()?",
        "held.bind(allow_fixture)?",
        "held.outputs.refuse()",
        "held: Box::new(held)",
        "same_build(&r.original.original.build, bundle.build())?",
        "body.reserved_state_sha256",
        "body.collector_unit_sha256",
        "bundle.revalidate()?",
        "bundle.build().bind_intent(&intent)?",
    ] {
        assert!(runtime.contains(marker), "{marker}");
    }
    for denied in [
        "pub fn start",
        "pub fn into_",
        "pub fn finish",
        "pub fn pump",
        "Command::",
        "TransientUnitSpec",
        "File::open(",
        "SystemTime",
        "SigningKey",
        "O_TRUNC",
        "remove_file(",
        "sync_all(",
        "impl Clone",
    ] {
        assert!(!runtime.contains(denied), "{denied}");
    }
    assert!(std::panic::catch_unwind(|| normalize(
        "tools/long-run-supervisor-074/src/diagnostic_original_collector_sinks.rs",
        &read("tools/long-run-supervisor-074/src/diagnostic_original_collector_sinks.rs")
            .replace("pub mod start_material;", "pub mod unchecked;"),
    ))
    .is_err());
}
