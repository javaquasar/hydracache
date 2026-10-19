//! One optional fixed-path substitution; all live leaf operations stay exact.
const BASE: &str = "a8a07600";
const ORIGIN: &str = "tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs";
const TEST: &str = "\n#[cfg(test)]\n#[path = \"diagnostic_fixed_leaf_path_tests.rs\"]\nmod fixed_leaf_path_tests;\n";
const BOOT_OLD: &str = "    let sys = child(root, \"sys\", true)?;\n    let kernel = child(&sys, \"kernel\", true)?;\n    let random = child(&kernel, \"random\", true)?;\n    let file = child(&random, \"boot_id\", false)?;";
const BOOT_NEW: &str = "    let sys = fixed_child(root, c\"sys\", true)?;\n    let kernel = fixed_child(&sys, c\"kernel\", true)?;\n    let random = fixed_child(&kernel, c\"random\", true)?;\n    let file = fixed_child(&random, c\"boot_id\", false)?;";
const NS_OLD: &str = ".open(format!(\"/proc/thread-self/ns/{name}\"))?;";
const NS_NEW: &str = ".open(namespace_path(name).as_ref())?;";
const HELPERS: &str = r#"
fn fixed_child(parent: &File, name: &std::ffi::CStr, directory: bool) -> Result<File, KernelError> {
    let flags = libc::O_RDONLY
        | libc::O_NOFOLLOW
        | libc::O_CLOEXEC
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 };
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let file = unsafe { File::from_raw_fd(fd) };
    filesystem(&file, 0x9fa0)?;
    Ok(file)
}
fn namespace_path(name: &str) -> std::borrow::Cow<'static, str> {
    use std::borrow::Cow;
    match name {
        "user" => Cow::Borrowed("/proc/thread-self/ns/user"),
        "mnt" => Cow::Borrowed("/proc/thread-self/ns/mnt"),
        "pid" => Cow::Borrowed("/proc/thread-self/ns/pid"),
        "time" => Cow::Borrowed("/proc/thread-self/ns/time"),
        _ => Cow::Owned(format!("/proc/thread-self/ns/{name}")),
    }
}
"#;
const GATE: &str = r#"
# BEGIN FIXED LEAF PATH GATE

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.fixed-leaf-path-cost"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_path_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "fixed_leaf_path_cost_abba"
cfg = 'target_os = "linux"'
env = ""
reason = "manual paired nonexecuting kernel leaf reads; never product or admission"
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","fixed_leaf_path_cost_abba","--","--ignored","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.fixed-leaf-path-single"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_path_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "fixed_leaf_path_single_cost"
cfg = 'target_os = "linux"'
env = ""
reason = "manual actual leaf for external image crossover; no admission or product"
tier = "manual"
required_features = []
required_env = ["HC074_LEAF_SINGLE"]
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","fixed_leaf_path_single_cost","--","--ignored","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

[[gate]]
id = "tool.hydracache-long-run-supervisor-074.fixed-leaf-allocation"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "fixed_leaf_path_allocation_attribution"
cfg = 'all (target_os = "linux" , feature = "fixed-leaf-alloc-probe")'
env = ""
reason = "manual opt-in Rust allocation requests; no latency or admission claim"
tier = "manual"
required_features = ["fixed-leaf-alloc-probe"]
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
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--features","fixed-leaf-alloc-probe","--lib","--locked","fixed_leaf_path_allocation_attribution","--","--ignored","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]

