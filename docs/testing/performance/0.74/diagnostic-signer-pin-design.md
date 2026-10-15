# Selected signer issuer without prebuilt custody admission

The operator selected raw-public fingerprint
`0c22ba7d6c6d16b334e4b1d4a0ce161117800135a977aa30020906caa70af884`
for the separate signer-build issuer role on October 15, 2026. The
[selection record](diagnostic-signer-issuer-selection.json) records that decision
without selecting an artifact, runtime, job, signing route or server admission.
The [preregistration](diagnostic-signer-pin-plan-20261015.md) fixes this scope.

## Code selected key and explicitly asserted build facts

The portable `diagnostic_signer_issuer` module has fixed raw key/fingerprint
constants and no key/path/environment/record override. It checks the fingerprint,
weak-key refusal and separation from the old controller, observer-builder and
worker-policy issuer roles. Those existing pins and their module stay unchanged.
The separate key helper and original public generation record also stay unchanged;
their unapproved flags describe generation, not the later operator decision.

`asserted_build_selection(expected)` binds only this selected issuer to the
existing `AssertedSignerBuildSelection`. The expected statement still comes from
the caller and remains explicitly asserted. The old result exposes a diagnostic
receipt digest, not an authenticated executable, runtime or job. No signing,
bootstrap, token, replay or worker consumer is connected. Selection of a key
does not independently establish its current revocation status or admit whatever
artifact a receipt happens to describe.

Tests verify the original public selfcheck and all 64 signature-byte mutations
without opening a private key or producing a new signature. A fully synthetic
matching receipt passes the old caller-key model but fails the selected-key
factory. Invalid expected profiles still refuse. Four independent threads each
construct 32 selections; compile-fail coverage forbids a caller issuer override.
No actual positive signer-build receipt is claimed.

## Unsigned prebuilt inventory and remaining join

A separate local build of `diagnostic_controller` uses the existing Rust 1.94
release/System/locked/GNU command from a clean exact source. The binary is not
executed. Readelf records only static ELF metadata: interpreter, declared shared
dependencies and loader search tags. Binary/source/lock hashes and compiler
observations bind that one unsigned build, not reproducibility or independently
authenticated compilation. Readelf declarations are not the loaded runtime
closure, root ownership, absence of writers, environment or TLS trust custody.

Before production use, independently admit the exact build statement/receipt and
original prebuilt files, their launcher/runtime root and protection, and the
workflow/subject/issuer-snapshot/authority policy. Neither this getter nor an
unsigned hash may manufacture that admission. The disabled job draft remains
disabled outside `.github/workflows`; no workflow configuration or credential
operation is authorized here. Worker original-start, every P2/P3 writer's cessation,
completion cells 2–4, server installation/rehearsal and qualification remain separate.
