#![allow(dead_code)] // Exact additive views for preceding contracts.
#[path = "bootstrap_installed_child_guard.rs"]
pub mod child;
pub const BLOCK: &str = r#"
<!-- BEGIN BOOTSTRAP INSTALLED MATERIAL V1 -->
## Parent retained installed entry materialization v1

The opt-in consuming wrapper retains HCBI0741 under the same InstalledBootstrapUnit.
It accepts no caller bytes/path/FDs or replacement owner. Original HCBT0741 is
read from its retained readonly/CLOEXEC/four-sealed zero-offset object with
stamp and exact prior-census brackets. Its installed prefix must match the old
27-FD/15-role census. A separate domain-separated parent fingerprint covers
fixed-order roles/FDs and original-entry, installed-image and ancestry stamps.
This is a parent-local assertion checksum, not a signature or child remapping.

The old sealed constructor supplies one readonly/CLOEXEC/four-sealed 1280-byte
memfd with no writable alias at publication and zero retained offset. Complete
installed-owner frames bracket two new inspections; each compares exact bytes,
retained metadata/flags/offset and repeated original expectations. Parent census
extends the exact 32-FD/16-role prefix to 33/17, rejecting old FD/inode aliases.
HCBI0741 bytes still describe its input 32/16 prefix, without a self-reference.
Any error latches the same owner; missing retained resource cannot be recreated.
Constructor failure keeps the refused owner, and retirement precedes closing
published retained descriptors. Internal sealed-constructor temporaries retain
their existing cleanup behavior.

Old record formats, child modes, predicates and absolute windows are unchanged.
Older binaries lack this optional consuming join; no migration, fallback or
default activation exists. This is not durable completion, child transport or
FD export, original manager/sandbox authentication, send/start/exec/release,
writer cessation, lifecycle progression or server admission.
<!-- END BOOTSTRAP INSTALLED MATERIAL V1 -->
"#;
pub fn hook(name: &str) -> Option<&'static str> {
    match name {
        "diagnostic_bootstrap_installed_owner.rs" => Some("\n#[path = \"diagnostic_bootstrap_installed_entry_owner.rs\"]\nmod entry_material;\n"),
        "diagnostic_bootstrap_installed_census.rs" => Some("\n#[path = \"diagnostic_bootstrap_installed_entry_census.rs\"]\npub(crate) mod entry_material;\n"),
        "diagnostic_bootstrap_entry_resource.rs" => Some("\n#[path = \"diagnostic_bootstrap_installed_entry_resource.rs\"]\npub(in super::super) mod installed_material;\n"),
        _ => None,
    }
}
fn remove(raw: &str, addition: &str) -> String {
    assert_eq!(raw.matches(addition).count(), 1);
    raw.replacen(addition, "", 1)
}
pub fn source_view(name: &str, raw: &str) -> String {
    let raw = child::source_view(name, raw);
    let raw = raw.as_str();
    match hook(name) {
        Some(hook) => remove(raw, hook),
        None => raw.into(),
    }
}
pub fn compat_view(raw: &str) -> String {
    remove(&child::compat_view(raw), BLOCK)
}
