//! Remove only the exact opt-in test hook and its manual registry entry.
#[path = "held_build_guard.rs"]
pub(super) mod held_build;
pub(super) const FILE: &str = "tools/long-run-supervisor-074/src/diagnostic_fixed_slice_tests.rs";
pub(super) const HOOK: &str =
    "\n#[path = \"../tests/support/fixed_slice_namespace.rs\"]\nmod namespace;\n";
pub(super) const REGISTRY: &str = "docs/testing/gated-test-registry.toml";
pub(super) const GATE: &str = r##"
# BEGIN FIXED SLICE NAMESPACE GATE
[[gate]]
id = "tool.hydracache-long-run-supervisor-074.fixed-slice-namespace"
kind = "external_tool"
source = "tools/long-run-supervisor-074/tests/support/fixed_slice_namespace.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "privileged_fixed_slice_namespace_matrix"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual root isolated mount and temporary cgroup2 fixture; not original or systemd admission"
tier = "manual"
required_features = []
required_env = ["HC074_FIXED_SLICE_NAMESPACE"]
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","privileged_fixed_slice_namespace_matrix","--","--ignored","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"
[gate.command.env]
HC074_FIXED_SLICE_NAMESPACE = "1"
# END FIXED SLICE NAMESPACE GATE
"##;
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let raw = held_build::normalize(path, raw);
    let exact = match path {
        FILE => HOOK,
        REGISTRY => GATE,
        _ => return raw,
    };
    assert_eq!(
        raw.matches(exact).count(),
        1,
        "exact namespace test hook: {path}"
    );
    raw.replacen(exact, "", 1)
}
