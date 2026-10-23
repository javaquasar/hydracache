#![allow(dead_code)]
#[path = "bootstrap_held_entry_guard.rs"]
pub mod entry;
pub const BLOCK: &str = r#"
<!-- BEGIN HELD READINESS EXCHANGE V1 -->
## Held delivery readiness candidate v1

The private parent candidate addresses StartTransientUnit to the retained unique
manager on the same connection. It preserves the original serial and hardening,
uses mode fail, an empty auxiliary list, the fixed installed bootstrap path and
the reserved argument --systemd-bootstrap-held-readiness-v1. It transfers three
stdio FDs and fifteen ExtraFileDescriptors in a(hs) order: the old fourteen names
followed by readiness. FD0..16 retain their old meanings; FD17 is a separate
SOCK_SEQPACKET readiness endpoint. Existing release-channel emptiness checks and
the historical two-FD exchange are unchanged. Unsupported manager properties
fail without fallback. The new child argument remains unactivated and refuses
under existing binaries; it is not an installed execution or enrollment gate.

HCHR0741 is a fixed 256-byte little-endian challenge: magic at 0, original
absolute BOOTTIME bound at 8, creator PID/UID/GID at 16/20/24, child PID at 28,
nonce at 32, installed-entry digest at 64, original-context digest at 96,
manager/client name digests at 128/160, invocation ID at 192 and zero reserved
bytes at 208..256. HCHA0741 is an 80-byte readiness acknowledgement: magic,
u64 length 80, SHA256 of domain hydracache-held-readiness-074-v1 followed by NUL
and all challenge bytes, then the original nonce. Unknown versions, sizes,
reserved bytes, peer or context mismatches refuse. This checksum is not a
signature and conveys no execution authority.

Each side uses atomic SCM_CREDENTIALS and SCM_PIDFD reception, with exact peer
PID/UID/GID and live pidfd checks. SO_PEERCRED and packet PID assertions are not
message authentication. Parent full manager/process/original-owner frames must
bracket exchange operations. Both send windows latch before any possible send;
failure or lost acknowledgement never permits retransmission. No admission
origin or deadline is renewed. A correlated manager job and child readiness are
distinct observations; neither grants release, completion or writer cessation.
No migration or automatic activation exists. Original record formats, old child
modes, qualification inputs and absolute windows remain unchanged.
<!-- END HELD READINESS EXCHANGE V1 -->
"#;
pub fn compat_view(raw: &str) -> String {
    let raw = entry::compat_view(raw);
    let raw = raw.as_str();
    assert_eq!(raw.matches(BLOCK).count(), 1);
    raw.replacen(BLOCK, "", 1)
}
pub fn hook(name: &str) -> Option<&'static str> {
    match name {
        "diagnostic_bootstrap_installed_entry_census.rs" => {
            Some("\n#[path = \"diagnostic_held_delivery_wire.rs\"]\npub(crate) mod held_wire;\n")
        }
        "diagnostic_manager_channel.rs" => {
            Some("\n#[path = \"diagnostic_manager_held_delivery.rs\"]\nmod held_delivery;\n")
        }
        "diagnostic_bootstrap_sandbox_owner.rs" => {
            Some("\n#[path = \"diagnostic_original_held_delivery.rs\"]\nmod held_delivery;\n")
        }
        _ => None,
    }
}
pub fn source_view(name: &str, raw: &str) -> String {
    let raw = entry::source_view(name, raw);
    let raw = raw.as_str();
    if let Some(hook) = hook(name) {
        assert_eq!(raw.matches(hook).count(), 1);
        raw.replacen(hook, "", 1)
    } else {
        raw.into()
    }
}
