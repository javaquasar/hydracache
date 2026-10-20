const BASE: &str = "72efce2401e753b857f7fc03f3300b6aae19bd5d";
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
