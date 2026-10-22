#[path = "support/authority_history_io.rs"]
mod history;
const BASE: &str = "79dcf64bee22d801dc983ef62b4c2fefa44a0988";
const ORIGIN: &str = "tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs";
const MODEL: &str = "tools/sample-boot-baseline-074/model.rs";
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
fn between<'a>(text: &'a str, start: &str, end: &str) -> &'a str {
    assert_eq!(text.matches(start).count(), 1);
    text.split_once(start).unwrap().1.split_once(end).unwrap().0
}
fn marked<'a>(text: &'a str, name: &str) -> &'a str {
    between(
        text,
        &format!("// BEGIN {name} CORE\n"),
        &format!("// END {name} CORE"),
    )
}
fn sample(text: &str) -> String {
    format!(
        "pub(super) fn sample{}",
        between(
            text,
            "pub(super) fn sample",
            "\n    pub(super) fn require_worker_context"
        )
    )
}
fn prelude(text: &str, end: &str) -> String {
    between(text, "->", end)
        .split_once('{')
        .unwrap()
        .1
        .to_owned()
}

#[test]
fn sample_boot_baseline_matches_complete_point_sample_decoder_and_prelude() {
    let origin = read(ORIGIN);
    assert_eq!(origin, old(ORIGIN));
    let model = read(MODEL);
    let point = format!(
        "#[derive(Debug, Clone)]{}",
        between(
            &origin,
            "#[derive(Debug, Clone)]",
            "\npub(super) struct Origin"
        )
    );
    assert_eq!(tokens(&point), tokens(marked(&model, "POINT")));
    assert_eq!(tokens(&sample(&origin)), tokens(marked(&model, "SAMPLE")));
    let decoder = format!(
        "fn boot_value{}",
        between(&origin, "fn boot_value", "\nfn clock")
    );
    assert_eq!(tokens(&decoder), tokens(marked(&model, "DECODER")));
    let stage = between(&model, "fn stage_prelude", "\n}\n\n// BEGIN DECODER");
    assert_eq!(
        tokens(&prelude(
            marked(&model, "SAMPLE"),
            "        Ok(KernelTimePoint {"
        )),
        tokens(&prelude(stage, "        Ok((boottime, realtime_ns))"))
    );
    for (before, after) in [
        ("self.revalidate()?;", ""),
        (
            "clock(libc::CLOCK_BOOTTIME)?",
            "clock(libc::CLOCK_REALTIME)?",
        ),
        ("boot: self.boot.clone(),", "boot: String::new(),"),
        ("Kind::Sample", "Kind::Other"),
        ("realtime_ns,", "realtime_ns: 0,"),
    ] {
        assert!(marked(&model, "SAMPLE").contains(before));
        assert_ne!(
            tokens(&sample(&origin)),
            tokens(&marked(&model, "SAMPLE").replace(before, after))
        );
    }
    for (before, after) in [
        ("boot: String", "boot: &'static str"),
        ("1_000_000_000", "1_000_000"),
        ("Debug, Clone", "Debug"),
    ] {
        assert_ne!(
            tokens(&point),
            tokens(&marked(&model, "POINT").replace(before, after))
        );
    }
    for (before, after) in [
        ("count != 37", "count != 36"),
        ("b'a'..=b'f'", "b'A'..=b'F'"),
        ("[8, 13, 18, 23]", "[8, 13, 18]"),
        ("boot-utf8", "other-utf8"),
    ] {
        assert_ne!(
            tokens(&decoder),
            tokens(&marked(&model, "DECODER").replace(before, after))
        );
    }
}

#[test]
fn sample_boot_baseline_preserves_runtime_and_reuses_the_exact_counter() {
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
    let counter = "tools/proc-document-baseline-074/counter.rs";
    assert_eq!(read(counter), old(counter));
    let probe = read("tools/sample-boot-baseline-074/probe.rs");
    assert_eq!(
        probe
            .matches("#[path = \"../proc-document-baseline-074/counter.rs\"]")
            .count(),
        1
    );
    assert!(read("tools/sample-boot-baseline-074/Cargo.toml").contains("[workspace]"));
    assert!(!read("tools/sample-boot-baseline-074/Cargo.toml").contains("dependencies"));
}

#[test]
fn sample_boot_baseline_has_no_live_or_timing_capabilities() {
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
        "tools/sample-boot-baseline-074/probe.rs",
        "tools/sample-boot-baseline-074/controls.rs",
    ] {
        let mut ids = vec![];
        visit(read(path).parse().unwrap(), &mut ids);
        for forbidden in [
            "unsafe",
            "File",
            "OpenOptions",
            "KernelPacket",
            "KernelReceiver",
            "read_at",
            "openat",
            "clock_gettime",
            "Instant",
            "SystemTime",
            "Command",
            "execve",
            "fork",
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
        read("tools/sample-boot-baseline-074/controls.rs")
            .matches("#[test]")
            .count(),
        5
    );
}
