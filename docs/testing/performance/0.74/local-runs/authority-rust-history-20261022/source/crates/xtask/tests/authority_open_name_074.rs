#[path = "support/document_scratch_guard.rs"]
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
fn baseline(path: &str) -> String {
    let output = history::command()
        .args(["show", &format!("6eeb4b63:{path}")])
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().replace('\r', "")
}
fn function(source: &str, name: &str) -> String {
    use proc_macro2::{Delimiter, TokenStream, TokenTree};
    let trees: Vec<_> = source.parse::<TokenStream>().unwrap().into_iter().collect();
    let mut bodies = Vec::new();
    for index in 0..trees.len().saturating_sub(1) {
        if trees[index].to_string() == "fn" && trees[index + 1].to_string() == name {
            bodies.push(
                trees[index + 2..]
                    .iter()
                    .find_map(|tree| match tree {
                        TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => {
                            Some(group.stream().to_string())
                        }
                        _ => None,
                    })
                    .unwrap(),
            );
        }
    }
    assert_eq!(bodies.len(), 1);
    bodies.remove(0)
}
#[test]
fn authority_open_name_restores_exact_test_hooks_and_preserves_unrelated_drift() {
    for (path, marker) in [
        (
            "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs",
            "pub(crate) mod open_name_controls;",
        ),
        (
            "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs",
            "mod authority_open_name_alloc;",
        ),
        (
            "docs/testing/gated-test-registry.toml",
            "test = \"authority_open_name_allocation_cpu_attribution\"",
        ),
    ] {
        let raw = read(path);
        let restored = crate::hc074_authority_open_name_view!(path, &raw);
        assert_eq!(restored, baseline(path));
        assert_eq!(
            crate::hc074_authority_open_name_view!(path, &restored),
            restored
        );
        assert!(crate::hc074_authority_open_name_view!(
            path,
            &format!("{raw}\n// unrelated drift\n")
        )
        .ends_with("// unrelated drift\n"));
        for changed in [raw.replacen(marker, "changed", 1), format!("{raw}{raw}")] {
            assert!(
                std::panic::catch_unwind(|| crate::hc074_authority_open_name_view!(path, &changed))
                    .is_err()
            );
        }
    }
    assert_eq!(guard::normalize("unrelated", "unchanged"), "unchanged");
}
#[test]
fn authority_open_name_freezes_original_syscall_and_clock_bodies() {
    let fixture = baseline("tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs");
    let controls =
        read("tools/long-run-supervisor-074/tests/support/authority_open_name_controls.rs");
    assert_eq!(
        function(&fixture, "open_at"),
        function(&controls, "reference")
    );
    let probe = read("tools/long-run-supervisor-074/tests/support/authority_open_name_alloc.rs");
    let clock = baseline("tools/long-run-supervisor-074/tests/support/authority_cpu.rs");
    for name in ["nanoseconds", "reading", "cpu"] {
        assert_eq!(function(&probe, name), function(&clock, name));
    }
    for path in [
        "tools/long-run-supervisor-074/tests/support/authority_owners_controls.rs",
        "tools/long-run-supervisor-074/tests/support/authority_owners_alloc.rs",
        "tools/long-run-supervisor-074/tests/support/validation_leaf_requests.rs",
    ] {
        assert_eq!(read(path), baseline(path));
    }
}

#[test]
fn authority_open_name_candidate_restores_only_the_exact_allocation_branch() {
    let path = "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs";
    let raw = crate::hc074_boot_operations_view!(path, &read(path));
    assert!(raw.contains("let mut short_name = [0_u8; 24];"));
    for (before, after) in [
        ("!directory &&", "directory &&"),
        (
            "name.len() < short_name.len()",
            "name.len() <= short_name.len()",
        ),
        ("name.len() + 1", "name.len()"),
        ("from_bytes_with_nul", "from_bytes_with_nul_unchecked"),
    ] {
        let changed = raw.replacen(before, after, 1);
        assert_ne!(changed, raw);
        assert!(
            std::panic::catch_unwind(|| crate::hc074_authority_open_name_view!(path, &changed))
                .is_err()
        );
    }
    // Flags, syscall, errno capture and ownership remain outside normalization.
    for (before, after) in [
        ("libc::O_NOFOLLOW", "libc::O_APPEND"),
        ("File::from_raw_fd(fd)", "File::from_raw_fd(fd + 1)"),
        (
            "std::io::Error::last_os_error()",
            "std::io::Error::other(\"drift\")",
        ),
    ] {
        let changed = raw.replacen(before, after, 1);
        assert_ne!(changed, raw);
        assert_ne!(
            crate::hc074_authority_open_name_view!(path, &changed),
            baseline(path)
        );
    }
}
