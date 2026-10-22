#[path = "support/context_document_stack_guard.rs"]
mod guard;
#[path = "support/authority_history_io.rs"]
mod history;

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &str) -> String {
    history::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "")
}
fn git(args: &[&str]) -> String {
    let value = history::command()
        .args(args)
        .current_dir(root())
        .output()
        .unwrap();
    assert!(value.status.success());
    String::from_utf8(value.stdout).unwrap().replace('\r', "")
}
fn tokens(text: &str) -> String {
    format!("{{{text}}}")
        .parse::<proc_macro2::TokenStream>()
        .unwrap()
        .to_string()
}
fn model_body<'a>(text: &'a str, name: &str) -> &'a str {
    text.split_once(name)
        .unwrap()
        .1
        .split_once(") -> Result<(), WorkerContextError> {\n")
        .unwrap()
        .1
        .split_once("\n}\n")
        .unwrap()
        .0
}

#[test]
fn context_document_stack_exactly_restores_both_old_files_and_rejects_live_predicate_drift() {
    let prior = "e5035a27";
    let worker = "tools/long-run-supervisor-074/src/diagnostic_worker_context.rs";
    let alloc = "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs";
    let old_paths = git(&[
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
        .filter(|p| old_paths.lines().any(|old| old == *p))
        .filter(|p| {
            if matches!(
                *p,
                "tools/long-run-supervisor-074/src/diagnostic_worker_trust_input.rs"
                    | "tools/long-run-supervisor-074/src/diagnostic_kernel_collector_inputs.rs"
            ) {
                assert_eq!(
                    crate::hc074_trust_component_stack_view!(*p, &read(p)),
                    git(&["show", &format!("{prior}:{p}")])
                );
                false
            } else {
                true
            }
        })
        .collect();
    assert_eq!(old_changed, [alloc, worker]);
    for path in old_changed {
        assert_eq!(
            crate::hc074_context_document_stack_view!(path, &read(path)),
            git(&["show", &format!("{prior}:{path}")])
        );
    }
    let raw = read(worker);
    let old = git(&["show", &format!("{prior}:{worker}")]);
    for (from, to) in [
        ("read(&mut buffer)?", "read(&mut buffer).unwrap()"),
        ("count != length", "count > length"),
        ("expected.len() + 1 != length", "false"),
        (
            "FileStamp::from(self.file.metadata()?) != self.stamp",
            "false",
        ),
    ] {
        assert_ne!(
            crate::hc074_context_document_stack_view!(worker, &raw.replace(from, to)),
            old
        );
    }
    for bad in [
        raw.replace("Scratch::new(length)", "Scratch::new(37)"),
        raw.replace("mod document_buffer;", "pub mod document_buffer;"),
    ] {
        assert!(
            std::panic::catch_unwind(|| crate::hc074_context_document_stack_view!(worker, &bad))
                .is_err()
        );
    }
}

#[test]
fn context_document_stack_models_keep_complete_baseline_and_candidate_bodies() {
    let baseline =
        read("tools/long-run-supervisor-074/tests/support/context_document_buffer_baseline.rs");
    assert_eq!(baseline,git(&["show","e5035a27:tools/long-run-supervisor-074/tests/support/context_document_buffer_baseline.rs"]));
    let old = model_body(&baseline, "fn frozen_read_with(\n");
    let controls = read("tools/long-run-supervisor-074/tests/support/context_document_stack.rs");
    assert_eq!(
        tokens(model_body(&controls, "fn reference(\n")),
        tokens(old)
    );
    let adapter =
        read("tools/long-run-supervisor-074/tests/support/context_document_stack_adapter.rs");
    let new = model_body(&adapter, "pub(crate) fn read_model(\n");
    assert_eq!(new.matches("Scratch::new(length)").count(), 1);
    assert_eq!(
        tokens(&new.replace("Scratch::new(length)", "vec![0; length + 1]")),
        tokens(old)
    );
    let production = read("tools/long-run-supervisor-074/src/diagnostic_worker_context.rs");
    let body = production
        .split_once("    fn read_with(\n")
        .unwrap()
        .1
        .split_once(") -> Result<(), WorkerContextError> {\n")
        .unwrap()
        .1
        .split_once("\n}\nstruct Namespaces")
        .unwrap()
        .0
        .strip_suffix("\n    }")
        .unwrap();
    assert_eq!(new.matches("!metadata()?").count(), 2);
    assert_eq!(
        tokens(
            &new.replace(
                "!metadata()?",
                "FileStamp::from(self.file.metadata()?) != self.stamp"
            )
            .replace(
                "Scratch::new(length)",
                "document_buffer::Scratch::new(length)"
            )
        ),
        tokens(body)
    );
}

#[test]
fn context_document_stack_storage_and_controls_have_no_live_capabilities() {
    fn visit(stream: proc_macro2::TokenStream, ids: &mut Vec<String>) {
        for token in stream {
            match token {
                proc_macro2::TokenTree::Ident(i) => ids.push(i.to_string()),
                proc_macro2::TokenTree::Group(g) => visit(g.stream(), ids),
                _ => {}
            }
        }
    }
    for path in [
        "tools/long-run-supervisor-074/src/diagnostic_context_document_buffer.rs",
        "tools/long-run-supervisor-074/tests/support/context_document_stack.rs",
        "tools/long-run-supervisor-074/tests/support/context_document_stack_adapter.rs",
    ] {
        let text = read(path);
        let mut ids = vec![];
        visit(text.parse().unwrap(), &mut ids);
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
            "Arc",
            "Mutex",
            "OnceLock",
            "static",
        ] {
            assert!(
                !ids.iter().any(|i| i == forbidden),
                "unexpected capability {forbidden} in {path}"
            );
        }
    }
    let helper = read("tools/long-run-supervisor-074/src/diagnostic_context_document_buffer.rs");
    assert!(helper.contains("bytes: [u8; 38]"));
    assert!(helper.contains("if length < 38"));
    assert!(helper.contains("Self::Heap(vec![0; length + 1])"));
    assert!(!helper.contains("pub(crate)"));
    assert_eq!(helper.matches("pub(super) fn ").count(), 1);
}
