#[path = "support/authority_history_io.rs"]
mod history;
const BASE: &str = "49f10265";
fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &str) -> String {
    history::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "")
}
fn old(revision: &str, path: &str) -> String {
    let out = history::command()
        .args(["show", &format!("{revision}:{path}")])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap().replace('\r', "")
}
fn tokens(text: &str) -> String {
    text.parse::<proc_macro2::TokenStream>()
        .unwrap()
        .to_string()
}
#[test]
fn operator_reference_and_frozen_storage_remain_unchanged() {
    let base = "tools/authority-operator-order-074/";
    for file in [
        "probe.rs",
        "operator_controls.rs",
        "Cargo.toml",
        "Cargo.lock",
    ] {
        let path = format!("{base}{file}");
        assert_eq!(read(&path), old(BASE, &path));
    }
    let journal = read(&format!("{base}journal.rs"))
        .replace("#[path = \"operator.rs\"]\npub(super) mod operator;\n", "");
    assert_eq!(
        tokens(&journal),
        tokens(&old(BASE, &format!("{base}journal.rs")))
    );
    let model = read(&format!("{base}model.rs")).replace(
        "#[path = \"candidate_controls.rs\"]\nmod candidate_controls;\n",
        "",
    );
    assert_eq!(
        tokens(&model),
        tokens(&old(BASE, &format!("{base}model.rs")))
    );
    for file in [
        "probe.rs",
        "model.rs",
        "journal.rs",
        "lifecycle.rs",
        "owned.rs",
        "controls.rs",
        "owned_controls.rs",
    ] {
        let path = format!("tools/authority-owned-journal-074/{file}");
        assert_eq!(read(&path), old("bf53c84a", &path));
    }
}
#[test]
fn operator_fresh_validation_precedes_transition_and_publication() {
    let actual = read("tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs");
    assert_eq!(
        actual,
        old(
            "bf53c84a",
            "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs"
        )
    );
    let change = actual
        .split_once("    fn change(\n")
        .unwrap()
        .1
        .split_once("    fn publish(\n")
        .unwrap()
        .0;
    assert!(
        change.find("self.revalidate()?;").unwrap() < change.find(".and_then(transition)").unwrap()
    );
    assert!(change.contains("Ok(next) => self.publish(next, fault)"));
    let source = read("tools/authority-operator-order-074/operator.rs");
    let change = source
        .split_once("    fn candidate_change(\n")
        .unwrap()
        .1
        .split_once("    fn candidate_publish(\n")
        .unwrap()
        .0;
    let expressions = [
        "self.revalidate(faults.change)?;",
        "let next = (||",
        "if self.entries.len() >= 128",
        "self.candidate_publish(refused, epoch, faults)",
    ];
    let mut position = 0;
    for expression in expressions {
        let next = change.find(expression).unwrap();
        assert!(next >= position);
        position = next;
    }
    assert_eq!(change.matches(".revalidate(").count(), 1);
    let publish = source.split_once("    fn candidate_publish(\n").unwrap().1;
    let mut position = 0;
    for expression in [
        "if self.failed",
        "self.revalidate(faults.publish)?;",
        "self.add_entry();",
        "self.refused = refused;",
        "self.epoch = epoch;",
        "self.revalidate(faults.post)?;",
    ] {
        let next = publish.find(expression).unwrap();
        assert!(next >= position);
        position = next;
    }
    assert_eq!(publish.matches(".revalidate(").count(), 2);
    assert!(!source.contains("begin_round"));
    assert!(!source.contains("unsafe"));
}
#[test]
fn operator_image_has_no_clock_os_or_transport_capability() {
    fn visit(stream: proc_macro2::TokenStream, names: &mut Vec<String>) {
        for token in stream {
            match token {
                proc_macro2::TokenTree::Ident(i) => names.push(i.to_string()),
                proc_macro2::TokenTree::Group(g) => visit(g.stream(), names),
                _ => {}
            }
        }
    }
    for file in [
        "probe.rs",
        "model.rs",
        "journal.rs",
        "operator_controls.rs",
        "operator.rs",
        "candidate_controls.rs",
    ] {
        let mut names = vec![];
        visit(
            read(&format!("tools/authority-operator-order-074/{file}"))
                .parse()
                .unwrap(),
            &mut names,
        );
        for forbidden in [
            "unsafe",
            "fs",
            "OpenOptions",
            "Command",
            "Instant",
            "SystemTime",
            "libc",
            "socket",
            "spawn",
            "send",
            "ignore",
            "Mutex",
            "OnceLock",
        ] {
            assert!(!names.iter().any(|n| n == forbidden), "{file}/{forbidden}");
        }
    }
    let out = history::command()
        .args([
            "diff",
            "--exit-code",
            "bf53c84a",
            "--",
            "tools/long-run-supervisor-074/src",
        ])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(out.status.success());
}
