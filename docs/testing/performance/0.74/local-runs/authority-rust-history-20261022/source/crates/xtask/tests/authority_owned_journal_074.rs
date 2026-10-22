#[path = "support/authority_history_io.rs"]
mod history;
const BASE: &str = "227f1559";
fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &str) -> String {
    history::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "")
}
fn old(path: &str) -> String {
    let out = history::command()
        .args(["show", &format!("{BASE}:{path}")])
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
fn owned_journal_keeps_whole_reference_bodies_receiver_and_main() {
    let base = "tools/authority-owned-journal-074/";
    for file in ["probe.rs", "controls.rs", "Cargo.toml", "Cargo.lock"] {
        let path = format!("{base}{file}");
        assert_eq!(read(&path), old(&path));
    }
    let model = read(&format!("{base}model.rs"));
    let reference = read("tools/authority-read-baseline-074/model.rs");
    assert_eq!(
        tokens(model.split("#[path = \"controls.rs\"]").next().unwrap()),
        tokens(reference.split("#[path = \"controls.rs\"]").next().unwrap())
    );
    let expected =
        old(&format!("{base}journal.rs")).replace("slot % 2 == 0", "slot.is_multiple_of(2)");
    let actual = read(&format!("{base}journal.rs"))
        .replace("#[path = \"lifecycle.rs\"]\nmod lifecycle;\n", "");
    assert_eq!(tokens(&actual), tokens(&expected));
}
#[test]
fn owned_storage_only_changes_borrowed_owner_to_value_and_adds_eviction() {
    let prior = read("tools/authority-scratch-reuse-074/candidate.rs");
    let expected = prior
        .replace("struct Scratch<'a>", "struct Owned")
        .replace("owner: &'a Leaf", "owner: Leaf")
        .replace("impl<'a> Scratch<'a>", "impl Owned")
        .replace("fn new(owner: &'a Leaf)", "fn new(owner: Leaf)")
        .replace("        self.owner\n", "        &self.owner\n");
    let actual = read("tools/authority-owned-journal-074/owned.rs");
    assert_eq!(
        tokens(actual.split("impl super::journal::Storage").next().unwrap()),
        tokens(&expected)
    );
    assert!(tokens(&actual).contains(&tokens("fn evict(&mut self) { self.buffer = None; }")));
    let journal = read("tools/authority-owned-journal-074/journal.rs");
    assert!(tokens(&journal).contains(&tokens(
        "pub fn latch(&mut self) {self.failed = true; self.evict();}"
    )));
    assert!(tokens(&journal).contains(&tokens(
        "for entry in &mut self.entries { entry.record.evict(); entry.marker.evict(); }"
    )));
}
#[test]
fn owned_model_has_no_os_clock_transport_or_launch_path() {
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
        "controls.rs",
        "owned.rs",
        "lifecycle.rs",
        "owned_controls.rs",
    ] {
        let mut names = vec![];
        visit(
            read(&format!("tools/authority-owned-journal-074/{file}"))
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
        if file == "owned.rs" {
            assert!(!names.iter().any(|n| n == "RefCell" || n == "Clone"));
        }
    }
    let out = history::command()
        .args([
            "diff",
            "--exit-code",
            "72c28a25",
            "--",
            "tools/long-run-supervisor-074/src",
        ])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(out.status.success());
}
