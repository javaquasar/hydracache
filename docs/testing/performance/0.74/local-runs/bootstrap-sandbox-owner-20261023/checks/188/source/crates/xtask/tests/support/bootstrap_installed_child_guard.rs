#![allow(dead_code)] // Exact additive views for preceding contracts.
#[path = "bootstrap_sandbox_guard.rs"]
pub mod sandbox;
pub const BLOCK: &str = r#"
<!-- BEGIN BOOTSTRAP INSTALLED CHILD V1 -->
## Installed child executable consistency v1

--systemd-bootstrap-installed-v1 is a separate opt-in refusal-only mode.
Activation requires exactly current canonical LISTEN_PID, LISTEN_FDS=14 and the
old twelve names followed by installed-image:installed-entry. Child FD0..14 keep
their meanings; FD15 is an asserted installed image and FD16 sealed HCBI0741.
These seventeen child slots are not the parent's 33 physical descriptors.
Unknown metadata/arguments refuse; old modes and record bytes remain unchanged.

A separate bounded procfs checker requires exactly FD0..16 and its own transient
enumeration FD (20 entries, at most 21 reads). The unchanged fifteen-role capture
runs all old predicates in order. Sealed FD16 must have readonly access, zero
offset, four exact seals and stable metadata; its embedded old entry bytes must
equal FD14. FD15 is readonly/non-O_PATH, zero-offset, single-link regular, mode
0555/0755, bounded by 128 MiB and tied to asserted device/inode/size/full hash.
Old sealed-reference ELF/hash checks remain; regular-role aliases refuse.

Fixed procfs thread-self/exe inspection must identify the same complete metadata
as FD15, not sealed FD13; inspection FDs close before the next closed census.
The entire observation repeats and must match. These sequential checks do not
authenticate root ancestry, UID/GID namespace, policy/build, sender, manager,
guardians or original custody. The parent fixed root-owned source policy is
unchanged; this leaf does not independently enroll it. A matching forged owned
filesystem executable still exits 2 with original authentication closed.

No parent FD transport/remap, start/send/exec/release, installation, migration,
default fallback, writer cessation, completion or server admission is enabled.
Old absolute windows and all old mode bodies are preserved. Older binaries
reject the new argument. Synthetic controls do not prove original delivery.
<!-- END BOOTSTRAP INSTALLED CHILD V1 -->
"#;
pub const MODE_HOOK: &str =
    "\n#[path = \"../diagnostic_bootstrap_installed_mode.rs\"]\nmod installed_mode;\n";
pub const DISPATCH: &str = "    if installed_mode::requested() {\n        if let Err(error) = installed_mode::run() {\n            eprintln!(\"Bootstrap refused: {error}; no start, send or executable handoff.\");\n        }\n        std::process::exit(2);\n    }\n";
pub fn hook(name: &str) -> Option<&'static str> {
    match name {
        "bin/diagnostic_held_bootstrap.rs" => Some(MODE_HOOK),
        "diagnostic_bootstrap_child_roles.rs" => Some("\n#[path = \"diagnostic_bootstrap_installed_child.rs\"]\npub(in super::super) mod installed;\n"),
        "bootstrap_current_image.rs" => Some("#[path = \"support/bootstrap_installed_controls.rs\"]\nmod installed_controls;\n"),
        _ => None,
    }
}
fn remove(raw: &str, addition: &str) -> String {
    assert_eq!(raw.matches(addition).count(), 1);
    raw.replacen(addition, "", 1)
}
pub fn source_view(name: &str, raw: &str) -> String {
    let raw = sandbox::source_view(name, raw);
    let raw = raw.as_str();
    let value = match hook(name) {
        Some(hook) => remove(raw, hook),
        None => raw.into(),
    };
    if name == "bin/diagnostic_held_bootstrap.rs" {
        remove(&value, DISPATCH)
    } else {
        value
    }
}
pub fn compat_view(raw: &str) -> String {
    remove(raw, BLOCK)
}
