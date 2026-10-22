//! Exact test registration only; no runtime or deadline adaptation.
#[path = "performance_contract_074_negotiated_dispatch.rs"]
mod negotiated_dispatch;
const BASE: &str = "290b550e1406c7a8350d845c0c0108312fdc6d3f";
const PATH: &str = "tools/long-run-supervisor-074/src/diagnostic_original_live_walk_cost_tests.rs";
const REG: &str =
    "\n#[path = \"diagnostic_original_plain_worker_tests.rs\"]\nmod plain_worker_tests;\n";
const GATE: &str = r###"
# BEGIN ORIGINAL PLAIN WORKER GATE

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-plain-worker"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_plain_worker_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_plain_worker_batch"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "manual nonroot plain original worker pairs; no reservation or execution"
tier = "manual"
required_features = []
required_env = []
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::leaf_cost_tests::component_cost_tests::live_walk_tests::plain_worker_tests::original_plain_worker_batch","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

# END ORIGINAL PLAIN WORKER GATE
"###;
const FIXTURE: &str = r###"//! Uncaptured retained worker timing only, before reservation.
use super::*;

#[test]
#[ignore = "manual nonroot plain original worker pairs; no reservation or execution"]
fn original_plain_worker_batch() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    with_original_at(false, None, |original, paths, _, _, _| {
        original.revalidate().unwrap();
        let request = original.original.original.request.clone();
        let worker = fs::read(&paths.worker).unwrap();
        let oidc = fs::read(&paths.oidc).unwrap();
        let started = std::time::Instant::now();
        for _ in 0..128 {
            original.original.original.worker.revalidate().unwrap();
        }
        let wall_ns = started.elapsed().as_nanos();
        original.revalidate().unwrap();
        assert_eq!(original.original.original.request, request);
        assert_eq!(fs::read(&paths.worker).unwrap(), worker);
        assert_eq!(fs::read(&paths.oidc).unwrap(), oidc);
        assert!(!original.is_refused());
        assert!(wall_ns > 0);
        println!("HC074-PLAIN-WORKER iterations=128 wall_ns={wall_ns} original=true unchanged=true capture=false reservation=false start=false");
    });
}
"###;
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let current = negotiated_dispatch::normalize(path, raw);
    let raw = current.as_str();
    let insertion = if path == PATH {
        REG
    } else if path == "docs/testing/gated-test-registry.toml" {
        GATE
    } else {
        return raw.to_owned();
    };
    assert_eq!(
        raw.matches(insertion).count(),
        1,
        "plain registration drift"
    );
    raw.replacen(insertion, "", 1)
}
fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &str) -> String {
    crate::runtime_io::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "")
}
fn old(path: &str) -> String {
    let output = std::process::Command::new("git")
        .args(["show", &format!("{BASE}:{path}")])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().replace('\r', "")
}
#[test]
fn plain_worker_preserves_all_old_runtime_and_test_sources_exactly() {
    let output = std::process::Command::new("git")
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
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    let paths: Vec<_> = text.lines().filter(|p| p.ends_with(".rs")).collect();
    assert_eq!(paths.len(), 355);
    for path in paths {
        assert_eq!(normalize(path, &read(path)), old(path), "{path}");
    }
    assert_eq!(
        normalize(
            "docs/testing/gated-test-registry.toml",
            &read("docs/testing/gated-test-registry.toml")
        ),
        old("docs/testing/gated-test-registry.toml")
    );
    assert_eq!(
        read("tools/long-run-supervisor-074/src/diagnostic_original_plain_worker_tests.rs"),
        FIXTURE
    );
}
#[test]
fn plain_worker_has_one_fixed_uncaptured_batch_and_closing_original_checks() {
    assert_eq!(FIXTURE.matches("Instant::now()").count(), 1);
    assert_eq!(FIXTURE.matches("elapsed().as_nanos()").count(), 1);
    assert!(FIXTURE.contains("for _ in 0..128"));
    assert_eq!(
        FIXTURE.matches("original.revalidate().unwrap();").count(),
        2
    );
    assert!(!FIXTURE.contains("capture("));
    assert!(!FIXTURE.contains("env::"));
    let raw = read(PATH);
    let changed = raw.replacen(REG, &format!("{REG}/* mutation */\n"), 1);
    assert_ne!(normalize(PATH, &changed), old(PATH));
    assert!(std::panic::catch_unwind(|| normalize(PATH, &raw.replacen(REG, "", 1))).is_err());
}
