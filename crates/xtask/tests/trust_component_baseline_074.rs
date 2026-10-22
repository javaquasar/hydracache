#[path = "support/trust_component_baseline_guard.rs"]
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

#[test]
fn trust_component_baseline_changes_only_one_private_test_hook() {
    let prior = "1cc648ad";
    let path = "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs";
    let old = git(&[
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
        .filter(|p| old.lines().any(|o| o == *p))
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
    assert_eq!(old_changed, [path]);
    let raw = read(path);
    let before = git(&["show", &format!("{prior}:{path}")]);
    assert_eq!(
        crate::hc074_trust_component_baseline_view!(path, &raw),
        before
    );
    for bad in [raw.replace("mod trust_component_baseline;","pub mod trust_component_baseline;"),format!("{raw}\n#[path = \"../tests/support/trust_component_baseline.rs\"]\nmod trust_component_baseline;\n")] {
        assert!(std::panic::catch_unwind(||crate::hc074_trust_component_baseline_view!(path,&bad)).is_err());
    }
    assert_ne!(
        crate::hc074_trust_component_baseline_view!(path, &format!("{raw}\n// drift\n")),
        before
    );
}

#[test]
fn trust_component_baseline_model_restores_the_complete_child_body_and_fixed_names() {
    let path = "tools/long-run-supervisor-074/src/diagnostic_worker_trust_input.rs";
    let raw = crate::hc074_trust_component_stack_view!(path, &read(path));
    assert_eq!(raw, git(&["show", &format!("1cc648ad:{path}")]));
    let body=raw.split_once("fn child(parent: &File, name: &str, directory: bool) -> Result<File, TrustInputError> {\n").unwrap().1.split_once("\n}\n").unwrap().0;
    let control = read("tools/long-run-supervisor-074/tests/support/trust_component_baseline.rs");
    let model = control
        .split_once(") -> Result<i32, TrustInputError> {\n")
        .unwrap()
        .1
        .split_once("\n}\n")
        .unwrap()
        .0;
    for term in ["open(parent, &name, flags)", "last_error()", "Ok(fd)"] {
        assert_eq!(model.matches(term).count(), 1);
    }
    let restored = model
        .replace(
            "open(parent, &name, flags)",
            "unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) }",
        )
        .replace("last_error()", "std::io::Error::last_os_error()")
        .replace("Ok(fd)", "Ok(unsafe { File::from_raw_fd(fd) })");
    assert_eq!(tokens(&restored), tokens(body));
    for (from, to) in [
        ("CString::new(name)", "CString::new(\"etc\")"),
        ("libc::O_NOFOLLOW", "0"),
        ("libc::O_NONBLOCK", "0"),
        ("fd < 0", "fd == -1"),
    ] {
        assert_ne!(tokens(&restored.replace(from, to)), tokens(body));
    }
    for name in [
        "etc",
        "hydracache-perf",
        "worker-trust-074.json",
        "controller-trust-074.json",
        "held-runtime-trust-074.json",
        "oidc-trust-074.json",
    ] {
        assert!(control.contains(&format!("\"{name}\"")));
    }
    for (source, literal) in [
        (path, "const TRUST_FILE: &str = \"worker-trust-074.json\";"),
        (
            path,
            "const ANCESTORS: [&str; 2] = [\"etc\", \"hydracache-perf\"];",
        ),
        (
            "tools/long-run-supervisor-074/src/diagnostic_controller_trust_input.rs",
            "const CONTROLLER_FILE: &str = \"controller-trust-074.json\";",
        ),
        (
            "tools/long-run-supervisor-074/src/diagnostic_held_runtime_trust.rs",
            "const TRUST_FILE: &str = \"held-runtime-trust-074.json\";",
        ),
        (
            "tools/long-run-supervisor-074/src/diagnostic_oidc_input.rs",
            "const FILE: &str = \"oidc-trust-074.json\";",
        ),
    ] {
        assert!(read(source).contains(literal));
    }
}

#[test]
fn trust_component_baseline_controls_have_no_live_capabilities_or_ignored_tests() {
    let control = read("tools/long-run-supervisor-074/tests/support/trust_component_baseline.rs");
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
    visit(control.parse().unwrap(), &mut ids);
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
    ] {
        assert!(
            !ids.iter().any(|id| id == forbidden),
            "unexpected capability {forbidden}"
        );
    }
    assert_eq!(control.matches("#[test]").count(), 5);
}
