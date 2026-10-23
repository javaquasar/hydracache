use crate::entry_guard;
pub const BLOCK: &str = r#"
<!-- BEGIN BOOTSTRAP BUILD V1 -->
## Signed bootstrap policy and build receipt v1

The independently pinned bootstrap selector selects the complete canonical
newline JSON signed-policy bytes, not an incoming key. The policy envelope
diagnostic-signed-bootstrap-policy-074-v1 contains schema_version, policy and
signature_hex. The policy body diagnostic-bootstrap-build-policy-074-v1 contains
schema_version, purpose, repository_id, builder_id, builder_key_hex,
controller_key_hex, authority_epoch, source (commit, tree, root_lock),
bootstrap_sha256, held_policy_sha256, held_receipt_sha256,
original_host_sha256 and held_launcher_sha256. Its domain is
hydracache-diagnostic-bootstrap-policy-074-v1 followed by NUL and canonical
newline body bytes. Maximum signed policy size is 4096 bytes.

The receipt envelope diagnostic-signed-bootstrap-build-074-v1 contains
schema_version, statement and signature_hex. The statement
diagnostic-bootstrap-build-074-v1 contains schema_version, purpose,
policy_sha256, authority_epoch, observation, bootstrap, root_lock and build_log.
Observation uses the held build v1 field shape with the exact separate bootstrap
build command. Digests contain sha256 and bytes. Its domain is
hydracache-diagnostic-bootstrap-build-074-v1 followed by NUL and canonical newline
statement bytes. Maximum receipt size is 65536 bytes. Signatures use strict
Ed25519 and lowercase 128-character hex. Purpose is
original-sealed-launcher-bootstrap; builder is hydracache-held-bootstrap-074-v1.

Both schemas reject unknown/future/duplicate/missing/noncanonical fields,
wrong signatures/domains/pins/keys/epoch and zero or over-budget digests.
Older readers do not accept these new artifacts; no old schema is reinterpreted.
There is no migration, default key, installed configuration or execution right.
Original-owner join is runtime-only: it consumes the existing material owner,
retains the protected source and sealed image, checks original held policy/receipt
and host/launcher digests inside unchanged complete frames, and retires that same
owner on refusal. Static assertions and model tests do not admit original delivery,
child credentials, execveat, all-writer cessation, completion or server operation.
<!-- END BOOTSTRAP BUILD V1 -->
"#;

#[allow(dead_code)] // Shared by distinct current-source integration test targets.
pub fn compat_view(raw: &str) -> String {
    let raw = entry_guard::compat_view(raw);
    assert_eq!(raw.matches(BLOCK).count(), 1);
    raw.replacen(BLOCK, "", 1)
}
#[allow(dead_code)] // Shared by distinct current-source integration test targets.
pub fn material_view(raw: &str) -> String {
    let hook = "\n#[path = \"diagnostic_bootstrap_owner.rs\"]\nmod bootstrap_owner;\n";
    assert_eq!(raw.matches(hook).count(), 1);
    raw.replacen(hook, "", 1)
}
