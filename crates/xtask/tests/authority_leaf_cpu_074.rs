#[path = "support/document_scratch_guard.rs"]
mod guard;

#[test]
fn authority_leaf_cpu_only_adds_exact_test_hooks_and_preserves_runtime() {
    for path in [
        "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs",
        "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs",
        "docs/testing/gated-test-registry.toml",
    ] {
        let raw = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(path),
        )
        .unwrap()
        .replace('\r', "");
        let output = std::process::Command::new("git")
            .args(["show", &format!("b6f84539:{path}")])
            .output()
            .unwrap();
        assert!(output.status.success());
        let baseline = String::from_utf8(output.stdout).unwrap().replace('\r', "");
        let restored = crate::hc074_authority_leaf_cpu_view!(path, &raw);
        assert_eq!(restored, baseline);
        assert_eq!(
            crate::hc074_authority_leaf_cpu_view!(path, &restored),
            restored
        );
        assert!(crate::hc074_authority_leaf_cpu_view!(
            path,
            &format!("{raw}\n// unrelated drift\n")
        )
        .contains("// unrelated drift"));
        if path.ends_with("fixture.rs") {
            continue;
        }
        let marker = if path.ends_with("alloc_tests.rs") {
            "mod authority_leaf_cpu;"
        } else {
            "test = \"authority_leaf_cpu_selected_attribution\""
        };
        for changed in [raw.replacen(marker, "changed", 1), format!("{raw}{raw}")] {
            assert!(
                std::panic::catch_unwind(|| crate::hc074_authority_leaf_cpu_view!(path, &changed))
                    .is_err()
            );
        }
    }
    assert_eq!(guard::normalize("unrelated", "unchanged"), "unchanged");
}

#[test]
fn authority_leaf_cpu_reuses_frozen_traces_and_exact_clock_bodies() {
    use proc_macro2::{Delimiter, TokenStream, TokenTree};
    fn read(path: &str) -> String {
        std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(path),
        )
        .unwrap()
        .replace('\r', "")
    }
    fn function(source: &str, name: &str) -> String {
        let stream: TokenStream = source.parse().unwrap();
        let trees: Vec<_> = stream.into_iter().collect();
        let mut found = Vec::new();
        for index in 0..trees.len().saturating_sub(1) {
            if trees[index].to_string() == "fn" && trees[index + 1].to_string() == name {
                let body = trees[index + 2..]
                    .iter()
                    .find_map(|tree| match tree {
                        TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => {
                            Some(group.stream().to_string())
                        }
                        _ => None,
                    })
                    .unwrap();
                found.push(body);
            }
        }
        assert_eq!(found.len(), 1);
        found.remove(0)
    }
    let new = read("tools/long-run-supervisor-074/tests/support/authority_leaf_cpu.rs");
    let old = read("tools/long-run-supervisor-074/tests/support/authority_cpu.rs");
    for name in ["nanoseconds", "reading", "cpu"] {
        assert_eq!(function(&new, name), function(&old, name));
    }
    for path in [
        "tools/long-run-supervisor-074/tests/support/authority_owners_controls.rs",
        "tools/long-run-supervisor-074/tests/support/authority_owners_alloc.rs",
        "tools/long-run-supervisor-074/tests/support/authority_cpu.rs",
        "tools/long-run-supervisor-074/tests/support/validation_leaf_requests.rs",
    ] {
        let output = std::process::Command::new("git")
            .args(["show", &format!("b6f84539:{path}")])
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(
            read(path),
            String::from_utf8(output.stdout).unwrap().replace('\r', "")
        );
    }
    assert!(new.contains("2..=8 => fixture.run(phase(kind, true), observer)"));
}
