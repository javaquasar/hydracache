#[path = "../../../tools/authority-rust-history-074/contract.rs"]
mod contract;
#[path = "support/current_tree_input.rs"]
mod current_inputs;
#[path = "support/authority_history_io.rs"]
mod history;
#[path = "../../../tools/authority-runtime-074/gate.rs"]
mod runtime;
#[path = "../../../tools/authority-runtime-074/io.rs"]
mod runtime_io;
#[path = "../../../tools/authority-storage-source-074/contract.rs"]
mod storage_contract;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

fn read(path: &str) -> String {
    current_inputs::read(path)
}
fn old(path: &str) -> String {
    String::from_utf8(history::frozen_bytes(path).unwrap())
        .unwrap()
        .replace("\r\n", "\n")
}
fn git(args: &[&str]) -> String {
    let result = std::process::Command::new("git")
        .arg("--no-replace-objects")
        .args(args)
        .current_dir(history::root())
        .output()
        .unwrap();
    assert!(result.status.success(), "current gate Git query failed");
    String::from_utf8(result.stdout).unwrap()
}
fn guard(path: &str) -> Option<(usize, usize)> {
    contract::GUARDS.iter().find_map(|(name, reads, commands)| {
        (path == format!("crates/xtask/tests/{name}.rs")).then_some((*reads, *commands))
    })
}
fn accepts_current(path: &str, current: &str, baseline: &str) -> bool {
    match guard(path) {
        Some((reads, commands)) => contract::accepts(baseline, current, reads, commands),
        None => current == baseline,
    }
}
fn protected(path: &str, pre_existing: bool) -> bool {
    (pre_existing && (path.starts_with("tools/") || path.starts_with("crates/")))
        || path.starts_with("tools/long-run-supervisor-074/src/")
        || path.starts_with("tools/long-run-supervisor-074/tests/")
        || matches!(
            path,
            "Cargo.toml"
                | "Cargo.lock"
                | "docs/COMPAT.md"
                | "docs/plans/releases.toml"
                | "docs/testing/gated-test-registry.toml"
        )
        || (pre_existing
            && path.starts_with("docs/testing/performance/0.74/")
            && path.ends_with(".toml"))
}
fn words(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).into()).collect()
}

#[test]
fn all_thirty_guards_preserve_complete_assertions_under_exact_io_substitutions() {
    assert_eq!(contract::SOURCE, history::SOURCE);
    let mut test_names = BTreeSet::new();
    let mut count = 0;
    for (name, reads, commands) in contract::GUARDS {
        let path = format!("crates/xtask/tests/{name}.rs");
        let original = old(&path);
        assert!(
            contract::accepts(&original, &read(&path), reads, commands),
            "full guard {path}"
        );
        let parsed = syn::parse_file(&original).unwrap();
        for item in parsed.items {
            if let syn::Item::Fn(function) = item {
                if function
                    .attrs
                    .iter()
                    .any(|attr| attr.path().is_ident("test"))
                {
                    assert!(test_names.insert(format!("{name}::{}", function.sig.ident)));
                    count += 1;
                }
            }
        }
    }
    assert_eq!(count, 79);
}

#[test]
fn independent_current_tree_gate_refuses_unapproved_source_and_runtime_drift() {
    let previous: BTreeSet<_> = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        contract::SOURCE,
        "--",
        "tools",
        "crates",
        "Cargo.toml",
        "Cargo.lock",
        "docs/COMPAT.md",
        "docs/plans/releases.toml",
        "docs/testing/gated-test-registry.toml",
        "docs/testing/performance/0.74",
    ])
    .lines()
    .map(str::to_owned)
    .collect();
    let changed = git(&[
        "diff",
        "--name-only",
        contract::SOURCE,
        "--",
        "tools",
        "crates",
        "Cargo.toml",
        "Cargo.lock",
        "docs/COMPAT.md",
        "docs/plans/releases.toml",
        "docs/testing/gated-test-registry.toml",
        "docs/testing/performance/0.74",
    ]);
    for path in changed
        .lines()
        .filter(|path| protected(path, previous.contains(*path)))
    {
        assert!(history::safe_path(path));
        if !previous.contains(path) && current_inputs::known_addition(path) {
            current_inputs::require_addition(path);
            continue;
        }
        assert!(
            previous.contains(path),
            "unapproved new runtime source {path}"
        );
        let metadata = std::fs::symlink_metadata(history::root().join(path)).unwrap();
        assert!(metadata.is_file() && !metadata.file_type().is_symlink());
        assert!(
            accepts_current(path, &read(path), &old(path)),
            "current source drift {path}"
        );
    }
    assert!(
        git(&[
            "ls-files",
            "--others",
            "--exclude-standard",
            "--",
            "tools/long-run-supervisor-074/src",
            "tools/long-run-supervisor-074/tests"
        ])
        .is_empty(),
        "untracked runtime source is not admitted"
    );
}

