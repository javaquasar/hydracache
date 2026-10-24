#[path = "support/trust_component_stack_guard.rs"]
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
fn tokens(body: &str) -> String {
    format!("{{{body}}}")
        .parse::<proc_macro2::TokenStream>()
        .unwrap()
        .to_string()
}
fn body<'a>(source: &'a str, name: &str) -> &'a str {
    source
        .split_once(name)
        .unwrap()
        .1
        .split_once(") -> Result<i32, TrustInputError> {\n")
        .unwrap()
        .1
        .split_once("\n}\n")
        .unwrap()
        .0
}

#[test]
fn trust_component_stack_restores_both_entire_old_files_and_rejects_predicate_drift() {
    let prior = "ed941972";
    let alloc = "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs";
    let trust = "tools/long-run-supervisor-074/src/diagnostic_worker_trust_input.rs";
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
        .filter(|p| previous.lines().any(|old| old == *p))
        .filter(|p| {
            if *p == "tools/long-run-supervisor-074/src/diagnostic_kernel_collector_inputs.rs" {
                assert_eq!(
                    crate::hc074_collector_peer_capacity_view!(*p, &read(p)),
                    git(&["show", &format!("{prior}:{p}")])
                );
                false
            } else {
                true
            }
        })
        .collect();
    assert_eq!(old_changed, [alloc, trust]);
    for path in old_changed {
        assert_eq!(
            crate::hc074_trust_component_stack_view!(path, &read(path)),
            git(&["show", &format!("{prior}:{path}")])
        );
    }
    let raw = read(trust);
    let old = git(&["show", &format!("{prior}:{trust}")]);
    for (from, to) in [
        ("libc::O_NOFOLLOW", "0"),
        ("libc::O_NONBLOCK", "0"),
        ("fd < 0", "fd == -1"),
        ("name.as_ptr()", "std::ptr::null()"),
        (
            "std::io::Error::last_os_error()",
            "std::io::Error::other(\"changed\")",
        ),
    ] {
        assert_ne!(
            crate::hc074_trust_component_stack_view!(trust, &raw.replace(from, to)),
            old
        );
    }
    for bad in [
        raw.replace("Name::new(name)", "Name::new(\"etc\")"),
        raw.replace("mod component_name;", "pub mod component_name;"),
    ] {
        assert!(
            std::panic::catch_unwind(|| crate::hc074_trust_component_stack_view!(trust, &bad))
                .is_err()
        );
    }
}

#[test]
fn trust_component_stack_models_and_corpus_match_the_full_baseline_and_actual_body() {
    let baseline_path = "tools/long-run-supervisor-074/tests/support/trust_component_baseline.rs";
    let baseline = read(baseline_path);
    assert_eq!(
        baseline,
        git(&["show", &format!("ed941972:{baseline_path}")])
    );
    let controls = read("tools/long-run-supervisor-074/tests/support/trust_component_stack.rs");
    let old = body(&baseline, "fn frozen_child(\n");
    assert_eq!(tokens(old), tokens(body(&controls, "fn frozen_child(\n")));
    let cases = |text: &str| {
        tokens(
            text.split_once("fn valid_cases() -> Vec<(String, String, bool)> {\n")
                .unwrap()
                .1
                .split_once("\n}\n")
                .unwrap()
                .0,
        )
    };
    assert_eq!(cases(&baseline), cases(&controls));
    let adapter =
        read("tools/long-run-supervisor-074/tests/support/trust_component_stack_adapter.rs");
    let new = body(&adapter, "pub(crate) fn child_model(\n");
    let reverted = new
        .replace("Name::new(name)", "CString::new(name)")
        .replace(
            "open(parent, view(&name), flags)",
            "open(parent, &name, flags)",
        );
    assert_eq!(tokens(&reverted), tokens(old));
    let runtime = read("tools/long-run-supervisor-074/src/diagnostic_worker_trust_input.rs");
    let actual=runtime.split_once("fn child(parent: &File, name: &str, directory: bool) -> Result<File, TrustInputError> {\n").unwrap().1.split_once("\n}\n").unwrap().0;
    for expression in [
        "Name::new(name)",
        "open(parent, view(&name), flags)",
        "last_error()",
        "Ok(fd)",
    ] {
        assert_eq!(new.matches(expression).count(), 1);
    }
    let restored = new
        .replace("Name::new(name)", "component_name::Name::new(name)")
        .replace(
            "open(parent, view(&name), flags)",
            "unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) }",
        )
        .replace("last_error()", "std::io::Error::last_os_error()")
        .replace("Ok(fd)", "Ok(unsafe { File::from_raw_fd(fd) })");
    assert_eq!(tokens(&restored), tokens(actual));
}

#[test]
fn trust_component_stack_storage_adapter_and_controls_have_no_live_capabilities() {
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
        "tools/long-run-supervisor-074/src/diagnostic_trust_component_name.rs",
        "tools/long-run-supervisor-074/tests/support/trust_component_stack.rs",
        "tools/long-run-supervisor-074/tests/support/trust_component_stack_adapter.rs",
    ] {
        let mut ids = vec![];
        visit(read(path).parse().unwrap(), &mut ids);
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
            "ignore",
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
    let helper = read("tools/long-run-supervisor-074/src/diagnostic_trust_component_name.rs");
    assert!(helper.contains("bytes: [u8; 32]"));
    assert!(helper.contains("if name.len() < 32"));
    assert!(helper.contains("CString::new(name)"));
    assert!(helper.contains("name.as_bytes().contains(&0)"));
    assert!(!helper.contains("pub(crate)"));
    assert_eq!(helper.matches("pub(super) fn ").count(), 3);
}
