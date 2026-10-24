#[path = "support/boot_operations_guard.rs"]
mod guard;
#[path = "support/authority_history_io.rs"]
mod history;
fn read(path: &str) -> String {
    history::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path),
    )
    .unwrap()
    .replace('\r', "")
}
fn old(path: &str) -> String {
    let result = history::command()
        .args(["show", &format!("c80a3fb8:{path}")])
        .output()
        .unwrap();
    assert!(result.status.success());
    String::from_utf8(result.stdout).unwrap().replace('\r', "")
}
fn body(source: &str, name: &str) -> String {
    use proc_macro2::{Delimiter, TokenStream, TokenTree};
    let tokens: Vec<_> = source.parse::<TokenStream>().unwrap().into_iter().collect();
    let matches: Vec<_> = (0..tokens.len() - 1)
        .filter(|&i| tokens[i].to_string() == "fn" && tokens[i + 1].to_string() == name)
        .map(|i| {
            tokens[i + 2..]
                .iter()
                .find_map(|token| match token {
                    TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => {
                        Some(g.stream().to_string())
                    }
                    _ => None,
                })
                .unwrap()
        })
        .collect();
    assert_eq!(matches.len(), 1);
    matches[0].clone()
}
#[test]
fn boot_operations_only_exact_hook_gate_and_directory_ownership_may_normalize() {
    for (path, marker) in [
        (
            "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs",
            "mod boot_operations;",
        ),
        (
            "docs/testing/gated-test-registry.toml",
            "# BEGIN BOOT OPERATIONS GATE",
        ),
    ] {
        let raw = read(path);
        assert_eq!(crate::hc074_boot_operations_view!(path, &raw), old(path));
        for bad in [raw.replacen(marker, "changed", 1), format!("{raw}{raw}")] {
            assert!(
                std::panic::catch_unwind(|| crate::hc074_boot_operations_view!(path, &bad))
                    .is_err()
            );
        }
        assert!(
            crate::hc074_boot_operations_view!(path, &format!("{raw}\n// drift\n"))
                .ends_with("// drift\n")
        );
    }
    let path = "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs";
    let baseline = old(path);
    let candidate = baseline.replacen(
        "if !directory && name.len() < short_name.len()",
        "if name.len() < short_name.len()",
        1,
    );
    assert_ne!(baseline, candidate);
    assert_eq!(
        crate::hc074_boot_operations_view!(path, &candidate),
        baseline
    );
    assert_eq!(
        crate::hc074_boot_operations_view!(path, &read(path)),
        baseline
    );
    for bad in [
        candidate.replacen(
            "let name = if name.len() < short_name.len()",
            "let name = if name.len() <= short_name.len()",
            1,
        ),
        format!("{candidate}{candidate}"),
    ] {
        assert!(
            std::panic::catch_unwind(|| crate::hc074_boot_operations_view!(path, &bad)).is_err()
        );
    }
    for (before, after) in [
        ("libc::O_NOFOLLOW", "libc::O_APPEND"),
        ("File::from_raw_fd(fd)", "File::from_raw_fd(fd+1)"),
    ] {
        assert_ne!(
            crate::hc074_boot_operations_view!(path, &candidate.replacen(before, after, 1)),
            baseline
        );
    }
}
#[test]
fn boot_operations_fresh_open_expression_and_original_runtime_are_frozen() {
    let origin = "tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs";
    assert_eq!(
        crate::hc074_pidfd_info_path_view!(origin, &read(origin)),
        old(origin)
    );
    for path in [
        "tools/long-run-supervisor-074/src/diagnostic_boot_document.rs",
        "tools/long-run-supervisor-074/src/diagnostic_boot_comparison.rs",
        "tools/long-run-supervisor-074/tests/support/authority_cpu.rs",
        "tools/long-run-supervisor-074/tests/support/authority_leaf_cpu.rs",
        "tools/long-run-supervisor-074/tests/support/authority_owners_controls.rs",
        "tools/long-run-supervisor-074/tests/support/authority_open_name_alloc.rs",
    ] {
        assert_eq!(read(path), old(path));
    }
    let probe = read("tools/long-run-supervisor-074/tests/support/boot_operations.rs");
    let open="let fd = observer.around(Site::Open, || unsafe {\n        libc::openat(parent.as_raw_fd(), name.as_ptr(), flags)\n    });";
    assert_eq!(probe.matches(open).count(), 1);
    let restored = probe
        .replacen(
            open,
            "let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };",
            1,
        )
        .replacen(
            "observer.around(Site::Filesystem, || filesystem(&file, 0x9fa0))?;",
            "filesystem(&file, 0x9fa0)?;",
            1,
        );
    assert_eq!(body(&restored, "child"), body(&read(origin), "fixed_child"));
}
