//! Exact isolated parser edit. No skipped predicate or permissive historical view.
use sha2::{Digest, Sha256};
pub const SOURCE: &str = "tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs";
pub const HELPER: &str = "tools/long-run-supervisor-074/src/diagnostic_projection_fields.rs";
pub const LEGACY_TEST: &str = "tools/long-run-supervisor-074/tests/support/projection_cost.rs";
pub const HELPER_SHA256: &str = "0bd6b4246b7d14151f378fda0090523fdc3485be254cc05df33eaa97264cd6ae";
pub fn helper_is_pinned(raw: &str) -> bool {
    let raw = raw.replace("\r\n", "\n");
    let digest = Sha256::digest(raw.as_bytes());
    let hex = b"0123456789abcdef";
    digest.iter().enumerate().all(|(index, byte)| {
        HELPER_SHA256.as_bytes()[index * 2] == hex[usize::from(byte >> 4)]
            && HELPER_SHA256.as_bytes()[index * 2 + 1] == hex[usize::from(byte & 15)]
    })
}
fn edit(raw: &str, before: &str, after: &str) -> String {
    assert_eq!(
        raw.matches(before).count(),
        1,
        "ambiguous status field edit"
    );
    raw.replacen(before, after, 1)
}
pub fn legacy_test(baseline: &str) -> String {
    edit(baseline,
        "    let runtime = include_str!(\"../../src/diagnostic_kernel_admission.rs\").replace('\\r', \"\");",
        "    let runtime = include_str!(\"../../src/diagnostic_kernel_admission.rs\").replace('\\r', \"\");\n    let runtime = crate::diagnostic_kernel_admission::projection_fields_tests::legacy_source(&runtime);")
}
pub fn candidate(baseline: &str) -> String {
    let mut result = edit(baseline,
        "#[path = \"diagnostic_projection_groups.rs\"]\nmod projection_groups;",
        "#[path = \"diagnostic_projection_fields.rs\"]\nmod projection_fields;\n#[path = \"diagnostic_projection_groups.rs\"]\nmod projection_groups;");
    result = edit(&result,
        "    let groups_field = |prefix: &str| -> Result<Vec<u32>, KernelError> {\n        let mut lines = text.lines().filter_map(|line| line.strip_prefix(prefix));\n        let body = kernel_required!(lines.next(), \"projection-missing-field\")?;\n        if lines.next().is_some() {",
        "    let fields = projection_fields::Fields::new(text);\n    let groups_field = |name| -> Result<Vec<u32>, KernelError> {\n        let (body, duplicate) = fields.get(name);\n        let body = kernel_required!(body, \"projection-missing-field\")?;\n        if duplicate {");
    result = edit(&result,
        "    let scalar = |prefix: &str| -> Result<([u32; 33], usize), KernelError> {\n        let mut lines = text.lines().filter_map(|line| line.strip_prefix(prefix));\n        let body = kernel_required!(lines.next(), \"projection-missing-field\")?;\n        if lines.next().is_some() {",
        "    let scalar = |name| -> Result<([u32; 33], usize), KernelError> {\n        let (body, duplicate) = fields.get(name);\n        let body = kernel_required!(body, \"projection-missing-field\")?;\n        if duplicate {");
    for name in ["Pid", "Tgid", "Uid", "Gid"] {
        result = edit(
            &result,
            &format!("scalar(\"{name}:\")?"),
            &format!("scalar(projection_fields::Name::{name})?"),
        );
    }
    result = edit(
        &result,
        "groups_field(\"Groups:\")?",
        "groups_field(projection_fields::Name::Groups)?",
    );
    edit(&result, "#[path = \"diagnostic_kernel_admission_tests.rs\"]\nmod tests;\n",
        "#[path = \"diagnostic_kernel_admission_tests.rs\"]\nmod tests;\n\n#[cfg(test)]\n#[path = \"diagnostic_projection_fields_tests.rs\"]\nmod projection_fields_tests;\n")
}
