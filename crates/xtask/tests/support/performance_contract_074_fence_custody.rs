//! Exact one-shot custody exception; original inspectors and old contracts freeze.
const OWNER: &str = "// Only Fence can construct this owner. No descriptor or execution API is exposed.\nstruct FenceCustody {\n    _lock: File,\n}\n";
const METHOD: &str = r#"    // Nonexecuting prerequisite only; the original-start join is not enrolled.
    #[allow(dead_code)]
    fn retain_custody(&mut self, parent: &File) -> Result<FenceCustody, OriginalHostFenceError> {
        if self.custody_retained {
            return Err(OriginalHostFenceError::Refused);
        }
        self.inspect(parent)?;
        let lock = self.lock.try_clone()?;
        self.inspect(parent)?;
        // Nothing fallible follows the irreversible selection of close-only drop.
        self.custody_retained = true;
        Ok(FenceCustody { _lock: lock })
    }
"#;
const DROP: &str = r#"        // Short observations retain explicit release. After custody transfer,
        // LOCK_UN would also unlock the guardian's same open-file description.
        if !self.custody_retained {
            let _ = FileExt::unlock(&self.lock);
        }
"#;
const OLD_DROP: &str = "        // Never remove state/markers or unlock a caller-supplied descriptor.\n        let _ = FileExt::unlock(&self.lock);\n";
const TEST_REGISTRATION: &str =
    "\n#[path = \"diagnostic_original_fence_custody_tests.rs\"]\nmod custody;\n";
const REGISTRATION: &str =
    "#[path = \"performance_contract_074_fence_custody.rs\"]\nmod fence_custody;\n";
const ADAPTER: &str =
    "    let current = fence_custody::normalize(path, raw);\n    let raw = current.as_str();\n";
const GATE: &str = r#"
# BEGIN ORIGINAL FENCE CUSTODY GATE
[[gate]]
id = "tool.hydracache-long-run-supervisor-074.original-fence-custody"
kind = "external_tool"
source = "tools/long-run-supervisor-074/src/diagnostic_original_fence_custody_tests.rs"
package = "hydracache-long-run-supervisor-074"
target = "hydracache_long_run_supervisor_074"
test = "original_fence_custody_matrix"
cfg = 'all (target_os = "linux" , target_arch = "x86_64")'
env = ""
reason = "Nonroot original fence ownership, private ACK loss and holder SIGKILL; no authenticated execution"
tier = "manual"
required_features = []
required_env = []
required_tools = ["cargo"]
timeout_seconds = 120
owner_release = "0.74.0"
ship_mandatory = false
artifacts = []

[gate.ci]
workflow = ".github/workflows/ci.yml"
job = "gated-proof-registry"
step = "Run registered gated proofs"

[gate.command]
program = "cargo"
args = ["test","--release","-p","hydracache-long-run-supervisor-074","--lib","--locked","diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::tests::custody::original_fence_custody_matrix","--","--ignored","--exact","--nocapture","--test-threads=1"]
cwd = "."
platform = "linux"

[gate.command.env]
# END ORIGINAL FENCE CUSTODY GATE
"#;
fn once(raw: &str, new: &str, old: &str) -> String {
    assert_eq!(raw.matches(new).count(), 1, "custody exception differs");
    raw.replacen(new, old, 1)
}
pub(super) fn normalize(path: &str, raw: &str) -> String {
    match path {
        "tools/long-run-supervisor-074/src/diagnostic_original_fence.rs" => {
            let mut raw = raw.to_owned();
            for (new, old) in [
                ("    custody_retained: bool,\n", ""),
                ("            custody_retained: false,\n", ""),
                (OWNER, ""),
                (METHOD, ""),
                (DROP, OLD_DROP),
            ] {
                raw = once(&raw, new, old);
            }
            raw
        }
        "tools/long-run-supervisor-074/src/diagnostic_original_fence_tests.rs" => {
            once(raw, TEST_REGISTRATION, "")
        }
        "crates/xtask/tests/support/performance_contract_074_fixed_grouped.rs" => {
            once(&once(raw, REGISTRATION, ""), ADAPTER, "")
        }
        "docs/testing/gated-test-registry.toml" => once(raw, GATE, ""),
        _ => raw.to_owned(),
    }
}
fn old(path: &str) -> String {
    let out = std::process::Command::new("git")
        .args(["show", &format!("09b43743:{path}")])
        .current_dir(super::super::root())
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap().replace('\r', "")
}
#[test]
fn original_custody_changes_only_one_shot_ownership_not_admission_or_old_guards() {
    let path = "tools/long-run-supervisor-074/src/diagnostic_original_fence.rs";
    let raw = super::read(path);
    for p in [
        path,
        "tools/long-run-supervisor-074/src/diagnostic_original_fence_tests.rs",
        "crates/xtask/tests/support/performance_contract_074_fixed_grouped.rs",
        "docs/testing/gated-test-registry.toml",
    ] {
        assert_eq!(normalize(p, &super::read(p)), old(p), "{p}");
    }
    for (from, to) in [
        ("if !self.custody_retained {", "if self.custody_retained {"),
        ("custody_retained: false,", "custody_retained: true,"),
        ("self.inspect(parent)?;", "let _ = parent;"),
        ("self.lock.try_clone()?", "File::open(\"/tmp/foreign\")?"),
    ] {
        assert!(std::panic::catch_unwind(|| normalize(path, &raw.replace(from, to))).is_err());
    }
    assert_ne!(
        normalize(
            path,
            &raw.replace("lock_stamp.len != 0", "lock_stamp.len > 1")
        ),
        old(path)
    );
    assert_ne!(
        normalize(path, &format!("{raw}\nfn unapproved_tail() {{}}\n")),
        old(path)
    );
    // No production caller can claim that this prerequisite enrolled a guardian.
    for entry in
        std::fs::read_dir(super::super::root().join("tools/long-run-supervisor-074/src")).unwrap()
    {
        let file = entry.unwrap().path();
        if file.extension().and_then(|v| v.to_str()) != Some("rs") {
            continue;
        }
        let name = file.file_name().unwrap().to_str().unwrap();
        if name == "diagnostic_original_fence_custody_tests.rs"
            || name == "diagnostic_original_fence.rs"
        {
            continue;
        }
        assert!(!std::fs::read_to_string(file)
            .unwrap()
            .contains("retain_custody("));
    }
    assert_eq!(raw.matches("fn retain_custody(").count(), 1);
    assert!(!raw.contains("pub fn retain_custody"));
    assert!(!raw.contains("pub(crate) fn retain_custody"));
    let tests =
        super::read("tools/long-run-supervisor-074/src/diagnostic_original_fence_custody_tests.rs");
    for required in [
        "ordinary_drop_still_unlocks_a_retained_alias",
        "custody_survives_original_drop",
        "custody_first_drop_keeps_original_checks_and_refuses_a_second_transfer",
        "custody_cannot_adopt_a_replaced_named_lock",
        "original_fence_custody_matrix",
        "repeat in 1..=3",
        "libc::SYS_pidfd_send_signal",
        "libc::WTERMSIG(status), libc::SIGKILL",
    ] {
        assert!(tests.contains(required), "{required}");
    }
}