#[test]
fn historical_reads_are_fixed_and_never_fall_back_to_current_artifacts() {
    let fixture = "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs";
    assert_eq!(
        history::read_to_string(history::root().join(fixture)).unwrap(),
        old(fixture)
    );
    let candidate = "tools/authority-storage-source-074/candidate.rs";
    assert!(history::root().join(candidate).is_file());
    assert!(
        history::read_to_string(history::root().join(candidate)).is_err(),
        "no current fallback"
    );
    assert!(history::frozen_bytes("tools/missing-history-input.rs").is_err());
}

#[test]
fn diff_translation_adds_only_fixed_rhs_and_retains_all_paths_and_flags() {
    for flag in ["--name-only", "--exit-code"] {
        let input = words(&[
            "diff",
            flag,
            "bf53c84a",
            "--",
            "tools/long-run-supervisor-074/src",
            "tools/long-run-supervisor-074/tests",
        ]);
        let mut expected = input.clone();
        expected.insert(3, history::SOURCE.into());
        assert_eq!(history::translated(&input).unwrap(), expected);
    }
    for input in [
        words(&["show", "bf53c84a:Cargo.toml"]),
        words(&[
            "ls-tree",
            "-r",
            "--name-only",
            "bf53c84a",
            "--",
            "tools/long-run-supervisor-074/src",
        ]),
    ] {
        assert_eq!(history::translated(&input).unwrap(), input);
    }
    let actual = history::command()
        .args([
            "diff",
            "--exit-code",
            history::SOURCE,
            "--",
            "tools/long-run-supervisor-074/src",
        ])
        .current_dir(history::root())
        .output()
        .unwrap();
    assert!(actual.status.success() && actual.stdout.is_empty());
}

#[test]
fn unsupported_commands_revisions_scopes_and_cwd_refuse_before_execution() {
    for input in [
        words(&["status"]),
        words(&["show", "HEAD:Cargo.toml"]),
        words(&["show", "bf53c84a:../secret"]),
        words(&["diff", "--name-only", "bf53c84a", "HEAD", "--", "tools"]),
        words(&["diff", "--name-only", "bf53c84a", "--"]),
        words(&["diff", "--exit-code", "bf53c84a", "--", ":(exclude)tools"]),
        words(&["diff", "--quiet", "bf53c84a", "--", "tools"]),
        words(&["ls-tree", "-r", "--name-only", "bf53c84a", "--", "/"]),
    ] {
        assert!(
            history::translated(&input).is_err(),
            "unsupported {input:?}"
        );
    }
    assert!(history::command()
        .args(["show", "bf53c84a:Cargo.toml"])
        .current_dir(history::root().join("tools"))
        .output()
        .is_err());
}

#[test]
fn unsafe_names_outside_paths_and_missing_inputs_do_not_open_current_files() {
    for path in [
        "",
        "/root",
        "../secret",
        "tools/../secret",
        "tools//file",
        "tools/./file",
        "tools/file.",
        "tools/NUL.rs",
        "tools/COM1.txt",
        "tools/a:b",
        "tools/a\\b",
        "tools/a b",
        "tools/a\nfile",
    ] {
        assert!(!history::safe_path(path), "unsafe {path:?}");
    }
    assert!(history::read_to_string(history::root().join("../outside.rs")).is_err());
    assert!(history::read_to_string("Cargo.toml").is_err());
}

#[test]
fn archived_success_does_not_admit_a_current_candidate_or_unrelated_mutation() {
    for path in [
        "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs",
        "tools/long-run-supervisor-074/tests/support/authority_owners_controls.rs",
        "tools/authority-owned-journal-074/model.rs",
        "crates/xtask/tests/support/authority_owners_guard.rs",
    ] {
        let original = old(path);
        assert!(protected(path, true));
        assert!(accepts_current(path, &original, &original));
        assert!(!accepts_current(
            path,
            &format!("{original}\nfn unexpected() {{}}"),
            &original
        ));
        assert_eq!(old(path), original, "historical side stays fixed");
    }
    let fixture = "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs";
    assert!(
        !accepts_current(
            fixture,
            &read("tools/authority-storage-source-074/candidate.rs"),
            &old(fixture)
        ),
        "unapplied storage artifact does not open runtime admission"
    );
    for path in [
        "tools/long-run-supervisor-074/src/new.rs",
        "tools/long-run-supervisor-074/tests/new.rs",
    ] {
        assert!(protected(path, false));
    }
    assert!(!protected("docs/testing/performance/0.74/report.md", true));
    assert!(protected(
        "docs/testing/performance/0.74/qualification-manifest.toml",
        true
    ));
}

