const BASE: &str = "72efce2401e753b857f7fc03f3300b6aae19bd5d";
#[path = "support/document_scratch_guard.rs"]
mod guard;
fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "")
}
fn git(path: &str) -> String {
    let out = std::process::Command::new("git")
        .args(["show", &format!("{BASE}:{path}")])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap().replace('\r', "")
}
fn body<'a>(text: &'a str, signature: &str, end: &str) -> &'a str {
    let start = text.find(signature).unwrap();
    let start = start + text[start..].find("    #[cfg(test)]").unwrap();
    &text[start..text[start..].find(end).unwrap() + start]
}
#[test]
fn document_scratch_reference_and_prototype_preserve_complete_sender_and_pidfd_checks() {
    let old = git("tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs");
    let source = read("tools/long-run-supervisor-074/tests/support/document_scratch.rs");
    let old_sender = body(&old, "    fn revalidate(&self, origin:", "\n}\nfn alive");
    let reference = body(
        &source,
        "fn scratch_reference(",
        "\n    pub(in crate::diagnostic_kernel_admission) fn scratch_prototype",
    );
    assert_eq!(reference.trim_end(), old_sender.trim_end());
    let prototype = body(
        &source,
        "fn scratch_prototype(",
        "\n}\npub(in crate::diagnostic_kernel_admission) fn current",
    );
    let mut reversed = prototype.replace("        let mut scratch = DocumentBuffer::new();\n", "");
    assert_eq!(
        prototype
            .matches("        let mut scratch = DocumentBuffer::new();\n")
            .count(),
        1
    );
    assert_eq!(
        prototype
            .matches("alive_read(&self.pidfd, self.credentials.pid, |file| scratch.read(file))?;")
            .count(),
        2
    );
    reversed = reversed.replace(
        "alive_read(&self.pidfd, self.credentials.pid, |file| scratch.read(file))?;",
        "alive(&self.pidfd, self.credentials.pid)?;",
    );
    assert_eq!(
        prototype
            .matches("projection(scratch.read(file)?, &self.credentials)?")
            .count(),
        1
    );
    reversed = reversed.replace(
        "projection(scratch.read(file)?, &self.credentials)?",
        "projection(&bounded(file)?, &self.credentials)?",
    );
    assert_eq!(reversed.trim_end(), old_sender.trim_end());
    let helper = read("tools/long-run-supervisor-074/tests/support/document_scratch_impl.rs");
    let original = body(&old, "fn alive(", "\n#[cfg(test)]");
    let buffered = body(
        &helper,
        "pub(super) fn alive_read",
        "\n#[cfg(test)]\nmod tests",
    );
    let reversed = buffered
        .replace("let raw = read(&info)?;", "let raw = bounded(&info)?;")
        .replace(
            "std::str::from_utf8(raw.as_ref())",
            "std::str::from_utf8(&raw)",
        );
    assert_eq!(reversed.trim_end(), original.trim_end());
}

#[test]
fn document_scratch_adoption_is_the_frozen_helper_and_reconstructs_all_473_old_bodies() {
    const FROZEN: &str = "4952b0f28fd752d9e479ed5f5ed34e6184bbb836";
    let old = |path: &str| {
        let out = std::process::Command::new("git")
            .args(["show", &format!("{FROZEN}:{path}")])
            .current_dir(root())
            .output()
            .unwrap();
        assert!(out.status.success());
        String::from_utf8(out.stdout).unwrap().replace('\r', "")
    };
    let origin = read(guard::ORIGIN);
    assert_eq!(origin.matches(guard::BLOCK).count(), 1);
    let inline = guard::BLOCK
        .strip_prefix("// BEGIN INVOCATION DOCUMENT SCRATCH\n")
        .unwrap()
        .strip_suffix("// END INVOCATION DOCUMENT SCRATCH\n\n")
        .unwrap()
        .replace("mod document_scratch_tests {", "mod tests {");
    let frozen = old("tools/long-run-supervisor-074/tests/support/document_scratch_impl.rs");
    assert_eq!(format!("//! One invocation's temporary bytes; never a retained observation or FD.\nuse super::*;\n{inline}"),frozen);
    let out = std::process::Command::new("git")
        .args([
            "ls-tree",
            "-r",
            "--name-only",
            FROZEN,
            "--",
            "tools/long-run-supervisor-074/src",
            "tools/long-run-supervisor-074/tests/support",
        ])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(out.status.success());
    let paths = String::from_utf8(out.stdout).unwrap();
    assert_eq!(paths.lines().count(), 473);
    for path in paths.lines() {
        assert_eq!(guard::normalize(path, &read(path)), old(path), "{path}");
    }
    let raw = read(guard::SOURCE);
    assert_eq!(raw.matches(guard::HOOK).count(), 1);
    for exact in [
        "        let mut scratch = DocumentBuffer::new();\n",
        guard::WRAPPER,
        "alive_read(&self.pidfd, self.credentials.pid, |file| scratch.read(file))?;",
        "projection(scratch.read(file)?, &self.credentials)?",
    ] {
        for changed in [raw.replacen(exact, "", 1), format!("{raw}{exact}")] {
            assert!(
                std::panic::catch_unwind(|| guard::normalize(guard::SOURCE, &changed)).is_err()
            );
        }
    }
    let changed = raw.replacen("origin.revalidate()?;", "// removed fresh observation", 1);
    assert_ne!(changed, raw);
    assert_ne!(
        guard::normalize(guard::SOURCE, &changed),
        old(guard::SOURCE)
    );
}

#[test]
fn legacy_source_views_preserve_every_old_assertion_and_only_restore_known_scratch_shapes() {
    for (path, view) in [
        (
            "crates/xtask/tests/support/performance_contract_074_kernel_walk.rs",
            "scratch_view",
        ),
        (
            "crates/xtask/tests/support/performance_contract_074_boot_comparison.rs",
            "scratch_view",
        ),
        (
            "crates/xtask/tests/fresh_kernel_cost_074.rs",
            "guard::pid_path::custody_cost::document_scratch",
        ),
        (
            "crates/xtask/tests/pid_path_cost_074.rs",
            "guard::custody_cost::document_scratch",
        ),
        (
            "crates/xtask/tests/held_preparation_cost_074.rs",
            "prefix::fresh_status::pid_path::custody_cost::document_scratch",
        ),
    ] {
        let out = std::process::Command::new("git")
            .args([
                "show",
                &format!("1f3715498cb3b9b1328061a3647f90c1c3bd2e19:{path}"),
            ])
            .current_dir(root())
            .output()
            .unwrap();
        assert!(out.status.success());
        let old = String::from_utf8(out.stdout).unwrap().replace('\r', "");
        let start = old.find("fn read(path: &str) -> String {").unwrap();
        let end = start + old[start..].find("\n}\n").unwrap() + 2;
        let original = &old[start..end];
        let expected = original
            .replacen(
                "    std::fs::read_to_string(",
                "    let raw = std::fs::read_to_string(",
                1,
            )
            .replacen(
                ".replace('\\r', \"\")\n}",
                &format!(".replace('\\r', \"\");\n    {view}::normalize(path, &raw)\n}}"),
                1,
            );
        let mut current = read(path);
        if view == "scratch_view" {
            let hook = "\n#[path = \"document_scratch_guard.rs\"]\nmod scratch_view;\n";
            assert_eq!(current.matches(hook).count(), 1);
            current = current.replacen(hook, "\n", 1);
        }
        assert_eq!(current.matches(&expected).count(), 1);
        assert_eq!(
            current.replacen(&expected, original, 1),
            old,
            "no old assertion may be removed: {path}"
        );
    }
}
