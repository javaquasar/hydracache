#[path = "support/context_document_baseline_guard.rs"]
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
fn tokens(body: &str) -> String {
    format!("{{{body}}}")
        .parse::<proc_macro2::TokenStream>()
        .unwrap()
        .to_string()
}

#[test]
fn context_document_baseline_changes_only_one_test_hook_not_any_runtime_body() {
    let prior = "8717ea4f";
    let path = "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs";
    let previous = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        prior,
        "--",
        "tools/long-run-supervisor-074/src",
        "tools/long-run-supervisor-074/tests",
    ]);
    let changed = git(&[
        "diff",
        "--name-only",
        prior,
        "--",
        "tools/long-run-supervisor-074/src",
        "tools/long-run-supervisor-074/tests",
    ]);
    let old_changed: Vec<_> = changed
        .lines()
        .filter(|p| previous.lines().any(|v| v == *p))
        .filter(|p| {
            if *p == "tools/long-run-supervisor-074/src/diagnostic_worker_context.rs" {
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
    assert_eq!(old_changed, [path]);
    let raw = read(path);
    let old = git(&["show", &format!("{prior}:{path}")]);
    assert_eq!(
        crate::hc074_context_document_baseline_view!(path, &raw),
        old
    );
    for bad in [raw.replace("mod context_document_buffer_baseline;","pub mod context_document_buffer_baseline;"), format!("{raw}\n#[path = \"../tests/support/context_document_buffer_baseline.rs\"]\nmod context_document_buffer_baseline;\n")] {
        assert!(std::panic::catch_unwind(||crate::hc074_context_document_baseline_view!(path,&bad)).is_err());
    }
    assert_ne!(
        crate::hc074_context_document_baseline_view!(path, &format!("{raw}\n// unrelated drift\n")),
        old
    );
}

#[test]
fn context_document_baseline_model_has_the_complete_original_read_body() {
    let context = "tools/long-run-supervisor-074/src/diagnostic_worker_context.rs";
    let raw = crate::hc074_context_document_stack_view!(context, &read(context));
    assert_eq!(raw, git(&["show", &format!("8717ea4f:{context}")]));
    let method = raw
        .split_once("    fn read_with(\n")
        .unwrap()
        .1
        .split_once("\n}\nstruct Namespaces")
        .unwrap()
        .0;
    let body = method
        .split_once(") -> Result<(), WorkerContextError> {\n")
        .unwrap()
        .1
        .strip_suffix("\n    }")
        .unwrap();
    let model =
        read("tools/long-run-supervisor-074/tests/support/context_document_buffer_baseline.rs");
    let method = model
        .split_once("fn frozen_read_with(\n")
        .unwrap()
        .1
        .split_once("\n}\n")
        .unwrap()
        .0;
    let proposed = method
        .split_once(") -> Result<(), WorkerContextError> {\n")
        .unwrap()
        .1;
    assert_eq!(proposed.matches("!metadata()?").count(), 2);
    let restored = proposed.replace(
        "!metadata()?",
        "FileStamp::from(self.file.metadata()?) != self.stamp",
    );
    assert_eq!(tokens(&restored), tokens(body));
    for (before, after) in [
        ("length + 1", "38"),
        ("read(&mut buffer)?", "read(&mut buffer).unwrap()"),
        ("count != length", "count > length"),
        ("buffer[count - 1] != b'\\n'", "false"),
    ] {
        assert_ne!(tokens(&restored.replace(before, after)), tokens(body));
    }
}

#[test]
fn context_document_baseline_controls_have_no_live_or_host_capabilities() {
    let source =
        read("tools/long-run-supervisor-074/tests/support/context_document_buffer_baseline.rs");
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
    visit(source.parse().unwrap(), &mut ids);
    for forbidden in [
        "unsafe",
        "File",
        "OpenOptions",
        "read_at",
        "Origin",
        "Probe",
        "KernelPacket",
        "KernelReceiver",
        "Command",
        "clock_gettime",
        "Instant",
        "SystemTime",
        "send",
        "openat",
        "fork",
        "execve",
    ] {
        assert!(
            !ids.iter().any(|i| i == forbidden),
            "unexpected baseline capability {forbidden}"
        );
    }
    assert_eq!(source.matches("#[test]").count(), 5);
}
