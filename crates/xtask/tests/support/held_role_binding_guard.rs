//! Exact additive v4 binding hook and compatibility block, never old exclusions.
pub(crate) const FILE: &str = "tools/long-run-supervisor-074/src/diagnostic_held_build.rs";
#[allow(dead_code)]
pub(crate) const SOURCE: &str = "tools/long-run-supervisor-074/src/diagnostic_held_role_binding.rs";
pub(crate) const HOOK: &str =
    "\n#[path = \"diagnostic_held_role_binding.rs\"]\npub mod role_binding;\n";
pub(crate) const COMPAT: &str = "docs/COMPAT.md";
pub(crate) const BLOCK: &str = r##"
<!-- BEGIN HELD ROLE BINDING V4 -->
## Held role policy and receipt binding v4

diagnostic-held-role-binding-074-v4 and
diagnostic-signed-held-role-binding-074-v4 are separate canonical newline JSON
body/envelope formats, bounded to 40960/49152 bytes. The controller signature
covers hydracache-diagnostic-held-role-binding-074-v4, one NUL and body bytes.
The fixed action is bind-held-build-to-placement. Hex is lowercase; future
versions, unknown/duplicate fields, noncanonical documents, malformed signatures
and over-budget body/envelope/receipt inputs refuse.

The body contains the complete signed v3 placement, held_policy_sha256,
held_receipt_sha256 and authority_epoch. Both controller signatures must verify
under the same controller selected by the independently checked held policy.
The canonical signed held receipt digest, its policy binding and signature must
agree. Its exact launcher/host role hashes must match all placement expectations;
the held policy, receipt and placement consent epochs must agree, with the same
repository 1217101761. The retained v3 bytes bind the original attempt, consent,
root context and absolute deadline. No new replay slot or renewed deadline exists.

This is static cross-document inspection. It neither enrolls the supplied
policy/pin/epoch nor authenticates an original live owner, host, process, FD,
manager or collector. Exact placement-byte/content checks export no originals,
start/send/arm/release, Running/completion or fence-unlock capability. There is
no migration, fallback or upgrade from any worker, v1/v2/v3 or held-build signature.
Old schemas and signed bytes remain unchanged. Original trust/frame/object join
and independent held-key enrollment remain separate gates.
<!-- END HELD ROLE BINDING V4 -->
"##;
pub(crate) fn normalize(path: &str, raw: &str) -> String {
    let exact = match path {
        FILE => HOOK,
        COMPAT => BLOCK,
        _ => return raw.into(),
    };
    assert_eq!(
        raw.matches(exact).count(),
        1,
        "exact role binding addition: {path}"
    );
    raw.replacen(exact, "", 1)
}
#[allow(dead_code)]
pub(crate) fn previous_list(current: &str) -> String {
    assert_eq!(
        current.lines().filter(|p| *p == SOURCE).count(),
        1,
        "exact role binding source"
    );
    current
        .lines()
        .filter(|p| *p != SOURCE)
        .map(|p| format!("{p}\n"))
        .collect()
}
