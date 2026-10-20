//! Test input repair only: old runtime, production pins and assertions stay fixed.
use sha2::{Digest, Sha256};
use std::path::PathBuf;

#[path = "support/fixed_slice_namespace_guard.rs"]
mod namespace;

const BASE: &str = "23c57b77523ad1cc16b1a79fc90b02a125f8d9b8";
const NEW_SOURCE: &str = "tools/long-run-supervisor-074/src/diagnostic_held_build.rs";
const LAUNCHER_SOURCES: [&str; 2] = [
    "tools/long-run-supervisor-074/src/bin/diagnostic_held_launcher.rs",
    "tools/long-run-supervisor-074/src/diagnostic_held_launcher_kernel.rs",
];
fn previous_runtime_list(current: &str) -> String {
    let current = namespace::held_build::launcher::roles::binding::previous_list(current);
    let current = namespace::held_build::launcher::roles::previous_list(&current);
    for path in LAUNCHER_SOURCES {
        assert_eq!(
            current.lines().filter(|p| *p == path).count(),
            1,
            "exact new launcher source"
        );
    }
    current
        .lines()
        .filter(|p| !LAUNCHER_SOURCES.contains(p))
        .map(|p| format!("{p}\n"))
        .collect()
}
fn only_new_source(old: &str, current: &str) -> bool {
    let old_set: std::collections::BTreeSet<_> = old.lines().collect();
    let current_set: std::collections::BTreeSet<_> = current.lines().collect();
    old.lines().count() == old_set.len()
        && current.lines().count() == current_set.len()
        && !old_set.contains(NEW_SOURCE)
        && current_set.len() == old_set.len() + 1
        && old_set.is_subset(&current_set)
        && current_set.difference(&old_set).copied().eq([NEW_SOURCE])
}
const FILE: &str = "tools/long-run-supervisor-074/tests/diagnostic_artifacts.rs";
const RECEIPTS: &str = "tools/long-run-supervisor-074/tests/diagnostic_receipts.rs";
const OLD_RECEIPT_LOCK: &str = "include_bytes!(\"../../../Cargo.lock\")";
const NEW_RECEIPT_LOCK: &str = "include_bytes!(\"fixtures/diagnostic-pilot-root-Cargo.lock\")";
const FIXTURE: &str =
    "tools/long-run-supervisor-074/tests/fixtures/diagnostic-pilot-root-Cargo.lock";
const HOOK: &str =
    "\n#[path = \"support/frozen_lock_fixture_tests.rs\"]\nmod frozen_fixture_tests;\n";
