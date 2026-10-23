#![allow(dead_code)] // Exact additive views for distinct historical source guards.
#[path = "bootstrap_installed_entry_guard.rs"]
mod installed_entry;
pub const BLOCK: &str = r#"
<!-- BEGIN BOOTSTRAP INSTALLED ORIGIN V1 -->
## Parent retained installed bootstrap origin v1

The opt-in parent wrapper consumes the original BootstrapOwnedUnit and retains
/opt/hydracache-perf/bin/diagnostic-held-bootstrap-074 independently from its
unchanged sealed reference. Only the privileged parent opens that fixed path.
Root UID/GID ancestry must allow owner/other search without special/group/other-write
bits; the executable must be a single-link root-owned regular file, mode 0555
or 0755. All retained opens are readonly/CLOEXEC/nofollow/nonblocking, with
positional bounded ELF/hash reads against the same verified bootstrap receipt.
Name, inode, metadata, flags, zero offset and ancestry drift latch refusal.

Parent-local census extends the old 27-FD/15-role prefix to 32 FDs/16 roles:
four ancestry FDs and the distinct installed image; no descriptor or owner export.
Three old complete bootstrap frames bracket two installed inspections. Failure
retires the same owner before added descriptors close; missing retained source
cannot be recreated. Old absolute origins/windows and child modes/record bytes
stay unchanged. This fixed-origin contract is not an enabled child wire v2,
ExecStart, installation, child namespace/credential enrollment, release, writer
cessation, completion or server admission. Older binaries do not inspect this
additional origin; no migration, default activation or weaker fallback exists.
<!-- END BOOTSTRAP INSTALLED ORIGIN V1 -->
"#;
fn remove(raw: &str, addition: &str) -> String {
    assert_eq!(raw.matches(addition).count(), 1);
    raw.replacen(addition, "", 1)
}
pub fn compat_view(raw: &str) -> String {
    remove(&installed_entry::compat_view(raw), BLOCK)
}
pub fn source_view(name: &str, raw: &str) -> String {
    let raw = installed_entry::source_view(name, raw);
    let raw = raw.as_str();
    let hook = match name {
        "diagnostic_bootstrap_build.rs" => "\n#[cfg(target_os = \"linux\")]\n#[path = \"diagnostic_bootstrap_installed_source.rs\"]\npub(crate) mod installed;\n",
        "diagnostic_bootstrap_owner.rs" => "\n#[path = \"diagnostic_bootstrap_installed_owner.rs\"]\nmod installed_origin;\n",
        "diagnostic_bootstrap_entry_census.rs" => "\n#[path = \"diagnostic_bootstrap_installed_census.rs\"]\npub(crate) mod installed;\n",
        _ => return raw.into(),
    };
    remove(raw, hook)
}
