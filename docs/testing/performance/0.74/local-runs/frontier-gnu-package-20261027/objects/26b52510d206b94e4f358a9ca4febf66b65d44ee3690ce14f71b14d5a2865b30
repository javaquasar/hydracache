//! Exact uncached PID formatter and call substitutions, no validator exclusions.
#[path = "custody_cost_guard.rs"]
pub(crate) mod custody_cost;
#[allow(dead_code)]
#[path = "pid_stage_join.rs"]
pub(crate) mod stage_join;
pub(crate) const ORIGIN: &str = "tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs";
pub(crate) const ADMISSION: &str =
    "tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs";
pub(crate) const HOOKS: [(&str, &str); 2] = [
    (
        ORIGIN,
        "\n#[cfg(test)]\n#[path = \"../tests/support/pid_path.rs\"]\nmod pid_path;\n",
    ),
    (
        "tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs",
        "\n#[path = \"../tests/support/pid_path_alloc.rs\"]\nmod pid_path;\n",
    ),
];
pub(crate) const STACK: &str = r#"
// BEGIN STACK PID NAME
pub(super) struct PidName {
    bytes: [u8; 12],
    start: usize,
}
impl PidName {
    pub(super) fn new(pid: i32) -> Self {
        let mut bytes = [0; 12];
        let mut start = 11;
        let mut remaining = pid.unsigned_abs();
        loop {
            start -= 1;
            bytes[start] = b'0' + (remaining % 10) as u8;
            remaining /= 10;
            if remaining == 0 {
                break;
            }
        }
        if pid < 0 {
            start -= 1;
            bytes[start] = b'-';
        }
        Self { bytes, start }
    }
    pub(super) fn as_c_str(&self) -> &std::ffi::CStr {
        std::ffi::CStr::from_bytes_with_nul(&self.bytes[self.start..])
            .expect("signed i32 digits and one terminal NUL fit twelve bytes")
    }
}
// END STACK PID NAME
"#;
pub(crate) const CHANGES: [(&str, &str, &str); 5] = [
    (ORIGIN,"#[cfg(test)]\nuse std::ffi::CString;","use std::ffi::CString;"),
    (ORIGIN,"#[cfg(test)]\npub(super) fn child(parent: &File, name: &str, directory: bool) -> Result<File, KernelError> {","pub(super) fn child(parent: &File, name: &str, directory: bool) -> Result<File, KernelError> {"),
    (ORIGIN,"    pub(super) fn child(&self, pid: i32, directory: bool) -> Result<File, KernelError> {\n        fixed_child(&self.root, PidName::new(pid).as_c_str(), directory)\n    }\n","    pub(super) fn child(&self, name: &str, directory: bool) -> Result<File, KernelError> {\n        child(&self.root, name, directory)\n    }\n"),
    (ADMISSION,"let directory = origin.child(credentials.pid, true)?;","let directory = origin.child(&credentials.pid.to_string(), true)?;"),
    (ADMISSION,"let directory = origin.child(self.credentials.pid, true)?;","let directory = origin.child(&self.credentials.pid.to_string(), true)?;"),
];
#[allow(dead_code)]
pub(crate) fn normalize(path: &str, raw: &str) -> String {
    let raw = custody_cost::normalize(path, raw);
    finish(path, stage_join::project(path, raw))
}
// Fresh-status and its successors already enter the named earlier origin stage.
#[allow(dead_code)]
pub(crate) fn normalize_legacy(path: &str, raw: &str) -> String {
    finish(path, custody_cost::normalize(path, raw))
}
fn finish(path: &str, mut raw: String) -> String {
    for (file, exact, old) in HOOKS
        .into_iter()
        .map(|(f, s)| (f, s, ""))
        .chain([(ORIGIN, STACK, "")])
        .chain(CHANGES)
    {
        if path == file {
            assert_eq!(
                raw.matches(exact).count(),
                1,
                "exact PID path substitution: {exact}"
            );
            raw = raw.replacen(exact, old, 1);
        }
    }
    raw
}