#[test]
fn guard_assertion_test_and_io_mutations_cannot_hide_behind_history() {
    for (name, reads, commands) in contract::GUARDS {
        let path = format!("crates/xtask/tests/{name}.rs");
        let original = old(&path);
        let current = contract::migrated(&original, reads, commands);
        for changed in [
            current.replacen("#[test]", "#[test]\n#[ignore]", 1),
            current.replacen("#[test]", "", 1),
            current.replacen("history::read_to_string", "std::fs::read_to_string", 1),
            current.replacen(
                "history::command()",
                "std::process::Command::new(\"git\")",
                1,
            ),
            format!("{current}\nfn extra_assertion_bypass() {{}}"),
            format!("{}{}", contract::PREFIX, current),
        ] {
            assert!(
                !contract::accepts(&original, &changed, reads, commands),
                "whole guard {path}"
            );
        }
        let changed = current.replacen("assert", "silenced_assert", 1);
        assert_ne!(changed, current);
        assert!(!contract::accepts(&original, &changed, reads, commands));
    }
}

#[test]
fn every_allowed_replacement_is_an_actual_io_call_not_a_literal_or_assertion() {
    #[derive(Default)]
    struct Calls {
        reads: usize,
        commands: usize,
    }
    impl<'ast> syn::visit::Visit<'ast> for Calls {
        fn visit_macro(&mut self, mac: &'ast syn::Macro) {
            use syn::parse::Parser;
            // syn leaves macro arguments as tokens. The baseline has a real Git
            // call in assert_eq!'s block expression; inspect parsed expressions,
            // never string-literal contents or an arbitrary textual expansion.
            if let Ok(arguments) =
                syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated
                    .parse2(mac.tokens.clone())
            {
                for argument in &arguments {
                    let mut nested = Calls::default();
                    syn::visit::Visit::visit_expr(&mut nested, argument);
                    self.reads += nested.reads;
                    self.commands += nested.commands;
                }
            }
        }
        fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
            if let syn::Expr::Path(path) = call.func.as_ref() {
                let names: Vec<_> = path
                    .path
                    .segments
                    .iter()
                    .map(|part| part.ident.to_string())
                    .collect();
                if names == ["std", "fs", "read_to_string"] {
                    self.reads += 1;
                }
                if names == ["std", "process", "Command", "new"] {
                    assert_eq!(call.args.len(), 1);
                    let syn::Expr::Lit(value) = &call.args[0] else {
                        panic!("literal Git only")
                    };
                    let syn::Lit::Str(value) = &value.lit else {
                        panic!("literal Git only")
                    };
                    assert_eq!(value.value(), "git");
                    self.commands += 1;
                }
            }
            syn::visit::visit_expr_call(self, call);
        }
    }
    let mut totals = (0, 0);
    for (name, reads, commands) in contract::GUARDS {
        let parsed = syn::parse_file(&old(&format!("crates/xtask/tests/{name}.rs"))).unwrap();
        let mut calls = Calls::default();
        syn::visit::Visit::visit_file(&mut calls, &parsed);
        assert_eq!(
            (calls.reads, calls.commands),
            (reads, commands),
            "IO call discovery {name}"
        );
        totals.0 += reads;
        totals.1 += commands;
    }
    assert_eq!(totals, (32, 40));
}

#[test]
fn qualification_raw_identity_and_previous_candidate_guard_remain_independent() {
    let bytes = std::fs::read(
        history::root().join("docs/testing/performance/0.74/qualification-manifest.toml"),
    )
    .unwrap();
    let digest = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(
        digest,
        "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
    );
    for path in [
        "tools/authority-storage-source-074/candidate.rs",
        "tools/authority-storage-source-074/observer.rs",
        "tools/authority-storage-source-074/contract.rs",
        "crates/xtask/tests/authority_storage_source_074.rs",
    ] {
        let fixed = git(&[
            "show",
            &format!("f193fabae86c9bd7a2e778a9463a43984834deb5:{path}"),
        ])
        .replace('\r', "");
        assert_eq!(
            read(path),
            fixed,
            "prior current-candidate guard is not rewritten: {path}"
        );
    }
}
