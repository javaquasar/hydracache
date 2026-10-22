#[path = "support/authority_history_io.rs"]
mod history;
const BASE: &str = "5310113d3a104b97f09325b4ac7f89c7f85263c4";
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
fn tokens(text: &str) -> String {
    text.parse::<proc_macro2::TokenStream>()
        .unwrap()
        .to_string()
}
const ORIGIN: &str = "tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs";
const MODEL: &str = "tools/proc-document-baseline-074/model.rs";
const ADAPTER: &str = "    pub(super) fn read<'a>(&'a mut self, file: &File) -> Result<&'a [u8], KernelError> {\n        self.read_with(|bytes| file.read_at(bytes, 0))\n    }\n";
fn core(origin: &str) -> String {
    assert_eq!(
        origin.matches("pub(super) struct DocumentBuffer {").count(),
        1
    );
    let block = origin
        .split_once("pub(super) struct DocumentBuffer {")
        .unwrap()
        .1
        .split_once("\npub(super) fn alive_read")
        .unwrap()
        .0;
    assert_eq!(block.matches(ADAPTER).count(), 1);
    format!(
        "pub(super) struct DocumentBuffer {{{}",
        block.replace(ADAPTER, "")
    )
}
fn model_core(model: &str) -> &str {
    model
        .split_once("// BEGIN PURE DOCUMENT CORE\n")
        .unwrap()
        .1
        .split_once("// END PURE DOCUMENT CORE")
        .unwrap()
        .0
}

#[test]
fn proc_document_baseline_core_is_complete_except_the_exact_file_adapter() {
    let origin = read(ORIGIN);
    assert_eq!(origin, old(ORIGIN));
    let model = read(MODEL);
    assert_eq!(tokens(&core(&origin)), tokens(model_core(&model)));
    for (before, after) in [
        ("DOC_BYTES + 1", "DOC_BYTES"),
        ("let count = read(&mut self.bytes)?;", "let count = 1;"),
        ("count == 0 || count > DOC_BYTES", "count == 0"),
        ("&self.bytes[..count]", "&self.bytes[..]"),
        ("proc-document-budget", "other-budget"),
        ("let _kernel_walk =", "let _other_walk ="),
    ] {
        assert!(model_core(&model).contains(before));
        assert_ne!(
            tokens(&core(&origin)),
            tokens(&model_core(&model).replace(before, after))
        );
    }
    for changed in [
        origin.replace(ADAPTER, ""),
        origin.replace(ADAPTER, &format!("{ADAPTER}{ADAPTER}")),
    ] {
        assert!(std::panic::catch_unwind(|| core(&changed)).is_err());
    }
}

#[test]
fn proc_document_baseline_changes_no_runtime_source_or_sender_schedule() {
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
    assert!(
        output.status.success(),
        "runtime drift: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let admission = read("tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs");
    assert_eq!(
        admission,
        old("tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs")
    );
    assert_eq!(
        admission
            .matches("let mut scratch = DocumentBuffer::new();")
            .count(),
        1
    );
    assert_eq!(
        admission
            .matches("alive_read(&self.pidfd, self.credentials.pid, |file| scratch.read(file))?;")
            .count(),
        2
    );
    assert_eq!(
        admission
            .matches("projection(scratch.read(file)?, &self.credentials)?")
            .count(),
        1
    );
}

#[test]
fn proc_document_baseline_model_and_controls_have_no_live_or_timing_capability() {
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
        MODEL,
        "tools/proc-document-baseline-074/controls.rs",
        "tools/proc-document-baseline-074/probe.rs",
    ] {
        let mut ids = vec![];
        visit(read(path).parse().unwrap(), &mut ids);
        for forbidden in [
            "unsafe",
            "File",
            "OpenOptions",
            "read_at",
            "Origin",
            "Sender",
            "KernelPacket",
            "Probe",
            "Command",
            "Instant",
            "SystemTime",
            "clock_gettime",
            "openat",
            "fork",
            "execve",
            "send",
            "ignore",
            "Mutex",
            "OnceLock",
            "Arc",
        ] {
            assert!(
                !ids.iter().any(|id| id == forbidden),
                "unexpected {forbidden} in {path}"
            );
        }
    }
    assert_eq!(
        read("tools/proc-document-baseline-074/controls.rs")
            .matches("#[test]")
            .count(),
        5
    );
    assert_eq!(
        read("tools/proc-document-baseline-074/counter.rs")
            .matches("#[test]")
            .count(),
        2
    );
    let manifest = read("tools/proc-document-baseline-074/Cargo.toml");
    assert!(manifest.contains("[workspace]"));
    assert!(!manifest.contains("dependencies"));
}
