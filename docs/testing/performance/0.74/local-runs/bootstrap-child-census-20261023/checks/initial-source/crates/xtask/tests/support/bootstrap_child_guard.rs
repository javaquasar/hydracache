#![allow(dead_code)] // Exact compatibility view shared by current-source contracts.
#[path = "bootstrap_roles_guard.rs"]
mod roles_guard;
pub fn source_view(name: &str, raw: &str) -> String {
    roles_guard::source_view(name, raw)
}
pub const BLOCK: &str = r#"
<!-- BEGIN BOOTSTRAP CHILD READER V1 -->
## Bootstrap entry reader v1 with authentication closed

The separate diagnostic_held_bootstrap binary accepts exactly one argument,
--systemd-bootstrap-entry-v1, and only LISTEN_PID (its canonical decimal PID),
LISTEN_FDS=12 and LISTEN_FDNAMES=release:worker:config:creator:root-a:root-b:alarm:bootstrap:host:launcher:bootstrap-image:entry.
These metadata fields are assertions, not manager or FD authentication. Proposed
slots 0..10 preserve held launcher semantics; host/launcher/bootstrap-image/entry
are 11/12/13/14. No parent remapping or transport is enabled. Only entry FD 14
is inspected in this implementation, not the complete inherited-role census.

Entry FD must be regular, readonly/non-O_PATH, exactly four-sealed and 1024 bytes,
with descriptor flags zero as inherited across exec and initial offset zero.
Bounded positional reads compare metadata/status/seals/offset before and after;
HCBT0741 structural decoding returns assertions only. A second fresh observation
must match. Unknown/missing/duplicate/noncanonical environment, extra/future
arguments, missing FD, wrong shape/size/access/seals or metadata drift refuse.
No environment, flag, deadline or descriptor repair, reopen, fallback or retry.

All paths, INCLUDING canonical forged records, exit 2 without start/send/execveat;
successful reading reaches explicit original-authentication-closed refusal.
Unsupported targets also refuse. Old readers reject this new mode; no migration
or implicit activation. Actual credentials, manager/guardians/root and every
inherited role, signed build enrollment and original owner connection remain
required before delivery/execution or server admission. Reader success is not
an authenticated entry, handoff receipt, completion or all-writer cessation.
<!-- END BOOTSTRAP CHILD READER V1 -->
"#;
pub fn compat_view(raw: &str) -> String {
    let raw = roles_guard::compat_view(raw);
    assert_eq!(raw.matches(BLOCK).count(), 1);
    raw.replacen(BLOCK, "", 1)
}
