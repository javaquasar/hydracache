//! Source-only origin join after the original document/custody views.
//! No whole-body fallback: unrelated bytes survive to the original equality guard.
pub(crate) const CURRENT: &str = "46648903c0eb7ce88d438008fa6465a96cc3c414";
pub(crate) const PREVIOUS: &str = "b5f1b8fce6b3046239087873cb85ff1bfc557f0f";
pub(crate) const STAGE_SHA256: &str =
    "4eb7a1463e4d2318ecdfee44759aafb632c950da4a9309f39d26004176a59c8a";
pub(crate) const PREVIOUS_SHA256: &str =
    "05cfe2ccbad64af3aaa4c38f65535ff3039e5051c0812e99502b79e2933c8bcc";
pub(crate) const CHANGES: &[(&str, &str, usize)] = &[
    ("use std::os::fd::FromRawFd;\n\n#[path = \"diagnostic_kernel_fd_identity.rs\"]\nmod fd_identity;\n\n#[cfg(test)]\n#[path = \"diagnostic_kernel_operation_cost.rs\"]\nmod operation_cost;\n\n#[cfg(all(test, target_arch = \"x86_64\"))]\n#[path = \"diagnostic_kernel_operation_tests.rs\"]\nmod operation_tests;\n\n#[cfg(all(test, target_arch = \"x86_64\", target_env = \"gnu\"))]\n#[path = \"diagnostic_namespace_open.rs\"]\nmod fixed_namespace_open;\n\n#[cfg(all(test, target_arch = \"x86_64\", target_env = \"gnu\"))]\n#[path = \"diagnostic_namespace_open_tests.rs\"]\nmod namespace_open_tests;\n\n#[cfg(all(test, target_arch = \"x86_64\", target_env = \"gnu\"))]\n#[path = \"diagnostic_filesystem_projection.rs\"]\nmod filesystem_projection;\n\n#[cfg(all(test, target_arch = \"x86_64\", target_env = \"gnu\"))]\n#[path = \"diagnostic_filesystem_projection_tests.rs\"]\nmod filesystem_projection_tests;\n\n", "use std::os::fd::FromRawFd;\n\n", 1),
    ("        .open(namespace_path(name).as_ref())?;\n    #[cfg(test)]\n    drop(operation_open);\n    filesystem(&file, 0x6e736673)?;\n    #[cfg(test)]\n    let operation_identity = operation_cost::span(operation_cost::Kind::Identity);\n    let id = fd_identity::fresh(&file)?;\n    #[cfg(test)]\n    drop(operation_identity);\n    Ok((file, id))\n}\n", "        .open(namespace_path(name).as_ref())?;\n    filesystem(&file, 0x6e736673)?;\n    let m = file.metadata()?;\n    Ok((file, (m.dev(), m.ino())))\n}\n", 1),
    ("        | if directory { libc::O_DIRECTORY } else { 0 };\n    #[cfg(test)]\n    let operation_open = operation_cost::span(operation_cost::Kind::Open);\n    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };\n    #[cfg(test)]\n    drop(operation_open);\n    if fd < 0 {\n", "        | if directory { libc::O_DIRECTORY } else { 0 };\n    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };\n    if fd < 0 {\n", 1),
    ("pub(super) fn filesystem(file: &File, magic: libc::c_long) -> Result<(), KernelError> {\n    #[cfg(test)]\n    let _operation_fs = operation_cost::span(operation_cost::Kind::Filesystem);\n    let mut stat: libc::statfs = unsafe { std::mem::zeroed() };\n", "pub(super) fn filesystem(file: &File, magic: libc::c_long) -> Result<(), KernelError> {\n    let mut stat: libc::statfs = unsafe { std::mem::zeroed() };\n", 1),
    ("            filesystem(file, 0x9fa0)?;\n            if fd_identity::fresh(file)? != self.id || !boot_comparison::matches(file, &self.boot)?\n            {\n                return Err(kernel_invalid!(\"proc-origin-or-boot\"));\n", "            filesystem(file, 0x9fa0)?;\n            let m = file.metadata()?;\n            if (m.dev(), m.ino()) != self.id || !boot_comparison::matches(file, &self.boot)? {\n                return Err(kernel_invalid!(\"proc-origin-or-boot\"));\n", 1),
    ("    // These fixed kernel magic links must be followed, then checked as nsfs.\n    #[cfg(test)]\n    let operation_open = operation_cost::span(operation_cost::Kind::Open);\n    let file = OpenOptions::new()\n", "    // These fixed kernel magic links must be followed, then checked as nsfs.\n    let file = OpenOptions::new()\n", 1),
    ("    #[cfg(test)]\n    let _operation_document = operation_cost::span(operation_cost::Kind::Document);\n    #[cfg(test)]\n    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(\n", "    #[cfg(test)]\n    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(\n", 1),
    ("            filesystem(file, 0x6e736673)?;\n            if fd_identity::fresh(file)? != *id || namespace(NAMES[index])?.1 != *id {\n                return Err(kernel_invalid!(\"namespace-origin\"));\n", "            filesystem(file, 0x6e736673)?;\n            let m = file.metadata()?;\n            if (m.dev(), m.ino()) != *id || namespace(NAMES[index])?.1 != *id {\n                return Err(kernel_invalid!(\"namespace-origin\"));\n", 1),
    ("fn boot_view(raw: &([u8; 38], usize)) -> Result<&str, KernelError> {\n    #[cfg(test)]\n    let _operation_decode = operation_cost::span(operation_cost::Kind::Decode);\n    let (raw, count) = raw;\n", "fn boot_view(raw: &([u8; 38], usize)) -> Result<&str, KernelError> {\n    let (raw, count) = raw;\n", 1),
    ("fn namespace_path(name: &str) -> std::borrow::Cow<'static, str> {\n    #[cfg(test)]\n    let _operation_path = operation_cost::span(operation_cost::Kind::Path);\n    use std::borrow::Cow;\n", "fn namespace_path(name: &str) -> std::borrow::Cow<'static, str> {\n    use std::borrow::Cow;\n", 1),
    (") -> Result<T, KernelError> {\n    #[cfg(test)]\n    drop(operation_cost::span(operation_cost::Kind::Empty));\n    #[cfg(test)]\n", ") -> Result<T, KernelError> {\n    #[cfg(test)]\n", 1),
    ("        filesystem(&root, 0x9fa0)?;\n        let id = fd_identity::fresh(&root)?;\n        let boot = boot(&root)?;\n", "        filesystem(&root, 0x9fa0)?;\n        let m = root.metadata()?;\n        let boot = boot(&root)?;\n", 1),
    ("    }\n    if fd_identity::fresh(file)? != expected {\n        return Err(kernel_invalid!(\"namespace-object\"));\n", "    }\n    let metadata = file.metadata()?;\n    if (metadata.dev(), metadata.ino()) != expected {\n        return Err(kernel_invalid!(\"namespace-object\"));\n", 1),
    ("            root,\n            id,\n            boot,\n", "            root,\n            id: (m.dev(), m.ino()),\n            boot,\n", 1),
    ("    #[cfg(test)]\n    drop(operation_cost::span(operation_cost::Kind::Empty));\n    #[cfg(test)]\n    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(\n", "    #[cfg(test)]\n    let _kernel_walk = crate::diagnostic_ipc::validation_trace::kernel_walk::span(\n", 1),
];
pub(crate) fn project(path: &str, source: String) -> String {
    if path != super::ORIGIN {
        return source;
    }
    let mut raw = source;
    for &(current, previous, count) in CHANGES {
        assert_eq!(
            raw.matches(current).count(),
            count,
            "exact PID origin stage join"
        );
        raw = raw.replace(current, previous);
    }
    raw
}
