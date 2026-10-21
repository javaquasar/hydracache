#[path = "support/collector_peer_capacity_guard.rs"]
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
fn collector_peer_capacity_restores_both_old_files_and_rejects_validation_or_union_drift() {
    let prior = "7e87960c";
    let alloc = "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs";
    let collector = "tools/long-run-supervisor-074/src/diagnostic_kernel_collector_inputs.rs";
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
    assert_eq!(old_changed, [alloc, collector]);
    for path in old_changed {
        assert_eq!(
            crate::hc074_collector_peer_capacity_view!(path, &read(path)),
            git(&["show", &format!("{prior}:{path}")])
        );
    }
    let raw = read(collector);
    let before = git(&["show", &format!("{prior}:{collector}")]);
    for (from, to) in [
        ("gids.push(gid);", ""),
        ("gids.dedup();", ""),
        ("gids.sort_unstable();", ""),
        ("self.sender.revalidate(&self.origin)?;", ""),
        ("valid_hold(&self.received, &now)?;", ""),
        ("self.latch_refusal();", ""),
        ("now.unix_seconds()", "0"),
    ] {
        assert_ne!(
            crate::hc074_collector_peer_capacity_view!(collector, &raw.replace(from, to)),
            before
        );
    }
    for bad in [
        raw.replace("supplementary.len() + 1", "33"),
        raw.replace(
            "pub(crate) mod capacity_controls;",
            "pub mod capacity_controls;",
        ),
    ] {
        assert!(
            std::panic::catch_unwind(|| crate::hc074_collector_peer_capacity_view!(
                collector, &bad
            ))
            .is_err()
        );
    }
}

#[test]
fn collector_peer_capacity_freezes_baseline_corpus_reference_and_complete_wrapper() {
    let baseline_path = "tools/long-run-supervisor-074/tests/support/collector_peer_baseline.rs";
    let baseline = read(baseline_path);
    assert_eq!(
        baseline,
        git(&["show", &format!("7e87960c:{baseline_path}")])
    );
    let model = read("tools/long-run-supervisor-074/tests/support/collector_peer_capacity.rs");
    assert_eq!(tokens(peer_body(&model)), tokens(peer_body(&baseline)));
    let cases = |text: &str| {
        tokens(
            text.split_once("fn cases() -> Vec<(usize, String, u32, Vec<u32>)> {\n")
                .unwrap()
                .1
                .split_once("\n}\n")
                .unwrap()
                .0,
        )
    };
    assert_eq!(cases(&model), cases(&baseline));
    assert_eq!(
        tokens(wrapper_body(&model)),
        tokens(wrapper_body(&baseline))
    );
    let runtime = read("tools/long-run-supervisor-074/src/diagnostic_kernel_collector_inputs.rs");
    let restored=peer_body(&runtime).replace("let mut gids = Vec::with_capacity(supplementary.len() + 1);\n    gids.extend_from_slice(supplementary);","let mut gids = supplementary.to_vec();");
    assert_eq!(tokens(&restored), tokens(peer_body(&baseline)));
    assert_eq!(
        tokens(&wrapper_body(&model).replace("self.project_peer(", "peer(")),
        tokens(wrapper_body(&runtime))
    );
    let adapter =
        read("tools/long-run-supervisor-074/tests/support/collector_peer_capacity_adapter.rs");
    assert_eq!(tokens(&adapter),tokens("//! Pure test-only forwarding to the actual private peer helper; no live packet.\nuse super::*;\npub(crate) fn candidate_peer(uid: u32, gid: u32, supplementary: &[u32]) -> DiagnosticPeer {\n    peer(uid, gid, supplementary)\n}\n"));
    let admission = "tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs";
    assert_eq!(
        read(admission),
        git(&["show", &format!("7e87960c:{admission}")])
    );
}

#[test]
fn collector_peer_capacity_controls_have_no_live_capabilities_or_ignored_tests() {
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
        "tools/long-run-supervisor-074/tests/support/collector_peer_capacity.rs",
        "tools/long-run-supervisor-074/tests/support/collector_peer_capacity_adapter.rs",
    ] {
        let source = read(path);
        let mut ids = vec![];
        visit(source.parse().unwrap(), &mut ids);
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
    }
    assert_eq!(
        read("tools/long-run-supervisor-074/tests/support/collector_peer_capacity.rs")
            .matches("#[test]")
            .count(),
        5
    );
}
