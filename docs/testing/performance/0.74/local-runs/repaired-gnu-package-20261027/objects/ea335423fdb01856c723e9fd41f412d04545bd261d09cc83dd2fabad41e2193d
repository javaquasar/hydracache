//! Exact test-only hooks and a manual local gate, never an old-body exclusion.
#[path = "projection_prefix_guard.rs"]
pub(crate) mod prefix;
pub(crate) const HOOKS: [(&str, &str); 3] = [
    ("tools/long-run-supervisor-074/src/diagnostic_privileged_root_cgroup_fixture.rs", "#[path = \"../tests/support/consuming_held_live.rs\"]\nmod held_live;\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_held_late_trust.rs", "#[cfg(test)]\n#[path = \"../tests/support/consuming_held_live_documents.rs\"]\npub(crate) mod live_fixture;\n"),
    ("tools/long-run-supervisor-074/src/diagnostic_consuming_held_join.rs", "#[cfg(test)]\n#[path = \"../tests/support/consuming_held_live_api.rs\"]\nmod live_api;\n"),
];
pub(crate) const BLOCK: &str = r#"
# BEGIN ACTUAL CONSUMING HELD GATE
[[gate]]
id = "tool.hydracache-long-run-supervisor-074.consuming-held-live"
kind = "external_tool"
source = "tools/long-run-supervisor-074/tests/support/consuming_held_live.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "privileged_consuming_held_live_matrix"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual local root private-namespace actual consuming holder; no send or product"
tier = "manual"
required_features = []
required_env = ["HC074_OWNED_ROOT_FIXTURE", "HC074_START_MATERIAL_BUNDLE"]
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","privileged_consuming_held_live_matrix","--","--ignored","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"
[gate.command.env]
# END ACTUAL CONSUMING HELD GATE
"#;
pub(crate) fn normalize(path: &str, raw: &str) -> String {
    let current = prefix::normalize(path, raw);
    let raw = current.as_str();
    let exact = if path == "docs/testing/gated-test-registry.toml" {
        BLOCK
    } else if let Some((_, hook)) = HOOKS.iter().find(|(file, _)| *file == path) {
        hook
    } else {
        return raw.into();
    };
    assert_eq!(
        raw.matches(exact).count(),
        1,
        "exact actual consuming test hook: {path}"
    );
    raw.replacen(exact, "", 1)
}
