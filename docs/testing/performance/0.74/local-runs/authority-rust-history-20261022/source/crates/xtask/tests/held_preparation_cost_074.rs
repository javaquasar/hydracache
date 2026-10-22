//! Historical bytes reconstructed only through exact observer/prefix changes.
#[path = "support/authority_history_io.rs"]
mod history;
#[path = "support/projection_prefix_guard.rs"]
mod prefix;
const BASE: &str = "b41a7fb652f0687fbe5bad3222b5fa82f5e09443";
const FIXTURE: &str = "tools/long-run-supervisor-074/tests/support/consuming_held_live.rs";
fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &str) -> String {
    let raw = history::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "");
    prefix::fresh_status::pid_path::custody_cost::document_scratch::normalize(path, &raw)
}
fn git(args: &[&str]) -> String {
    let out = history::command()
        .args(args)
        .current_dir(root())
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap().replace('\r', "")
}
fn restore(raw: &mut String, new: &str, old: &str) {
    assert_eq!(raw.matches(new).count(), 1, "exact observer change: {new}");
    *raw = raw.replacen(new, old, 1);
}
fn fixture(mut raw: String) -> String {
    restore(
        &mut raw,
        "\n#[path = \"consuming_held_cost.rs\"]\nmod cost;\n",
        "",
    );
    restore(
        &mut raw,
        "use sha2::{Digest, Sha256};\n",
        "use sha2::{Digest, Sha256};\nuse std::time::Instant;\n",
    );
    let old = git(&["show", &format!("{BASE}:{FIXTURE}")]);
    let start = old.find("fn profiled<T>").unwrap();
    let end = old[start..].find("\n#[test]").unwrap() + start;
    restore(&mut raw,"fn profiled<T>(phase: &str, call: impl FnOnce() -> T) -> T {\n    cost::profiled(phase, call)\n}\n",&old[start..end]);
    for (new,old) in [
        ("let prepared = measured(\"prepare-consent\", || {\n                causal.prepare_framed_consent(\"1\".repeat(64)).unwrap()\n            });","let prepared = causal.prepare_framed_consent(\"1\".repeat(64)).unwrap();"),
        ("let accepted = measured(\"accept-consent\", || {\n                prepared.accept_consent(relay.receive(&bytes)).unwrap()\n            });","let accepted = prepared.accept_consent(relay.receive(&bytes)).unwrap();"),
        ("let outputs = measured(\"outputs\", || accepted.prepare_original_outputs().unwrap());","let outputs = accepted.prepare_original_outputs().unwrap();"),
        ("let frame = measured(\"enrollment-frame\", || {\n                outputs\n                    .prepare_privileged_enrollment_for_test(&bundle.path().join(\"bundle\"))\n                    .unwrap()\n            });","let frame = outputs\n                .prepare_privileged_enrollment_for_test(&bundle.path().join(\"bundle\"))\n                .unwrap();"),
        ("let message = measured(\"fixed-manager-message\", || {\n                frame.prepare_fixed_manager_message().unwrap()\n            });","let message = frame.prepare_fixed_manager_message().unwrap();"),
        ("let mut owned = measured(\"owned-root-preparation\", || {\n                message.prepare_owned_root_guardians(&mut root).unwrap()\n            });","let mut owned = message.prepare_owned_root_guardians(&mut root).unwrap();"),
        ("measured(\"enrollment\", || owned.enroll_once().unwrap());","owned.enroll_once().unwrap();"),
    ] {restore(&mut raw,new,old)}
    raw
}
#[test]
fn held_preparation_and_prefix_reconstruct_all_old_runtime_and_fixture_assertions() {
    let paths = git(&[
        "ls-tree",
        "-r",
        "--name-only",
        BASE,
        "--",
        "tools/long-run-supervisor-074/src",
        "tools/long-run-supervisor-074/tests/support",
    ]);
    assert_eq!(paths.lines().count(), 462);
    for path in paths.lines().chain([
        "Cargo.lock",
        "docs/COMPAT.md",
        "docs/testing/gated-test-registry.toml",
        "docs/testing/performance/0.74/qualification-manifest.toml",
    ]) {
        let current = prefix::normalize(path, &read(path));
        let current = if path == FIXTURE {
            fixture(current)
        } else {
            current
        };
        assert_eq!(current, git(&["show", &format!("{BASE}:{path}")]), "{path}");
    }
}
#[test]
fn held_preparation_observer_adapter_cannot_hide_missing_duplicate_or_changed_assertion() {
    let raw = read(FIXTURE);
    for token in [
        "cost::profiled(phase, call)",
        "measured(\"enrollment\", || owned.enroll_once().unwrap());",
    ] {
        for bad in [
            raw.replacen(token, "", 1),
            raw.replacen(token, &format!("{token}{token}"), 1),
        ] {
            assert!(std::panic::catch_unwind(|| fixture(bad)).is_err());
        }
    }
    let old = git(&["show", &format!("{BASE}:{FIXTURE}")]);
    let drift = raw.replace(
        "[300000000000, 60000000000, 10000000000]",
        "[300000000000, 60000000000, 11000000000]",
    );
    assert_ne!(drift, raw);
    assert_ne!(fixture(drift), old);
    for (path, hook) in prefix::HOOKS {
        let raw = read(path);
        for bad in [raw.replacen(hook, "", 1), format!("{raw}{hook}")] {
            assert!(std::panic::catch_unwind(|| prefix::normalize(path, &bad)).is_err());
        }
    }
}
#[test]
fn projection_reference_is_the_complete_frozen_decoder_not_a_relaxed_model() {
    let old = git(&[
        "show",
        &format!("{BASE}:tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs"),
    ]);
    let old = &old[old.find("fn projection(").unwrap()..old.find("\nstruct Sender").unwrap()];
    let old = &old[old.find("\n    #[cfg(test)]").unwrap()..];
    let raw = read("tools/long-run-supervisor-074/tests/support/projection_prefix.rs");
    assert!(raw.contains("pub(in crate::diagnostic_kernel_admission) fn reference("));
    let raw = &raw[raw.find("\n    #[cfg(test)]").unwrap()..raw.find("\n#[test]").unwrap()];
    assert_eq!(raw.trim_end(), old.trim_end());
}

#[test]
fn projection_prefix_adapter_refuses_missing_duplicate_and_unrelated_guard_drift() {
    let raw = read(prefix::SOURCE);
    for (exact, _) in prefix::SUBSTITUTIONS {
        for bad in [raw.replacen(exact, "", 1), format!("{raw}{exact}")] {
            assert!(std::panic::catch_unwind(|| prefix::normalize(prefix::SOURCE, &bad)).is_err());
        }
    }
    let old = git(&["show", &format!("{BASE}:{}", prefix::SOURCE)]);
    for token in [
        ".take(33)",
        "origin.revalidate()?;",
        "alive(&self.pidfd, self.credentials.pid)?;",
    ] {
        let drift = raw.replacen(token, "/* missing old validation */", 1);
        assert_ne!(drift, raw);
        assert_ne!(prefix::normalize(prefix::SOURCE, &drift), old);
    }
}
