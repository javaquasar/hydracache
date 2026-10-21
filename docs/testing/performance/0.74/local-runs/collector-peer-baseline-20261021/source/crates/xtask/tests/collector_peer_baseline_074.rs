#[path = "support/collector_peer_baseline_guard.rs"]
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
    let value = std::process::Command::new("git")
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
fn peer_body(text: &str) -> &str {
    text.split_once("fn peer(")
        .unwrap()
        .1
        .split_once("-> DiagnosticPeer {\n")
        .unwrap()
        .1
        .split_once("\n}\n")
        .unwrap()
        .0
}
fn wrapper_body(text: &str) -> &str {
    text.split_once("fn original_collector_inputs(")
        .unwrap()
        .1
        .split_once("KernelError> {\n")
        .unwrap()
        .1
        .split_once("\n    }\n")
        .unwrap()
        .0
}

#[test]
fn collector_peer_baseline_changes_only_a_private_allocator_test_hook() {
    let prior = "1cde821f";
    let alloc = "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs";
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
        .collect();
    assert_eq!(old_changed, [alloc]);
    let raw = read(alloc);
    let before = git(&["show", &format!("{prior}:{alloc}")]);
    assert_eq!(
        crate::hc074_collector_peer_baseline_view!(alloc, &raw),
        before
    );
    for bad in [raw.replace("mod collector_peer_baseline;","pub mod collector_peer_baseline;"),format!("{raw}\n#[path = \"../tests/support/collector_peer_baseline.rs\"]\nmod collector_peer_baseline;\n")] {
        assert!(std::panic::catch_unwind(||crate::hc074_collector_peer_baseline_view!(alloc,&bad)).is_err());
    }
    assert_ne!(
        crate::hc074_collector_peer_baseline_view!(alloc, &format!("{raw}\n// drift\n")),
        before
    );
}

#[test]
fn collector_peer_baseline_freezes_complete_peer_and_wrapper_bodies() {
    let path = "tools/long-run-supervisor-074/src/diagnostic_kernel_collector_inputs.rs";
    let runtime = read(path);
    assert_eq!(runtime, git(&["show", &format!("1cde821f:{path}")]));
    let model = read("tools/long-run-supervisor-074/tests/support/collector_peer_baseline.rs");
    assert_eq!(tokens(peer_body(&model)), tokens(peer_body(&runtime)));
    let restored = wrapper_body(&model).replace("self.project_peer(", "peer(");
    assert_eq!(
        wrapper_body(&model).matches("self.project_peer(").count(),
        1
    );
    assert_eq!(tokens(&restored), tokens(wrapper_body(&runtime)));
    for (from, to) in [
        ("gids.push(gid);", ""),
        ("gids.dedup();", ""),
        (
            "supplementary.to_vec()",
            "supplementary.iter().take(32).copied().collect::<Vec<_>>()",
        ),
    ] {
        assert_ne!(
            tokens(&peer_body(&model).replace(from, to)),
            tokens(peer_body(&runtime))
        );
    }
    for (from, to) in [
        ("self.sender.revalidate(&self.origin)?;", ""),
        ("valid_hold(&self.received, &now)?;", ""),
        ("self.latch_refusal();", ""),
        ("now.unix_seconds()", "0"),
    ] {
        assert_ne!(
            tokens(&restored.replace(from, to)),
            tokens(wrapper_body(&runtime))
        );
    }
    let admission = "tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs";
    assert_eq!(
        read(admission),
        git(&["show", &format!("1cde821f:{admission}")])
    );
    assert!(read(admission).contains("if values.len() > 32"));
}

#[test]
fn collector_peer_baseline_controls_have_no_live_capabilities() {
    fn visit(stream: proc_macro2::TokenStream, ids: &mut Vec<String>) {
        for token in stream {
            match token {
                proc_macro2::TokenTree::Ident(i) => ids.push(i.to_string()),
                proc_macro2::TokenTree::Group(g) => visit(g.stream(), ids),
                _ => {}
            }
        }
    }
    let model = read("tools/long-run-supervisor-074/tests/support/collector_peer_baseline.rs");
    let mut ids = vec![];
    visit(model.parse().unwrap(), &mut ids);
    for forbidden in [
        "unsafe",
        "File",
        "OpenOptions",
        "Origin",
        "KernelPacket",
        "KernelReceiver",
        "Probe",
        "Command",
        "clock_gettime",
        "Instant",
        "SystemTime",
        "openat",
        "read_at",
        "fork",
        "execve",
        "send",
        "ignore",
        "static",
        "Arc",
        "Mutex",
        "OnceLock",
    ] {
        assert!(
            !ids.iter().any(|i| i == forbidden),
            "unexpected capability {forbidden}"
        );
    }
    assert_eq!(model.matches("#[test]").count(), 5);
}
