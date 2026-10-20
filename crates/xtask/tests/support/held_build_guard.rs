//! Exact additive build-authority normalization; never excludes old source files.
pub(crate) const BUILDER: &str = "tools/long-run-supervisor-074/src/diagnostic_builder.rs";
pub(crate) const HOOK: &str = "\n#[path = \"diagnostic_held_build.rs\"]\npub mod held_build;\n";
pub(crate) const COMPAT: &str = "docs/COMPAT.md";
pub(crate) const BLOCK: &str = r##"
<!-- BEGIN HELD BUILD AUTHORITY V1 -->
## Purpose-bound held runtime build v1

The diagnostic-held-build-policy-074-v1 policy pins the exact source commit,
tree and root lock, repository 1217101761, builder identity and a separate key,
controller and expected epoch. Its independently supplied digest and the legacy
worker policy's independent digest are mandatory. Both policies must name the
same controller; the held builder, worker builder and controller keys differ.
This is static reviewed configuration, not live issuer enrollment or epoch proof.

The diagnostic-held-build-074-v1 body and diagnostic-signed-held-build-074-v1
envelope have the purpose original-host-and-held-launcher. The signature covers
hydracache-diagnostic-held-build-074-v1, one NUL and canonical newline-terminated
body JSON. Policy/envelope limits are 4096/65536 bytes; unknown/duplicate fields,
noncanonical JSON, unknown versions, invalid keys/signatures and mismatched pins,
epochs or recipe refuse. Hex is lowercase. There is no migration, fallback or
upgrade from the frozen worker receipt or any placement/launch v1/v2/v3 format.

The fixed Cargo 1.94.0 Linux GNU release recipe selects only the supervisor host
and diagnostic_held_launcher binaries, locked and without features. Observation
assertions bind clean before/after source/tree, exact tools and isolated build.
Signed digests/lengths bind two distinct ELF images, root lock and raw Cargo log.
Binary/lock/log bounds are 128 MiB/1 MiB/16 MiB; log records are at most 1 MiB.
Require the two exact package/target/source/output/profile records and one final
successful event; missing, duplicate or foreign binaries and trailing events
refuse. Other Cargo metadata is retained through the complete log digest.

The fixed role map is launcher=held_launcher and watchdog=collector=original_host.
It attests the trusted build job's assertion, not independent reproducibility,
actual role-file identity, an active collector, original owner or execution.
Checked output only compares content/expected digests. No start/send/renewal,
completion or unlock capability follows; all existing live checks and deadlines
remain separate. The launcher target is a reserved recipe name, not an installed
executable or permission to replace the original worker ExecStart.
<!-- END HELD BUILD AUTHORITY V1 -->
"##;
pub(crate) fn normalize(path: &str, raw: &str) -> String {
    let exact = match path {
        BUILDER => HOOK,
        COMPAT => BLOCK,
        _ => return raw.to_owned(),
    };
    assert_eq!(
        raw.matches(exact).count(),
        1,
        "exact held build addition: {path}"
    );
    raw.replacen(exact, "", 1)
}
