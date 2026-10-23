#![allow(dead_code)] // Exact additive view for the historical source/compat guards.
pub const BLOCK: &str = r#"
<!-- BEGIN BOOTSTRAP INSTALLED ENTRY V1 -->
## Installed bootstrap entry assertions v1

HCBI0741 is a separate opt-in canonical 1280-byte assertion envelope, not an
authenticated message or execution permit. It embeds unchanged HCBT0741 bytes
at 16..1040, installed device/inode/size at 1040, content SHA-256 at 1064,
fixed pathname SHA-256 at 1096, opaque parent inventory SHA-256 at 1128,
little-endian parent counts 32/16 at 1160/1164 and embedded-entry SHA-256 at
1168. Header padding 12..16 and trailing padding 1200..1280 must be zero.
The fixed pathname is /opt/hydracache-perf/bin/diagnostic-held-bootstrap-074.
Counts describe the parent inventory, not inherited child FD slots.

Installed identity components must be nonzero, size at most 128 MiB and equal
to the sealed reference size, content hash equal to that reference, and its
device/inode pair distinct from every embedded regular-image identity.
Unknown magic, length, padding, counts, path digest, embedded digest, embedded
shape or cross-image mismatch refuses. Old/new readers reject each other's
records; there is no fallback, migration or changed old child mode.

All fields remain untrusted assertions, including original owner, times and
inventory fingerprint. Canonical consistency does not establish signed policy,
filesystem/custody, credentials, namespaces or manager authentication. This
component adds no record materialization, child inventory remapping, transport,
start/send/exec/release, writer cessation, completion or server admission.
Original absolute windows, old record bytes and live validation stay unchanged.
<!-- END BOOTSTRAP INSTALLED ENTRY V1 -->
"#;
pub const HOOK: &str = "\n#[path = \"diagnostic_bootstrap_installed_entry_format.rs\"]\npub mod installed_entry_format;\n";
fn remove(raw: &str, addition: &str) -> String {
    assert_eq!(raw.matches(addition).count(), 1);
    raw.replacen(addition, "", 1)
}
pub fn compat_view(raw: &str) -> String {
    remove(raw, BLOCK)
}
pub fn source_view(name: &str, raw: &str) -> String {
    if name == "diagnostic_bootstrap_build.rs" {
        remove(raw, HOOK)
    } else {
        raw.into()
    }
}
