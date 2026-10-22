//! Exact reversible ownership-only substitution; never a live-check waiver.
#[path = "performance_contract_074_held_placement.rs"]
mod held_placement;
const BASE: &str = "aa84aeb8";
const PREFIX: &str = "tools/long-run-supervisor-074/src/";
const HOOKS: &[(&str, &str, &str)] = &[
    (
        "diagnostic_kernel_origin.rs",
        r##"fn boot(root: &File) -> Result<String, KernelError> {
    boot_apply(root, boot_value)
}
#[path = "diagnostic_boot_comparison.rs"]
mod boot_comparison;
fn boot_apply<T>(
    root: &File,
    decode: impl FnOnce(([u8; 38], usize)) -> Result<T, KernelError>,
) -> Result<T, KernelError> {
"##,
        r##"fn boot(root: &File) -> Result<String, KernelError> {
"##,
    ),
    (
        "diagnostic_kernel_origin.rs",
        r##"    drop(file);
    decode(raw)
"##,
        r##"    drop(file);
    boot_value(raw)
"##,
    ),
    (
        "diagnostic_kernel_origin.rs",
        r##"fn boot_value(raw: ([u8; 38], usize)) -> Result<String, KernelError> {
    boot_view(&raw).map(str::to_owned)
}
fn boot_view(raw: &([u8; 38], usize)) -> Result<&str, KernelError> {
    let (raw, count) = raw;
    let count = *count;
    if count == 0 {
        return Err(kernel_invalid!("proc-document-budget"));
    }
    if count != 37
        || raw[36] != b'\n'
        || raw[..36].iter().enumerate().any(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                *c != b'-'
            } else {
                !c.is_ascii_digit() && !(b'a'..=b'f').contains(c)
            }
        })
    {
        return Err(kernel_invalid!("boot-document"));
    }
    std::str::from_utf8(&raw[..36]).map_err(|_| kernel_invalid!("boot-utf8"))
}
"##,
        r##"fn boot_value((raw, count): ([u8; 38], usize)) -> Result<String, KernelError> {
    if count == 0 {
        return Err(kernel_invalid!("proc-document-budget"));
    }
    if count != 37
        || raw[36] != b'\n'
        || raw[..36].iter().enumerate().any(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                *c != b'-'
            } else {
                !c.is_ascii_digit() && !(b'a'..=b'f').contains(c)
            }
        })
    {
        return Err(kernel_invalid!("boot-document"));
    }
    String::from_utf8(raw[..36].to_vec()).map_err(|_| kernel_invalid!("boot-utf8"))
}
"##,
    ),
    (
        "diagnostic_kernel_origin.rs",
        r##"!boot_comparison::matches(file, &self.boot)?"##,
        r##"boot(file)? != self.boot"##,
    ),
    (
        "diagnostic_kernel_origin.rs",
        r##"
#[cfg(test)]
#[path = "diagnostic_boot_comparison_tests.rs"]
mod boot_comparison_tests;
"##,
        r##""##,
    ),
    (
        "diagnostic_fixed_leaf_alloc_tests.rs",
        r##"
#[path = "diagnostic_boot_comparison_alloc_tests.rs"]
mod boot_comparison;
"##,
        r##""##,
    ),
];
const HELPER: &str = r##"//! Fresh validated UUID equality without temporary ownership; never a cache.
use super::*;

