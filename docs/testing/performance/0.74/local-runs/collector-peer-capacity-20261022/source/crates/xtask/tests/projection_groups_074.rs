#[path = "support/projection_groups_guard.rs"]
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
fn projection_groups_reconstructs_entire_old_parser_and_every_changed_old_source() {
    let prior = "07cfb39e";
    let admission = "tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs";
    let alloc = "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs";
    let old_paths = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        prior,
        "--",
        "tools/long-run-supervisor-074/src",
    ]);
    let changed = git(&[
        "diff",
        "--name-only",
        prior,
        "--",
        "tools/long-run-supervisor-074/src",
    ]);
    let changed_old: Vec<_> = changed
        .lines()
        .filter(|p| old_paths.lines().any(|o| o == *p))
        .filter(|p| {
            if matches!(
                *p,
                "tools/long-run-supervisor-074/src/diagnostic_worker_context.rs"
                    | "tools/long-run-supervisor-074/src/diagnostic_worker_trust_input.rs"
                    | "tools/long-run-supervisor-074/src/diagnostic_kernel_collector_inputs.rs"
            ) {
                assert_eq!(
                    crate::hc074_context_document_stack_view!(*p, &read(p)),
                    git(&["show", &format!("{prior}:{p}")])
                );
                false
            } else {
                true
            }
        })
        .collect();
    assert_eq!(changed_old, [alloc, admission]);
    for path in changed_old {
        assert_eq!(
            crate::hc074_projection_groups_view!(path, &read(path)),
            git(&["show", &format!("{prior}:{path}")])
        );
    }
    let raw = read(admission);
    let old = git(&["show", &format!("{prior}:{admission}")]);
    for (before, after) in [
        (".take(33)", ".take(32)"),
        ("if values.len() > 32", "if values.len() > 33"),
        (
            "let groups = groups_field(\"Groups:\")?;",
            "let groups = groups_field(\"Gid:\")?;",
        ),
        ("origin.revalidate()?;", "// origin check removed"),
        (
            "projection(scratch.read(file)?, &self.credentials)?",
            "self.projection",
        ),
    ] {
        assert_ne!(
            crate::hc074_projection_groups_view!(admission, &raw.replace(before, after)),
            old
        );
    }
    for bad in [
        raw.replace("mod projection_groups;", "pub mod projection_groups;"),
        raw.replace("projection_groups::Values", "Vec<u32>"),
        format!("{raw}\n#[path = \"diagnostic_projection_groups.rs\"]\nmod projection_groups;\n\n"),
    ] {
        assert!(
            std::panic::catch_unwind(|| crate::hc074_projection_groups_view!(admission, &bad))
                .is_err()
        );
    }
}

#[test]
fn projection_groups_helper_is_bounded_local_and_has_no_live_capabilities() {
    let helper = read("tools/long-run-supervisor-074/src/diagnostic_projection_groups.rs");
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
        "Arc",
        "Mutex",
        "OnceLock",
        "static",
        "File",
        "OpenOptions",
        "clock_gettime",
        "openat",
        "read_at",
        "send",
        "fork",
        "execve",
        "with_capacity",
        "reserve",
    ] {
        assert!(
            !ids.iter().any(|i| i == forbidden),
            "unexpected capability {forbidden}"
        );
    }
    assert!(helper.contains("values: [u32; 33]"));
    assert!(helper.contains("self.values[..self.len].to_vec()"));
    assert_eq!(helper.matches("pub(super) fn ").count(), 1);
    assert!(!helper.contains("pub(crate)"));
}
