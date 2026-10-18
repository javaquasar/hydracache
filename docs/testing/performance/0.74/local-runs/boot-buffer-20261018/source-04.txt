//! One exact experimental substitution; never normalize other live checks.
const BASE: &str = "3b8e73feec8412e4090bf05575f06a05901e1010";
const PATH: &str = "tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs";
const OLD: &str = r###"fn boot(root: &File) -> Result<String, KernelError> {
    #[cfg(test)]
    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Boot,
    );
    let sys = child(root, "sys", true)?;
    let kernel = child(&sys, "kernel", true)?;
    let random = child(&kernel, "random", true)?;
    let raw = bounded(&child(&random, "boot_id", false)?)?;
    if raw.len() != 37
        || raw[36] != b'\n'
        || raw[..36].iter().enumerate().any(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                *c != b'-'
            } else {
                !c.is_ascii_digit() && !(b'a'..=b'f').contains(c)
            }
        })
    {
        return Err(kernel_invalid!("boot-document"));
    }
    String::from_utf8(raw[..36].to_vec()).map_err(|_| kernel_invalid!("boot-utf8"))
}
"###;
const NEW: &str = r###"#[path = "diagnostic_boot_document.rs"]
mod boot_document;
fn boot(root: &File) -> Result<String, KernelError> {
    #[cfg(test)]
    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Boot,
    );
    let sys = child(root, "sys", true)?;
    let kernel = child(&sys, "kernel", true)?;
    let random = child(&kernel, "random", true)?;
    let file = child(&random, "boot_id", false)?;
    let raw = boot_read(|bytes| file.read_at(bytes, 0))?;
    // As before, close the proc document before decoding the owned bytes.
    drop(file);
    boot_value(raw)
}
fn boot_read(
    read: impl FnOnce(&mut [u8]) -> std::io::Result<usize>,
) -> Result<([u8; 38], usize), KernelError> {
    #[cfg(test)]
    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(
        crate::diagnostic_ipc::validation_trace::kernel_walk::Kind::Document,
    );
    Ok(boot_document::read_once(read)?)
}
fn boot_value((raw, count): ([u8; 38], usize)) -> Result<String, KernelError> {
    if count == 0 {
        return Err(kernel_invalid!("proc-document-budget"));
    }
    if count != 37
        || raw[36] != b'\n'
        || raw[..36].iter().enumerate().any(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                *c != b'-'
            } else {
                !c.is_ascii_digit() && !(b'a'..=b'f').contains(c)
            }
        })
    {
        return Err(kernel_invalid!("boot-document"));
    }
    String::from_utf8(raw[..36].to_vec()).map_err(|_| kernel_invalid!("boot-utf8"))
}
"###;
const HELPER: &str = r###"//! One-read temporary storage only; no origin, clock, or admission state.
pub(super) fn read_once(
    read: impl FnOnce(&mut [u8]) -> std::io::Result<usize>,
) -> std::io::Result<([u8; 38], usize)> {
    // Canonical UUID plus newline and one byte detecting an overlength read.
    // No retry: a short read or EINTR must remain visible to the caller.
    let mut bytes = [0; 38];
    let count = read(&mut bytes)?;
    Ok((bytes, count))
}
"###;
const TEST: &str =
    "#[cfg(test)]\n#[path = \"diagnostic_boot_buffer_tests.rs\"]\nmod boot_buffer_tests;\n\n";
pub(super) fn normalize(path: &str, raw: &str) -> String {
    if path != PATH {
        return raw.to_owned();
    }
    assert_eq!(
        raw.matches(NEW).count(),
        1,
        "boot candidate implementation drift"
    );
    let clean = raw.replacen(NEW, OLD, 1);
    assert_eq!(
        clean.matches(TEST).count(),
        1,
        "boot candidate test registration drift"
    );
    clean.replacen(TEST, "", 1)
}
fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path))
        .unwrap()
        .replace('\r', "")
}
fn old(path: &str) -> String {
    let out = std::process::Command::new("git")
        .args(["show", &format!("{BASE}:{path}")])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap().replace('\r', "")
}
#[test]
fn boot_buffer_exact_substitution_preserves_all_other_old_runtime_sources() {
    let out = std::process::Command::new("git")
        .args([
            "ls-tree",
            "-r",
            "--name-only",
            BASE,
            "--",
            "tools/long-run-supervisor-074/src",
        ])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(out.status.success());
    let raw = String::from_utf8(out.stdout).unwrap();
    let paths: Vec<_> = raw.lines().filter(|p| p.ends_with(".rs")).collect();
    assert_eq!(paths.len(), 353);
    for path in paths {
        assert_eq!(normalize(path, &read(path)), old(path), "{path}");
    }
    assert_eq!(
        read("tools/long-run-supervisor-074/src/diagnostic_boot_document.rs"),
        HELPER
    );
    assert!(HELPER.contains("impl FnOnce(&mut [u8])"));
    assert!(HELPER.contains("let count = read(&mut bytes)?;"));
    let bounded = old("tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs");
    assert!(bounded.contains("const DOC_BYTES: usize = 65_536;"));
    assert!(bounded.contains("let mut bytes = vec![0; DOC_BYTES + 1];"));
    assert!(bounded.contains("let count = file.read_at(&mut bytes, 0)?;"));
    assert!(bounded.contains("bytes.truncate(count);"));
}
#[test]
fn boot_buffer_adapter_cannot_hide_changed_read_guard_or_schedule() {
    let raw = read(PATH);
    for token in [
        "boot_document::read_once(read)?",
        "file.read_at(bytes, 0)",
        "if count != 37",
        "if count == 0",
        "filesystem(file, 0x9fa0)?;",
        "self.revalidate()?;",
    ] {
        assert!(raw.contains(token));
        let changed = raw.replacen(token, &format!("{token} /* drift */"), 1);
        if NEW.contains(token) {
            assert!(std::panic::catch_unwind(|| normalize(PATH, &changed)).is_err());
        } else {
            assert_ne!(normalize(PATH, &changed), old(PATH));
        }
    }
}
