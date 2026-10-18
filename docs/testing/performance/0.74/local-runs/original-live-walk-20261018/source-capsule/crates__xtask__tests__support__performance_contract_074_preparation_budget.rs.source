//! Test-only attribution must preserve every old Rust body and fixture operation.
const BASE: &str = "0847b53f37a0fd79ad64845e53dc38354d99e552";
const REG: &str = "#[path = \"diagnostic_original_preparation_budget_tests.rs\"]\nmod budget;\n";
const GATES: &str = r###"
# BEGIN ORIGINAL PREPARATION BUDGET GATE

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-preparation-budget"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_preparation_budget_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_preparation_budget_attribution"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual nonroot original preparation budget attribution; no dispatch"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::collector_tests::enrollment_tests::manager_tests::addressed_tests::budget::original_preparation_budget_attribution","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

# END ORIGINAL PREPARATION BUDGET GATE
"###;
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let current = crate::live_walk::normalize(path, raw);
    let raw = current.as_str();
    let exact = match path {
        "docs/testing/gated-test-registry.toml" => GATES,
        "tools/long-run-supervisor-074/src/diagnostic_original_manager_message_frame_tests.rs" => {
            REG
        }
        _ => return raw.to_owned(),
    };
    assert_eq!(
        raw.matches(exact).count(),
        1,
        "attribution registration drift"
    );
    raw.replacen(exact, "", 1)
}
fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "")
}
fn compact(raw: &str) -> String {
    raw.split_whitespace().collect()
}
fn old(path: &str) -> String {
    let out = std::process::Command::new("git")
        .args(["show", &format!("{BASE}:{path}")])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap().replace('\r', "")
}
fn helper(raw: &str, name: &str) -> String {
    let start = raw.find(&format!("fn {name}<")).unwrap();
    raw[start..]
        .split("\n#[test]")
        .next()
        .unwrap()
        .trim_end()
        .to_owned()
}
fn restore(raw: &str) -> String {
    let mut raw = helper(raw, "accepted_profiled").replace("fn accepted_profiled<", "fn accepted<");
    for exact in [
        "    timeline: &mut Timeline,\n",
        "    trace::stage(Stage::Setup);\n",
        "    timeline.challenge(challenge.challenge_to_sign());\n",
        "    trace::stage(Stage::Fence);\n",
        "    timeline.mark(\"response-ready\");\n",
        "    trace::stage(Stage::Retain);\n",
        "    timeline.mark(\"consent-prepared\");\n",
        "    trace::stage(Stage::Observe);\n",
    ] {
        assert_eq!(raw.matches(exact).count(), 1);
        raw = raw.replacen(exact, "", 1);
    }
    raw
}
#[test]
fn preparation_budget_preserves_all_existing_rust_sources_and_registered_frame() {
    let out = std::process::Command::new("git")
        .args([
            "ls-tree",
            "-r",
            "--name-only",
            BASE,
            "--",
            "tools/long-run-supervisor-074/src",
        ])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(out.status.success());
    let paths = String::from_utf8(out.stdout).unwrap();
    let paths: Vec<_> = paths.lines().filter(|p| p.ends_with(".rs")).collect();
    assert_eq!(paths.len(), 348);
    for path in paths {
        assert_eq!(normalize(path, &read(path)), old(path), "{path}");
    }
    assert!(read("docs/testing/gated-test-registry.toml").contains(GATES));
}
#[test]
fn preparation_budget_helper_is_the_exact_original_schedule_with_numeric_observation_only() {
    let raw =
        read("tools/long-run-supervisor-074/src/diagnostic_original_preparation_budget_tests.rs");
    let original =
        old("tools/long-run-supervisor-074/src/diagnostic_collector_starting_publication_tests.rs");
    let expected = helper(&original, "accepted");
    assert_eq!(restore(&raw), expected);
    for changed in [
        raw.replace(".accept_response(", ".different_response("),
        raw.replace(".prepare_framed_consent(", ".different_consent("),
        raw.replace(".accept_consent(", ".different_acceptance("),
        raw.replace("let request = original.original.original.request.clone();", "let request = original.original.original.request.clone(); original.revalidate().unwrap();"),
    ] { assert_ne!(restore(&changed), expected); }
    let s = compact(&raw);
    for part in [
        "trace::stage(Stage::Setup);letchallenge=reserved.issue_progress_challenge().unwrap();",
        "timeline.challenge(challenge.challenge_to_sign());trace::stage(Stage::Fence);",
        "timeline.mark(\"response-ready\");trace::stage(Stage::Retain);",
        "timeline.mark(\"consent-prepared\");trace::stage(Stage::Observe);",
        "assert_eq!(origins[1],timeline.issued);",
        "timeline.emit(sample);",
        "libc::CLOCK_BOOTTIME",
        "ifpass==0{\"ok\"}else{\"delivery\"}",
    ] {
        assert!(s.contains(part), "{part}");
    }
    for denied in [
        "OriginalManagerJoin{",
        "OriginalManagerMessage{",
        ".send(",
        "clock_settime",
        "AUTHENTICATION_SECONDS=",
    ] {
        assert!(!s.contains(denied), "{denied}");
    }
}
