const BASE: &str = "a6f95e4e1245b52334956ab835bdea4b1f6f2332";
fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "")
}
fn old(path: &str) -> String {
    let value = std::process::Command::new("git")
        .args(["show", &format!("{BASE}:{path}")])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(value.status.success());
    String::from_utf8(value.stdout).unwrap().replace('\r', "")
}
#[test]
fn inline_boot_reference_restores_the_complete_frozen_receiver_and_model() {
    assert_eq!(
        read("tools/sample-boot-inline-074/probe.rs"),
        old("tools/sample-boot-baseline-074/probe.rs")
    );
    let model = read("tools/sample-boot-inline-074/model.rs");
    let extra = "\n#[path = \"cost.rs\"]\nmod cost;\n";
    assert_eq!(model.matches(extra).count(), 1);
    let restored = model.replace(extra, "").replace(
        "#[path = \"../sample-boot-baseline-074/controls.rs\"]",
        "#[path = \"controls.rs\"]",
    );
    assert_eq!(restored, old("tools/sample-boot-baseline-074/model.rs"));
    let value = std::process::Command::new("git")
        .args([
            "diff",
            "--exit-code",
            BASE,
            "--",
            "tools/long-run-supervisor-074/src",
        ])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(value.status.success());
}
#[test]
fn inline_boot_reference_cost_protocol_is_bounded_and_separate() {
    let cost = read("tools/sample-boot-inline-074/cost.rs");
    for exact in [
        "ITERATIONS: usize = 262_144;",
        "WARMUP: usize = 32_768;",
        "for sample in 0..7",
        "std::hint::black_box(point).boot_id().len()",
        "std::time::Instant::now()",
        "allocator_capture_active",
    ] {
        assert!(cost.contains(exact), "missing protocol {exact}");
    }
    assert_eq!(cost.matches("Instant::now()").count(), 1);
    assert_eq!(
        read("tools/proc-document-baseline-074/counter.rs"),
        old("tools/proc-document-baseline-074/counter.rs")
    );
    assert_eq!(
        read("tools/sample-boot-baseline-074/controls.rs"),
        old("tools/sample-boot-baseline-074/controls.rs")
    );
}
#[test]
fn inline_boot_reference_has_no_host_capability() {
    fn visit(stream: proc_macro2::TokenStream, ids: &mut Vec<String>) {
        for token in stream {
            match token {
                proc_macro2::TokenTree::Ident(i) => ids.push(i.to_string()),
                proc_macro2::TokenTree::Group(g) => visit(g.stream(), ids),
                _ => {}
            }
        }
    }
    for name in ["probe.rs", "model.rs", "cost.rs"] {
        let mut ids = vec![];
        visit(
            read(&format!("tools/sample-boot-inline-074/{name}"))
                .parse()
                .unwrap(),
            &mut ids,
        );
        for forbidden in [
            "unsafe",
            "File",
            "OpenOptions",
            "KernelPacket",
            "KernelReceiver",
            "clock_gettime",
            "openat",
            "Command",
            "fork",
            "execve",
            "send",
            "ignore",
            "SystemTime",
            "Mutex",
            "Arc",
            "OnceLock",
        ] {
            assert!(!ids.iter().any(|id| id == forbidden), "{name}/{forbidden}");
        }
        if name != "cost.rs" {
            assert!(!ids.iter().any(|id| id == "Instant"));
        }
    }
}