# END FIXED LEAF PATH GATE
"#;
fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "")
}
fn enabled() -> bool {
    let doc: toml::Value = toml::from_str(&read(
        "docs/testing/performance/0.74/fixed-leaf-path-contract.toml",
    ))
    .unwrap();
    doc["candidate_enabled"].as_bool().unwrap()
}
fn clean(path: &str, raw: &str, candidate: bool) -> String {
    let mut text = raw.to_owned();
    if path == "tools/long-run-supervisor-074/Cargo.toml" {
        let feature = "[features]\nfixed-leaf-alloc-probe = []\n\n";
        assert_eq!(text.matches(feature).count(), 1);
        text = text.replacen(feature, "", 1);
    }
    if path == ORIGIN {
        assert_eq!(text.matches(TEST).count(), 1);
        text = text.replacen(TEST, "", 1);
        if candidate {
            for (new, old) in [(BOOT_NEW, BOOT_OLD), (NS_NEW, NS_OLD), (HELPERS, "")] {
                assert_eq!(text.matches(new).count(), 1, "exact leaf candidate");
                text = text.replacen(new, old, 1);
            }
        } else {
            assert_eq!(text.matches(BOOT_OLD).count(), 1);
            assert_eq!(text.matches(NS_OLD).count(), 1);
            assert!(!text.contains("fn fixed_child("));
        }
    }
    if path == "docs/testing/gated-test-registry.toml" {
        assert_eq!(text.matches(GATE).count(), 1);
        text = text.replacen(GATE, "", 1);
    }
    text
}
pub(super) fn normalize(path: &str, raw: &str) -> String {
    clean(path, raw, path == ORIGIN && enabled())
}
fn git(args: &[&str]) -> String {
    let result = std::process::Command::new("git")
        .args(args)
        .current_dir(root())
        .output()
        .unwrap();
    assert!(result.status.success());
    String::from_utf8(result.stdout).unwrap().replace('\r', "")
}
#[test]
fn fixed_leaf_path_preserves_previous_sources_protocols_and_all_live_operations() {
    let files = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        BASE,
        "--",
        "tools/long-run-supervisor-074/src",
    ]);
    assert_eq!(files.lines().count(), 404);
    for path in files.lines().chain([
        "tools/long-run-supervisor-074/Cargo.toml",
        "docs/COMPAT.md",
        "docs/testing/gated-test-registry.toml",
        "tools/long-run-supervisor-074/tests/prearmed_watchdog.py",
        "tools/long-run-supervisor-074/tests/redundant_watchdog.py",
        "tools/long-run-supervisor-074/tests/root_creator_sigkill.py",
    ]) {
        assert_eq!(
            normalize(path, &read(path)),
            git(&["show", &format!("{BASE}:{path}")]),
            "{path}"
        );
    }
    let raw = read("tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_path_tests.rs");
    let origin = git(&["show", &format!("{BASE}:{ORIGIN}")]);
    for (name, end) in [
        ("namespace", "#[path = \"diagnostic_boot_document.rs\"]"),
        ("boot", "fn boot_read("),
    ] {
        let old = origin
            .split_once(&format!("fn {name}("))
            .unwrap()
            .1
            .split_once(end)
            .unwrap()
            .0;
        let reference = raw.split_once(&format!("fn reference_{name}(")).unwrap().1;
        assert!(
            reference.starts_with(old.trim_end()),
            "frozen reference {name}"
        );
    }
}
#[test]
fn fixed_leaf_path_adapter_rejects_changed_operations_and_missing_hooks() {
    let old = git(&["show", &format!("{BASE}:{ORIGIN}")]);
    let trial = old
        .replacen(BOOT_OLD, BOOT_NEW, 1)
        .replacen(NS_OLD, NS_NEW, 1)
        + HELPERS
        + TEST;
    assert_eq!(clean(ORIGIN, &trial, true), old);
    for extra in [BOOT_NEW, NS_NEW, HELPERS, TEST] {
        assert!(
            std::panic::catch_unwind(|| clean(ORIGIN, &trial.replacen(extra, "", 1), true))
                .is_err()
        );
        assert!(
            std::panic::catch_unwind(|| clean(ORIGIN, &format!("{trial}{extra}"), true)).is_err()
        );
    }
    assert_ne!(
        clean(
            ORIGIN,
            &trial.replace("file.metadata()?", "file.metadata() /* drift */?"),
            true
        ),
        old
    );
}
