#[path = "support/pidfd_info_path_guard.rs"]
mod guard;

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "")
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
fn pidfd_info_path_restores_entire_old_origin_and_keeps_all_other_old_runtime_files() {
    let origin = "tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs";
    let raw = read(origin);
    let old = git(&["show", &format!("f9e7cada:{origin}")]);
    assert_eq!(crate::hc074_pidfd_info_path_view!(origin, &raw), old);
    for bad in [
        raw.replace("mod pidfd_info_path;", "pub mod pidfd_info_path;"),
        raw.replace(
            "pidfd_info_path::InfoPath::new(pidfd.as_raw_fd()).as_path()",
            "pidfd_info_path::InfoPath::new(0).as_path()",
        ),
        format!("{raw}\n#[path = \"diagnostic_pidfd_info_path.rs\"]\nmod pidfd_info_path;\n"),
    ] {
        assert!(
            std::panic::catch_unwind(|| crate::hc074_pidfd_info_path_view!(origin, &bad)).is_err()
        );
    }
    for (before, after) in [
        ("libc::O_NOFOLLOW", "libc::O_APPEND"),
        ("let raw = read(&info)?;", "let raw = read(&info).unwrap();"),
        (
            "if lines.next().is_some() || observed != pid || pid <= 0",
            "if observed != pid",
        ),
    ] {
        assert_ne!(
            crate::hc074_pidfd_info_path_view!(origin, &raw.replace(before, after)),
            old
        );
    }
    let previous = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        "f9e7cada",
        "--",
        "tools/long-run-supervisor-074/src",
        "tools/long-run-supervisor-074/tests",
    ]);
    let changed = git(&[
        "diff",
        "--name-only",
        "f9e7cada",
        "--",
        "tools/long-run-supervisor-074/src",
        "tools/long-run-supervisor-074/tests",
    ]);
    let old_changes: Vec<_> = changed
        .lines()
        .filter(|p| previous.lines().any(|old| old == *p))
        .filter(|p| {
            if matches!(
                *p,
                "tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs"
                    | "tools/long-run-supervisor-074/src/diagnostic_worker_context.rs"
                    | "tools/long-run-supervisor-074/src/diagnostic_worker_trust_input.rs"
            ) {
                // A successor may change only the exact bounded-collection leaf.
                // Require the entire earlier file, not a filename exemption.
                assert_eq!(
                    crate::hc074_projection_groups_view!(*p, &read(p)),
                    git(&["show", &format!("f9e7cada:{p}")])
                );
                false
            } else {
                true
            }
        })
        .collect();
    assert_eq!(
        old_changes,
        vec![
            "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs",
            origin
        ]
    );
}

#[test]
fn pidfd_info_path_helper_is_private_pure_owned_bytes_without_cache_or_new_kernel_calls() {
    let helper = read("tools/long-run-supervisor-074/src/diagnostic_pidfd_info_path.rs");
    fn visit(stream: proc_macro2::TokenStream, ids: &mut Vec<String>) {
        for token in stream {
            match token {
                proc_macro2::TokenTree::Ident(i) => ids.push(i.to_string()),
                proc_macro2::TokenTree::Group(g) => visit(g.stream(), ids),
                _ => {}
            }
        }
    }
    let mut ids = vec![];
    visit(helper.parse().unwrap(), &mut ids);
    for forbidden in [
        "unsafe",
        "String",
        "Vec",
        "Arc",
        "Mutex",
        "OnceLock",
        "static",
        "clock_gettime",
        "File",
        "OpenOptions",
        "read_at",
        "open",
        "openat",
        "send",
        "fork",
        "execve",
    ] {
        assert!(
            !ids.iter().any(|i| i == forbidden),
            "unexpected helper capability: {forbidden}"
        );
    }
    assert!(helper.contains("bytes: [u8; PREFIX.len() + 11]"));
    assert!(helper.contains("let digits = PidName::new(fd);"));
    assert_eq!(helper.matches("pub(super) fn ").count(), 2);
    assert!(!helper.contains("pub(crate)"));
}

#[test]
fn pidfd_info_path_only_exact_allocation_hook_is_added_at_baseline() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs";
    let raw = std::fs::read_to_string(root.join(path))
        .unwrap()
        .replace('\r', "");
    let prior = std::process::Command::new("git")
        .args(["show", &format!("f9e7cada:{path}")])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(prior.status.success());
    let prior = String::from_utf8(prior.stdout).unwrap().replace('\r', "");
    assert_eq!(crate::hc074_pidfd_info_path_view!(path, &raw), prior);
    let bad = raw.replace("mod pidfd_info_path;", "pub mod pidfd_info_path;");
    assert!(std::panic::catch_unwind(|| crate::hc074_pidfd_info_path_view!(path, &bad)).is_err());
    assert_ne!(
        crate::hc074_pidfd_info_path_view!(path, &format!("{raw}\n// drift\n")),
        prior
    );
}