const OLD: &str = r##"// Synthetic observer fixtures need its frozen lock, not the newer controller's
// two crypto dependency edges. Reconstruct only that package block and require
// the original fixed SHA. This also works in the builder's shallow checkout.
fn frozen_root_lock() -> &'static [u8] {
    static ORIGINAL: std::sync::LazyLock<Vec<u8>> = std::sync::LazyLock::new(|| {
        let current = include_str!("../../../Cargo.lock").replace("\r\n", "\n");
        let marker = "[[package]]\nname = \"hydracache-long-run-supervisor-074\"";
        let (before, rest) = current.split_once(marker).unwrap();
        let (package, after) = rest.split_once("\n[[package]]").unwrap();
        let package = package
            .replace("\n \"base64\",", "")
            .replace("\n \"ring\",", "");
        let raw = format!("{before}{marker}{package}\n[[package]]{after}").into_bytes();
        assert_eq!(ArtifactDigest::of(&raw).sha256, ROOT_LOCK_SHA256);
        raw
    });
    ORIGINAL.as_slice()
}
"##;
const NEW: &str = r##"// Exact worker-source bytes, independent of moving workspace dependencies.
// This test-only fixture also works offline and in a shallow checkout.
fn frozen_root_lock() -> &'static [u8] {
    const ORIGINAL: &[u8] = include_bytes!("fixtures/diagnostic-pilot-root-Cargo.lock");
    assert_eq!(ArtifactDigest::of(ORIGINAL).sha256, ROOT_LOCK_SHA256);
    ORIGINAL
}
"##;
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "")
}
fn git(args: &[&str]) -> String {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().replace('\r', "")
}
fn normalize(raw: &str) -> String {
    assert_eq!(raw.matches(NEW).count(), 1, "exact immutable helper");
    assert_eq!(raw.matches(HOOK).count(), 1, "exact regression hook");
    raw.replacen(NEW, OLD, 1).replacen(HOOK, "", 1)
}
fn normalize_receipts(raw: &str) -> String {
    assert_eq!(
        raw.matches(NEW_RECEIPT_LOCK).count(),
        1,
        "exact receipt lock input"
    );
    raw.replacen(NEW_RECEIPT_LOCK, OLD_RECEIPT_LOCK, 1)
}
#[test]
fn immutable_fixture_preserves_all_437_runtime_sources_and_production_inputs() {
    let paths = [
        "tools/long-run-supervisor-074/src",
        "Cargo.lock",
        "docs/COMPAT.md",
        "docs/testing/gated-test-registry.toml",
        "docs/testing/performance/0.74/qualification-manifest.toml",
        "tools/long-run-supervisor-074/tests/owned_root_creator_sigkill.py",
    ];
    let old = git(&["ls-tree", "-r", "--name-only", BASE, "--", paths[0]]);
    let current = previous_runtime_list(&git(&["ls-files", "--", paths[0]]));
    assert_eq!(old.lines().count(), 437);
    assert!(only_new_source(&old, &current));
    for path in old.lines().chain(paths[1..].iter().copied()) {
        assert_eq!(
            namespace::normalize(path, &read(path)),
            git(&["show", &format!("{BASE}:{path}")]),
            "old bytes after exact test-only adapter: {path}"
        );
    }
}
#[test]
fn immutable_fixture_is_exact_worker_blob_and_checkout_keeps_raw_lf() {
    let raw = std::fs::read(root().join(FIXTURE)).unwrap();
    assert_eq!(raw.len(), 175300);
    assert!(!raw.contains(&b'\r') && !raw.starts_with(&[0xef, 0xbb, 0xbf]));
    assert_eq!(
        Sha256::digest(&raw)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
        "be46eaf8e97e507bda5e3f2b671d0d622e6a639ce1d4613769a4693723d62b19"
    );
    assert_eq!(
        git(&["hash-object", "--no-filters", FIXTURE]).trim(),
        "59c78f536f3b8ad1d0b13be0f8f1d16b8c892b16"
    );
    assert_eq!(
        git(&["check-attr", "text", "--", FIXTURE]).trim(),
        format!("{FIXTURE}: text: unset")
    );
}
#[test]
fn immutable_fixture_changes_only_old_input_helper_not_any_old_assertion() {
    assert_eq!(
        normalize(&read(FILE)),
        git(&["show", &format!("{BASE}:{FILE}")])
    );
    let receipts = read(RECEIPTS);
    assert_eq!(
        normalize_receipts(&receipts),
        git(&["show", &format!("{BASE}:{RECEIPTS}")])
    );
}
#[test]
fn immutable_fixture_adapter_refuses_missing_duplicate_and_unrelated_test_drift() {
    let raw = read(FILE);
    for exact in [NEW, HOOK] {
        for bad in [raw.replacen(exact, "", 1), format!("{raw}{exact}")] {
            assert!(std::panic::catch_unwind(|| normalize(&bad)).is_err());
        }
    }
    let bad = raw.replacen(
        "assert!(result.stdout.is_empty());",
        "assert!(!result.stdout.is_empty());",
        1,
    );
    assert_ne!(raw, bad);
    assert_ne!(normalize(&bad), git(&["show", &format!("{BASE}:{FILE}")]));
    let receipts = read(RECEIPTS);
    for bad in [
        receipts.replacen(NEW_RECEIPT_LOCK, "", 1),
        format!("{receipts}{NEW_RECEIPT_LOCK}"),
    ] {
        assert!(std::panic::catch_unwind(|| normalize_receipts(&bad)).is_err());
    }
    let changed = receipts.replacen("assert_eq!(", "assert_ne!(", 1);
    assert_ne!(changed, receipts);
    assert_ne!(
        normalize_receipts(&changed),
        git(&["show", &format!("{BASE}:{RECEIPTS}")])
    );
}

#[test]
fn namespace_adapter_refuses_missing_duplicate_and_unrelated_drift() {
    for (path, exact) in [
        (namespace::FILE, namespace::HOOK),
        (namespace::REGISTRY, namespace::GATE),
    ] {
        let raw = read(path);
        for bad in [raw.replacen(exact, "", 1), format!("{raw}{exact}")] {
            assert!(std::panic::catch_unwind(|| namespace::normalize(path, &bad)).is_err());
        }
        assert_ne!(
            namespace::normalize(path, &format!("{raw}\n// unrelated drift\n")),
            git(&["show", &format!("{BASE}:{path}")])
        );
    }
}

