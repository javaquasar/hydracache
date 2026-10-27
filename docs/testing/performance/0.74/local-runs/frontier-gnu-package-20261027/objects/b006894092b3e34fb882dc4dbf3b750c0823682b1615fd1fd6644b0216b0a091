#[path = "support/authority_history_io.rs"]
mod history;
const BASE: &str = "94114c62";
fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &str) -> String {
    history::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "")
}
fn old(path: &str) -> String {
    let output = history::command()
        .args(["show", &format!("{BASE}:{path}")])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().replace('\r', "")
}
fn expected() -> String {
    let source = old("tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs");
    let stamp = source
        .split_once("#[derive(Clone, PartialEq, Eq)]")
        .unwrap()
        .1
        .split_once("    fn directory_matches(")
        .unwrap()
        .0;
    let leaf = source
        .split_once("struct Leaf {")
        .unwrap()
        .1
        .split_once("struct Entry {")
        .unwrap()
        .0;
    let regular = source
        .split_once("fn regular(")
        .unwrap()
        .1
        .split_once("fn open_at(")
        .unwrap()
        .0;
    format!("//! Whole original bodies with synthetic receivers; no storage candidate.\nuse crate::{{open_at, AuthorityError, File, Metadata}};\nuse std::io::Read;\n\n// BEGIN STAMP CORE\n#[derive(Clone, PartialEq, Eq)]{stamp}}}\n// END STAMP CORE\n\n// BEGIN LEAF CORE\nstruct Leaf {{{leaf}// END LEAF CORE\n\n// BEGIN REGULAR CORE\nfn regular({regular}// END REGULAR CORE\n\n#[path = \"controls.rs\"]\nmod controls;\n")
}
#[test]
fn authority_read_baseline_copies_complete_bodies_and_leaves_runtime_unchanged() {
    assert_eq!(
        read("tools/authority-read-baseline-074/model.rs"),
        expected()
    );
    assert_eq!(
        read("tools/proc-document-baseline-074/counter.rs"),
        old("tools/proc-document-baseline-074/counter.rs")
    );
    let output = history::command()
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
    assert!(output.status.success());
}
#[test]
fn authority_read_baseline_adapter_rejects_omissions_bounds_and_ownership_drift() {
    let expected = expected();
    for (before, after) in [
        ("open_at(dir, name, false)?", "open_at(dir, name, true)?"),
        ("regular(&before, uid, gid, 0o400)?;", ""),
        ("dir.metadata()?.dev()", "before.dev()"),
        (
            "before.len() > budget as u64",
            "before.len() >= budget as u64",
        ),
        (".take(budget as u64 + 1)", ".take(before.len())"),
        (".read_to_end(&mut raw)?", ".read_exact(&mut raw)?"),
        ("raw.len() as u64 != stamp.len", "false"),
        (
            "named.1 != self.stamp || named.2 != self.raw",
            "self.raw != self.raw",
        ),
        ("let named = Self::read", "let named = Self::cached_read"),
        ("mtime_ns: m.mtime_nsec()", "mtime_ns: 0"),
        ("Ok((file, stamp, raw))", "Ok((file, stamp, Vec::new()))"),
    ] {
        assert!(expected.contains(before));
        assert_ne!(expected.replace(before, after), expected);
    }
}
#[test]
fn authority_read_baseline_receivers_have_no_os_clock_or_launch_capability() {
    fn visit(tokens: proc_macro2::TokenStream, names: &mut Vec<String>) {
        for token in tokens {
            match token {
                proc_macro2::TokenTree::Ident(i) => names.push(i.to_string()),
                proc_macro2::TokenTree::Group(g) => visit(g.stream(), names),
                _ => {}
            }
        }
    }
    for file in ["probe.rs", "model.rs", "controls.rs"] {
        let text = read(&format!("tools/authority-read-baseline-074/{file}"));
        let mut names = vec![];
        visit(text.parse().unwrap(), &mut names);
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
            "Arc",
            "Mutex",
            "OnceLock",
        ] {
            assert!(!names.iter().any(|n| n == forbidden), "{file}/{forbidden}");
        }
    }
    assert!(read("tools/authority-read-baseline-074/probe.rs").contains("struct File {"));
}
