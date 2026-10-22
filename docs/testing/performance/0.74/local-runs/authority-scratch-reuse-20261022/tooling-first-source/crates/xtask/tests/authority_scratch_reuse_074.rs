const REFERENCE: &str = "187ac5c7";
fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "")
}
fn old(revision: &str, path: &str) -> String {
    let output = std::process::Command::new("git")
        .args(["show", &format!("{revision}:{path}")])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().replace('\r', "")
}
fn tokens(text: &str) -> String {
    text.parse::<proc_macro2::TokenStream>()
        .unwrap()
        .to_string()
}
#[test]
fn scratch_reference_receivers_and_cost_helper_remain_frozen() {
    let base = "tools/authority-scratch-reuse-074/";
    for file in ["probe.rs", "cost.rs", "Cargo.toml", "Cargo.lock"] {
        let path = format!("{base}{file}");
        assert_eq!(read(&path), old(REFERENCE, &path));
    }
    let old_model = old(REFERENCE, &format!("{base}model.rs"));
    assert_eq!(read(&format!("{base}model.rs")), format!("{old_model}\n#[path = \"candidate.rs\"]\nmod candidate;\n#[path = \"comparisons.rs\"]\nmod comparisons;\n"));
    assert_eq!(
        read(&format!("{base}probe.rs")),
        old("84b5e62c", "tools/authority-read-baseline-074/probe.rs")
    );
}
#[test]
fn scratch_only_substitutes_storage_not_fresh_validation() {
    let candidate = read("tools/authority-scratch-reuse-074/candidate.rs");
    let original = read("tools/authority-read-baseline-074/model.rs");
    let original_read = original
        .split_once("        let mut file = open_at")
        .unwrap()
        .1
        .split_once("    fn revalidate(")
        .unwrap()
        .0
        .trim_end()
        .strip_suffix('}')
        .unwrap();
    let candidate_read = candidate
        .split_once("        let mut file = open_at")
        .unwrap()
        .1
        .trim_end()
        .strip_suffix('}')
        .unwrap()
        .trim_end()
        .strip_suffix('}')
        .unwrap();
    let storage = "let mut raw = match slot.take() { Some(mut raw) if raw.capacity() == before.len() as usize => { raw.clear(); raw } _ => Vec::with_capacity(before.len() as usize), };";
    assert_eq!(
        tokens(candidate_read).replace(
            &tokens(storage),
            &tokens("let mut raw = Vec::with_capacity(before.len() as usize);")
        ),
        tokens(original_read)
    );
    let validate = candidate
        .split_once("        if Stamp::of(&self.owner.file.metadata()?)")
        .unwrap()
        .1
        .split_once("    fn read(")
        .unwrap()
        .0
        .trim_end()
        .strip_suffix('}')
        .unwrap();
    let retain = "if named.2.capacity() == named.1.len as usize && named.2.capacity() <= budget { self.buffer = Some(named.2); }";
    let expected = original
        .split_once("        if Stamp::of(&self.file.metadata()?)")
        .unwrap()
        .1
        .split_once("// END LEAF CORE")
        .unwrap()
        .0
        .trim_end()
        .strip_suffix('}')
        .unwrap()
        .trim_end()
        .strip_suffix('}')
        .unwrap();
    let adjusted = validate
        .replace("self.owner", "self")
        .replace(", &mut self.buffer", "");
    assert_eq!(
        tokens(&adjusted).replace(&tokens(retain), ""),
        tokens(expected)
    );
    assert!(tokens(&candidate).contains(&tokens("if result.is_err() { self.buffer = None; }")));
    assert!(tokens(&candidate).contains(&tokens("owner: &'a Leaf")));
    assert!(!candidate.contains("Clone"));
}
#[test]
fn scratch_image_has_no_os_transport_launch_or_admission_clock() {
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
        "candidate.rs",
        "comparisons.rs",
        "cost.rs",
    ] {
        let mut names = vec![];
        visit(
            read(&format!("tools/authority-scratch-reuse-074/{file}"))
                .parse()
                .unwrap(),
            &mut names,
        );
        for forbidden in [
            "unsafe",
            "fs",
            "OpenOptions",
            "Command",
            "SystemTime",
            "libc",
            "socket",
            "spawn",
            "send",
            "ignore",
            "Arc",
            "Mutex",
            "OnceLock",
        ] {
            assert!(!names.iter().any(|n| n == forbidden), "{file}/{forbidden}");
        }
        if file != "cost.rs" {
            assert!(!names.iter().any(|n| n == "Instant"));
        }
    }
    let output = std::process::Command::new("git")
        .args([
            "diff",
            "--exit-code",
            "84b5e62c",
            "--",
            "tools/long-run-supervisor-074/src",
        ])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(output.status.success());
}