#[test]
fn namespace_fixture_is_test_only_private_mount_and_not_manager_authority() {
    let raw = read("tools/long-run-supervisor-074/tests/support/fixed_slice_namespace.rs");
    for exact in [
        "libc::unshare(libc::CLONE_NEWNS)",
        "libc::MS_REC | libc::MS_PRIVATE",
        "FixedSlice::pin(&root, attempt)",
        "drop(fixture)",
        "host_anchors_unchanged=true",
        "original_join=false",
        "observer_readonly(&slice)",
    ] {
        assert!(raw.contains(exact), "{exact}");
    }
    for forbidden in [
        "remove_dir_all",
        "write_at",
        "cgroup.procs",
        "cgroup.kill",
        "StartTransientUnit",
        "systemctl",
        "LOCK_UN",
    ] {
        assert!(!raw.contains(forbidden), "{forbidden}");
    }
    assert_eq!(raw.matches("#[ignore =").count(), 1);
    assert_eq!(raw.matches("/proc/thread-self/ns/mnt").count(), 4);
    assert!(!raw.contains("/proc/self/ns/mnt"));
    assert!(namespace::GATE.contains("ship_mandatory = false"));
}

#[test]
fn held_build_adapter_rejects_missing_duplicate_and_unrelated_old_drift() {
    for (path, exact) in [
        (namespace::held_build::BUILDER, namespace::held_build::HOOK),
        (namespace::held_build::COMPAT, namespace::held_build::BLOCK),
    ] {
        let raw = read(path);
        for bad in [raw.replacen(exact, "", 1), format!("{raw}{exact}")] {
            assert!(std::panic::catch_unwind(|| namespace::normalize(path, &bad)).is_err());
        }
        assert_eq!(
            namespace::normalize(path, &raw),
            git(&["show", &format!("{BASE}:{path}")])
        );
        assert_ne!(
            namespace::normalize(path, &format!("{raw}\n// drift\n")),
            git(&["show", &format!("{BASE}:{path}")])
        );
    }
}
#[test]
fn held_build_source_allowlist_requires_all_old_files_and_exactly_one_named_addition() {
    let old = "old/a.rs\nold/b.rs\n";
    assert!(only_new_source(old, &format!("{old}{NEW_SOURCE}\n")));
    for bad in [
        old.into(),
        format!("old/a.rs\n{NEW_SOURCE}\n"),
        format!("{old}{NEW_SOURCE}\nextra.rs\n"),
        format!("{old}wrong.rs\n"),
        format!("{old}{NEW_SOURCE}\n{NEW_SOURCE}\n"),
    ] {
        assert!(!only_new_source(old, &bad));
    }
}

#[test]
fn launcher_preserves_all_438_preceding_sources_and_exact_old_formats() {
    const BASELINE: &str = "2a86c1fd";
    let old = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        BASELINE,
        "--",
        "tools/long-run-supervisor-074/src",
    ]);
    assert_eq!(old.lines().count(), 438);
    assert_eq!(
        previous_runtime_list(&git(&[
            "ls-files",
            "--",
            "tools/long-run-supervisor-074/src"
        ])),
        old
    );
    for path in old.lines().chain([
        "Cargo.lock",
        "docs/COMPAT.md",
        "docs/testing/gated-test-registry.toml",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ]) {
        assert_eq!(
            namespace::held_build::launcher::normalize(path, &read(path)),
            git(&["show", &format!("{BASELINE}:{path}")]),
            "preceding bytes: {path}"
        );
    }
}
#[test]
fn launcher_adapter_refuses_missing_duplicate_and_unrelated_drift() {
    use namespace::held_build::launcher;
    for (path, exact) in [
        (launcher::COMPAT, launcher::BLOCK),
        (launcher::REGISTRY, launcher::GATE),
    ] {
        let raw = read(path);
        for bad in [raw.replacen(exact, "", 1), format!("{raw}{exact}")] {
            assert!(std::panic::catch_unwind(|| launcher::normalize(path, &bad)).is_err());
        }
        assert_ne!(
            launcher::normalize(path, &format!("{raw}\n// drift\n")),
            git(&["show", &format!("2a86c1fd:{path}")])
        );
    }
    let good = format!(
        "old.rs\n{}\n{}\n{}\n{}\n",
        LAUNCHER_SOURCES[0],
        LAUNCHER_SOURCES[1],
        namespace::held_build::launcher::roles::SOURCE,
        namespace::held_build::launcher::roles::binding::SOURCE
    );
    assert_eq!(previous_runtime_list(&good), "old.rs\n");
    for file in LAUNCHER_SOURCES {
        for bad in [
            good.replacen(&format!("{file}\n"), "", 1),
            format!("{good}{file}\n"),
        ] {
            assert!(std::panic::catch_unwind(|| previous_runtime_list(&bad)).is_err());
        }
    }
}

