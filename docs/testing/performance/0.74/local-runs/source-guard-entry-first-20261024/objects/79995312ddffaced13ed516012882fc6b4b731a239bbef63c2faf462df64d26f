#[path = "../../../tools/authority-runtime-074/io.rs"]
mod bound_io;
use bound_io::reviewed_inputs as contract;
#[allow(dead_code)]
#[path = "../../../tools/authority-runtime-074/gate.rs"]
mod runtime;
#[path = "../../../tools/authority-storage-source-074/contract.rs"]
mod storage_contract;

#[test]
fn historical_input_reader_is_one_exact_additive_verified_branch() {
    let path = "tools/authority-runtime-074/io.rs";
    let old = runtime::frozen("81f4add2", path);
    let expected = old.replacen("use std::{io, path::Path};", "use std::{io, path::Path};\n#[path = \"../historical-input-view-074/contract.rs\"]\npub(crate) mod reviewed_inputs;", 1)
        .replacen("    Ok(super::runtime::view(&relative, raw))", "    if reviewed_inputs::PATHS.contains(&relative.as_str()) {\n        return Ok(reviewed_inputs::predecessor(&relative, &raw));\n    }\n    Ok(super::runtime::view(&relative, raw))", 1);
    let actual = std::fs::read_to_string(runtime::root().join(path)).unwrap();
    assert!(storage_contract::accepts(&expected, &actual));
    assert_ne!(expected, old);
    for file in contract::PATHS {
        assert_eq!(
            bound_io::read_to_string(runtime::root().join(file)).unwrap(),
            contract::frozen(contract::PREDECESSOR, file)
        );
    }
    assert!(bound_io::read_to_string(std::env::temp_dir()).is_err());
    for (before, after) in [
        (
            "reviewed_inputs::PATHS.contains(&relative.as_str())",
            "true",
        ),
        ("reviewed_inputs::predecessor(&relative, &raw)", "raw"),
    ] {
        assert!(actual.contains(before));
        assert!(!storage_contract::accepts(
            &expected,
            &actual.replacen(before, after, 1)
        ));
    }
}

#[test]
fn historical_inputs_require_the_complete_reviewed_current_file_before_predecessor() {
    for path in contract::PATHS {
        let current = std::fs::read_to_string(contract::root().join(path)).unwrap();
        assert!(
            contract::accepts(path, &current),
            "raw current input: {path}"
        );
        assert_eq!(
            contract::predecessor(path, &current),
            contract::frozen(contract::PREDECESSOR, path)
        );
        assert!(
            !contract::accepts(path, &contract::frozen(contract::PREDECESSOR, path)),
            "old input is not current: {path}"
        );
        for changed in [
            String::new(),
            format!("{current}\nUNREVIEWED_CHANGE\n"),
            current.chars().skip(1).collect(),
        ] {
            assert!(!contract::accepts(path, &changed));
            assert!(std::panic::catch_unwind(|| contract::predecessor(path, &changed)).is_err());
        }
    }
    assert!(contract::expected("tools/long-run-supervisor-074/src/unrecognized.rs").is_none());
    assert!(!contract::accepts(
        "tools/long-run-supervisor-074/src/unrecognized.rs",
        "anything"
    ));
}

#[test]
fn historical_input_list_covers_all_changed_old_files_except_the_existing_exact_authority_gate() {
    let git = |args: &[&str]| {
        let output = std::process::Command::new("git")
            .arg("--no-replace-objects")
            .args(args)
            .current_dir(contract::root())
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap()
    };
    let scope = [
        "tools/long-run-supervisor-074/src",
        "tools/long-run-supervisor-074/tests",
        "docs/COMPAT.md",
        "docs/testing/gated-test-registry.toml",
    ];
    let old = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        contract::PREDECESSOR,
        "--",
        scope[0],
        scope[1],
        scope[2],
        scope[3],
    ]);
    let changed = git(&[
        "diff",
        "--name-only",
        contract::PREDECESSOR,
        contract::REVIEWED,
        "--",
        scope[0],
        scope[1],
        scope[2],
        scope[3],
    ]);
    let authority = [
        "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs",
        "tools/long-run-supervisor-074/src/diagnostic_authority_fixture_tests.rs",
        "tools/long-run-supervisor-074/tests/support/authority_owners_controls.rs",
        "tools/long-run-supervisor-074/tests/support/authority_breakdown_controls.rs",
        "tools/long-run-supervisor-074/tests/support/authority_leaf_name_alloc.rs",
        "tools/long-run-supervisor-074/tests/support/authority_leaf_name_controls.rs",
        "tools/long-run-supervisor-074/tests/support/authority_open_name_alloc.rs",
        "tools/long-run-supervisor-074/tests/support/authority_open_name_controls.rs",
    ];
    let expected: Vec<_> = changed
        .lines()
        .filter(|p| old.lines().any(|old| old == *p) && !authority.contains(p))
        .collect();
    assert_eq!(expected, contract::PATHS);
}
