//! Exact single-provider fixture view, never a whole phase rollback.
#[path = "../../../tools/historical-blob-batch-074/mod.rs"]
pub mod census_runtime;
#[path = "../../../tools/package-closure-inputs-074/contract.rs"]
mod closure_inputs;
#[path = "support/census_provider_input.rs"]
mod provider_input;

const BASE: &str = "c91068bfa8b297bb075b63457da235aaa9c6232e";
const FILE: &str = "crates/xtask/tests/held_complete_role_census_074.rs";

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn census_reader_preserves_every_original_assertion_and_only_adapts_one_provider() {
    let output = std::process::Command::new("git")
        .arg("--no-replace-objects")
        .args(["show", &format!("{BASE}:{FILE}")])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(output.status.success());
    let old = String::from_utf8(output.stdout)
        .unwrap()
        .replace("\r\n", "\n");
    let hook = "#[path = \"support/held_role_census_guard.rs\"]\nmod guard;\n";
    let imports = "#[path = \"support/census_provider_input.rs\"]\nmod provider_input;\n";
    let before = "    std::fs::read_to_string(\n";
    let after = "    let raw = std::fs::read_to_string(\n";
    let end = "    .replace('\\r', \"\")\n}";
    assert_eq!(old.matches(hook).count(), 1);
    assert_eq!(old.matches(before).count(), 1);
    assert_eq!(old.matches(end).count(), 1);
    let runtime =
        "#[path = \"../../../tools/historical-blob-batch-074/mod.rs\"]\npub mod census_runtime;\n";
    let expected = old
        .replacen(hook, &format!("{hook}{imports}"), 1)
        .replacen(before, after, 1)
        .replacen(
            end,
            "    .replace('\\r', \"\");\n    provider_input::view(name, &raw)\n}",
            1,
        );
    let current = std::fs::read_to_string(root().join(FILE))
        .unwrap()
        .replace("\r\n", "\n");
    let current = closure_inputs::project("census", FILE, current);
    let build = "#[path = \"support/bootstrap_build_guard.rs\"]\nmod build_guard;\n";
    assert_eq!(expected.matches(build).count(), 1);
    let expected = expected.replacen(build, &format!("{build}{runtime}"), 1);
    let signature_test = "\n#[test]\nfn census_standalone_panic_api_signatures_do_not_add_source_reads() {\n    let _: fn() -> String = provider_input::original;\n    let _: fn(&str, &str) -> bool = provider_input::accepts;\n    let _: fn(&str, &str) -> String = provider_input::view;\n}\n";
    assert_eq!(expected.matches(imports).count(), 1);
    assert_eq!(
        expected.replacen(imports, &format!("{imports}{signature_test}"), 1),
        current
    );
}

#[test]
fn census_provider_requires_complete_known_fixture_delta_and_pinned_helper() {
    let path = provider_input::contract::SOURCE;
    let raw = std::fs::read_to_string(root().join(path)).unwrap();
    let original = provider_input::original();
    assert!(provider_input::contract::accepts(&original, &raw));
    assert!(provider_input::accepts(&original, &raw));
    assert_eq!(
        provider_input::view("diagnostic_artifacts_linux.rs", &raw),
        original
    );
    assert!(original.contains("pub(crate) mod held_role_census;"));
    for bad in [
        String::new(),
        format!("{raw}\n// unknown edit"),
        raw.replacen("held_role_census", "foreign_census", 1),
        raw.replacen("libc::O_NOFOLLOW", "0", 1),
    ] {
        assert_ne!(bad, raw);
        assert!(!provider_input::accepts(&original, &bad));
        assert!(std::panic::catch_unwind(|| provider_input::view(
            "diagnostic_artifacts_linux.rs",
            &bad
        ))
        .is_err());
    }
    for name in [
        "diagnostic_held_unit_material.rs",
        "diagnostic_bundle_projection.rs",
        "unrelated.rs",
    ] {
        assert_eq!(provider_input::view(name, "unknown bytes"), "unknown bytes");
    }
    let helper = std::fs::read_to_string(root().join(provider_input::contract::CONTROL)).unwrap();
    assert!(provider_input::contract::helper_matches(&helper));
    assert!(!provider_input::contract::helper_matches(&format!(
        "{helper}\n// unknown helper"
    )));
}