pub(super) fn matches(root: &File, expected: &str) -> Result<bool, KernelError> {
    boot_apply(root, |raw| view(&raw).map(|value| value == expected))
}
pub(super) fn view(raw: &([u8; 38], usize)) -> Result<&str, KernelError> {
    boot_view(raw)
}
"##;
const OLD_VALUE: &str = r##"fn boot_value((raw, count): ([u8; 38], usize)) -> Result<String, KernelError> {
    if count == 0 {
        return Err(kernel_invalid!("proc-document-budget"));
    }
    if count != 37
        || raw[36] != b'\n'
        || raw[..36].iter().enumerate().any(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                *c != b'-'
            } else {
                !c.is_ascii_digit() && !(b'a'..=b'f').contains(c)
            }
        })
    {
        return Err(kernel_invalid!("boot-document"));
    }
    String::from_utf8(raw[..36].to_vec()).map_err(|_| kernel_invalid!("boot-utf8"))
}
"##;
pub(super) fn normalize(path: &str, raw: &str) -> String {
    let mut text = held_placement::normalize(path, raw);
    for (file, new, old) in HOOKS {
        if path == format!("{PREFIX}{file}") {
            assert_eq!(
                text.matches(new).count(),
                1,
                "exact boot comparison hook: {path}"
            );
            text = text.replacen(new, old, 1);
        }
    }
    text
}
fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &str) -> String {
    let raw = crate::runtime_io::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "");
    crate::hc074_document_scratch_view!(path, &raw)
}
fn git(args: &[&str]) -> String {
    let value = std::process::Command::new("git")
        .args(args)
        .current_dir(root())
        .output()
        .unwrap();
    assert!(value.status.success());
    String::from_utf8(value.stdout).unwrap().replace('\r', "")
}
#[test]
fn boot_comparison_preserves_all_420_previous_sources_and_external_fault_harness() {
    let files = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        BASE,
        "--",
        "tools/long-run-supervisor-074/src",
    ]);
    assert_eq!(files.lines().count(), 420);
    for path in files.lines().chain([
        "docs/COMPAT.md",
        "docs/testing/gated-test-registry.toml",
        "tools/long-run-supervisor-074/tests/owned_root_creator_sigkill.py",
    ]) {
        assert_eq!(
            normalize(path, &read(path)),
            git(&["show", &format!("{BASE}:{path}")]),
            "{path}"
        );
    }
}
#[test]
fn boot_comparison_adapter_rejects_missing_duplicate_and_live_operation_drift() {
    for (file, hook, _) in HOOKS {
        let path = format!("{PREFIX}{file}");
        let raw = read(&path);
        for bad in [raw.replacen(hook, "", 1), format!("{raw}{hook}")] {
            assert!(std::panic::catch_unwind(|| normalize(&path, &bad)).is_err());
        }
    }
    let path = format!("{PREFIX}diagnostic_kernel_origin.rs");
    let raw = read(&path);
    for bad in [
        raw.replace("libc::O_NOFOLLOW | libc::O_CLOEXEC", "libc::O_CLOEXEC"),
        raw.replace("file.metadata()?", "file.metadata() /* drift */?"),
        raw.replace("self.revalidate()?;", "/* skipped live round */"),
        raw.replace("libc::CLOCK_BOOTTIME", "libc::CLOCK_REALTIME"),
    ] {
        assert_ne!(bad, raw);
        assert_ne!(
            normalize(&path, &bad),
            git(&["show", &format!("{BASE}:{path}")])
        );
    }
}
#[test]
fn boot_comparison_helper_and_frozen_reference_preserve_strict_decoder_and_owned_api() {
    assert_eq!(
        read(&format!("{PREFIX}diagnostic_boot_comparison.rs")),
        HELPER
    );
    for forbidden in [
        "unsafe",
        "clock_gettime",
        "timerfd",
        "Command::",
        "static ",
        "thread_local!",
        "Vec<",
        "String",
    ] {
        assert!(!HELPER.contains(forbidden), "{forbidden}");
    }
    let tests = read(&format!("{PREFIX}diagnostic_boot_comparison_tests.rs"));
    let reference = tests
        .split_once("fn reference_value(")
        .unwrap()
        .1
        .split_once("fn pair(")
        .unwrap()
        .0;
    assert_eq!(format!("fn boot_value({reference}"), OLD_VALUE);
    let path = format!("{PREFIX}diagnostic_boot_comparison_alloc_tests.rs");
    let current = read(&path);
    let frozen = git(&["show", &format!("d54c2526:{path}")]);
    assert_eq!(
        current.split_once("#[test]").unwrap().0,
        frozen.split_once("#[test]").unwrap().0
    );
}