#[test]
fn role_files_preserve_all_440_previous_runtime_sources_and_inputs() {
    use namespace::held_build::launcher::roles;
    const BASELINE: &str = "773e6bf0";
    let old = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        BASELINE,
        "--",
        "tools/long-run-supervisor-074/src",
    ]);
    assert_eq!(old.lines().count(), 440);
    assert_eq!(
        roles::previous_list(&roles::binding::previous_list(&git(&[
            "ls-files",
            "--",
            "tools/long-run-supervisor-074/src"
        ]))),
        old
    );
    for path in old.lines().chain([
        "Cargo.lock",
        "docs/COMPAT.md",
        "docs/testing/gated-test-registry.toml",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ]) {
        assert_eq!(
            roles::normalize(path, &read(path)),
            git(&["show", &format!("{BASELINE}:{path}")]),
            "preceding bytes: {path}"
        );
    }
}
#[test]
fn role_files_adapter_refuses_missing_duplicate_and_unrelated_drift() {
    use namespace::held_build::launcher::roles;
    let raw = read(roles::FILE);
    for bad in [
        raw.replacen(roles::HOOK, "", 1),
        format!("{raw}{}", roles::HOOK),
    ] {
        assert!(std::panic::catch_unwind(|| roles::normalize(roles::FILE, &bad)).is_err());
    }
    assert_ne!(
        roles::normalize(roles::FILE, &format!("{raw}\n// drift\n")),
        git(&["show", &format!("773e6bf0:{}", roles::FILE)])
    );
    let good = format!("old.rs\n{}\n", roles::SOURCE);
    assert_eq!(roles::previous_list(&good), "old.rs\n");
    for bad in ["old.rs\n".into(), format!("{good}{}\n", roles::SOURCE)] {
        assert!(std::panic::catch_unwind(|| roles::previous_list(&bad)).is_err());
    }
    assert_ne!(
        roles::previous_list(&format!("{good}extra.rs\n")),
        "old.rs\n"
    );
}

#[test]
fn role_binding_preserves_all_441_preceding_runtime_sources_and_inputs() {
    use namespace::held_build::launcher::roles::binding;
    const BASELINE: &str = "697a911a";
    let old = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        BASELINE,
        "--",
        "tools/long-run-supervisor-074/src",
    ]);
    assert_eq!(old.lines().count(), 441);
    assert_eq!(
        binding::previous_list(&git(&[
            "ls-files",
            "--",
            "tools/long-run-supervisor-074/src"
        ])),
        old
    );
    for path in old.lines().chain([
        "Cargo.lock",
        "docs/COMPAT.md",
        "docs/testing/gated-test-registry.toml",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ]) {
        assert_eq!(
            binding::normalize(path, &read(path)),
            git(&["show", &format!("{BASELINE}:{path}")]),
            "preceding bytes: {path}"
        );
    }
}
#[test]
fn role_binding_adapter_refuses_missing_duplicate_and_unrelated_drift() {
    use namespace::held_build::launcher::roles::binding;
    for (path, exact) in [
        (binding::FILE, binding::HOOK),
        (binding::COMPAT, binding::BLOCK),
    ] {
        let raw = read(path);
        for bad in [raw.replacen(exact, "", 1), format!("{raw}{exact}")] {
            assert!(std::panic::catch_unwind(|| binding::normalize(path, &bad)).is_err());
        }
        assert_ne!(
            binding::normalize(path, &format!("{raw}\n// drift\n")),
            git(&["show", &format!("697a911a:{path}")])
        );
    }
    let good = format!("old.rs\n{}\n", binding::SOURCE);
    assert_eq!(binding::previous_list(&good), "old.rs\n");
    for bad in ["old.rs\n".into(), format!("{good}{}\n", binding::SOURCE)] {
        assert!(std::panic::catch_unwind(|| binding::previous_list(&bad)).is_err());
    }
    assert_ne!(
        binding::previous_list(&format!("{good}extra.rs\n")),
        "old.rs\n"
    );
}
