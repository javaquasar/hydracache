#![allow(dead_code)] // Shared exact compatibility/source views, not snapshots.
pub const BLOCK: &str = r#"
<!-- BEGIN BOOTSTRAP CHILD CENSUS V1 -->
## Bootstrap complete role assertions v1 with authentication closed

--systemd-bootstrap-census-v1 is a separate opt-in mode of the same bootstrap.
It uses the unchanged three activation assertions and twelve role names, not
entry-mode fallback or retry. The earlier --systemd-bootstrap-entry-v1 behavior
and HCBT0741/HCHL0741 bytes remain unchanged; older binaries reject the new mode.

The census checks all slots 0..14, with inherited flags zero: EOF empty stdin,
distinct nonblocking output pipes, empty AF_UNIX/SOCK_SEQPACKET release with
SO_PASSCRED and matching SO_PEERCRED, image/config/legacy identity and bounded
content hashes, live creator/guardian pidfds using fixed procfs fdinfo, and the
same absolute BOOTTIME alarm. Regular material inodes must be distinct; pidfd
inodes may alias on older kernels. Host/launcher/bootstrap/config/legacy/entry
require exact seals; original worker also permits root-owned nonwritable ELF.
Entry revalidation brackets two complete role observations. The procfs FD set
must contain exactly 0..14 plus the enumerator itself; extra descriptors refuse.
Enumeration, fdinfo and content reads have fixed bounds and never repair inputs,
consume queued traffic, create a runtime timer, send, start or exec.

Matching claims is not original provenance: current executable/credentials,
initial namespace, manager/root/cgroup/privileged guardian and signed actual
build/policy enrollment remain unauthenticated. The parent fingerprint is opaque,
not a child remapping map. Even a complete canonical forged role set exits 2 at
authentication-closed; all mismatches also exit 2 without an execution handle.
Unsupported targets refuse. No installation, default activation, migration,
original owner transfer, writer-cessation or server admission follows this mode.
<!-- END BOOTSTRAP CHILD CENSUS V1 -->
"#;
fn remove(raw: &str, addition: &str) -> String {
    assert_eq!(raw.matches(addition).count(), 1);
    raw.replacen(addition, "", 1)
}
pub fn compat_view(raw: &str) -> String {
    remove(raw, BLOCK)
}
pub fn source_view(name: &str, raw: &str) -> String {
    match name {
        "bin/diagnostic_held_bootstrap.rs" => {
            let raw = remove(
                raw,
                "\n#[path = \"../diagnostic_bootstrap_census_entry.rs\"]\nmod census_entry;\n",
            );
            remove(&raw,"    if census_entry::requested() {\n        if let Err(error) = census_entry::run() {\n            eprintln!(\"Bootstrap refused: {error}; no start, send or executable handoff.\");\n        }\n        std::process::exit(2);\n    }\n")
        }
        "diagnostic_bootstrap_child_reader.rs" => remove(
            raw,
            "\n#[path = \"diagnostic_bootstrap_child_roles.rs\"]\npub(super) mod roles;\n",
        ),
        _ => raw.into(),
    }
}
