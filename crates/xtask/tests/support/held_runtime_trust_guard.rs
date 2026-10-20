//! Exact additive early held trust source, not an old-source exclusion.
pub(crate) const FILE: &str =
    "tools/long-run-supervisor-074/src/diagnostic_controller_trust_input.rs";
#[allow(dead_code)]
pub(crate) const SOURCE: &str =
    "tools/long-run-supervisor-074/src/diagnostic_held_runtime_trust.rs";
pub(crate) const HOOK: &str =
    "\n#[path = \"diagnostic_held_runtime_trust.rs\"]\npub mod held_runtime;\n";
pub(crate) const COMPAT: &str = "docs/COMPAT.md";
pub(crate) const BLOCK: &str = r##"
<!-- BEGIN HELD RUNTIME TRUST V1 -->
## Fixed held runtime trust input v1

diagnostic-held-runtime-trust-074-v1 is canonical newline JSON, at most 2048
bytes, read only from /etc/hydracache-perf/held-runtime-trust-074.json. Its sole
public opener requires root UID/GID and offers no path/key/pin override. Root,
ancestors and regular single-link leaf are retained with no-follow descriptors;
unsafe ownership/modes, replacement, metadata/content drift or malformed input
cause sticky refusal. There is no writer, default held key/pin or enrollment.

Fields are schema_version, repository_id, controller_key_hex,
held_builder_key_hex, held_policy_sha256, worker_builder_policy_sha256,
worker_policy_sha256 and policy_epoch. Hashes/keys are lowercase, epoch nonzero,
repository 1217101761. Unknown/duplicate/missing/null fields and noncanonical or
future documents refuse. The protected file independently selects the held key
and policy digest; held key must differ from controller, old worker builder and
worker issuer. The latter three keys and old builder-policy pin remain reviewed
constants. Incoming v4/policy documents cannot choose these trust selectors.

OriginalHeldTrust borrows the same SignedControllerInput and held source. Fresh
observations retain the actual controller key, original request digest and active
worker policy digest/epoch/revision/trust snapshot. Both sources and the original
kernel/authority chain are revalidated around policy inspection, with sticky
refusal of both borrows on any error. Optional v4 inspection also matches the
original reservation request, controller, identity, source, preset and actual
worker authority. This early read-only guard exports only static Inspection,
not a controller, policy/FD original, start/send/replay or fence-unlock capability.

This is not late consuming placement/role-object/full-frame admission, consent
receive, durable intent, root/process/Armed proof, production enrollment or server
admission. No migration/fallback or changes to old signed formats, live checks,
46 brackets or original 300/60/10 origins/deadlines are introduced.
<!-- END HELD RUNTIME TRUST V1 -->
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
        "exact held runtime trust addition: {path}"
    );
    raw.replacen(exact, "", 1)
}
#[allow(dead_code)]
pub(crate) fn previous_list(current: &str) -> String {
    assert_eq!(
        current.lines().filter(|p| *p == SOURCE).count(),
        1,
        "exact held runtime trust source"
    );
    current
        .lines()
        .filter(|p| *p != SOURCE)
        .map(|p| format!("{p}\n"))
        .collect()
}
