# Separate signer build receipt model

Baseline is `99beb76dbcb8d10c4d4c9d0b46ca9cbfa3b68d22`. The approved next step
models a dedicated signer-build receipt using synthetic keys locally. It does
not select a production issuer, attest an actual build or activate bootstrap.

## Fixed local verification scope

Add portable `diagnostic_signer_build` with the separate schema and signature
domain `diagnostic-signer-build-074-v1` and
`hydracache-diagnostic-signer-build-074-v1` respectively. Domain framing is
domain bytes, NUL and canonical statement JSON. The receipt is strict canonical
LF JSON, at most 8192 bytes, with one Ed25519 strict signature in lowercase hex.
Unknown, duplicate, missing, noncanonical and future fields/schema refuse.

The statement fixes repository 1217101761, role `diagnostic-controller-signer`,
Linux x86_64 target, release profile, empty features, System allocator, no counting
allocator, clean source assertions, and exact Rust/Cargo 1.94.0 version strings.
The command is `cargo +1.94.0 build -p hydracache-long-run-supervisor-074 --bin
diagnostic_controller --release --locked --target x86_64-unknown-linux-gnu`.
Explicit assertions select the source commit/tree and SHA256/length of binary,
root lock, build log, runtime manifest and bootstrap context. Bounds are 128 MiB
binary, 1 MiB lock, 16 MiB log, and 64 KiB for each context, all nonempty.
Source IDs are 40 lowercase hex characters; artifact digests are 64 lowercase hex.

`AssertedSignerBuildSelection::new` validates an explicitly supplied key and
expected statement. Its name and documentation must make its unauthenticated
provenance clear. Reject weak keys and all three existing reviewed controller,
observer-builder and worker-issuer pins. Verification requires an exact expected
statement, valid domain signature and matching supplied content slices. Contexts
are opaque digest-bound bytes, not independently inspected runtime facts.

Return only an opaque `CheckedSignerBuildModel` with its receipt digest. No
Deserialize/Clone, execution identity conversion, FD/path access, signing
authorization, production factory, default pin, file reader, CLI, workflow,
network access or receipt producer is added. Tests sign synthetic statements
directly; there is no production signing API. Old observer readers, pins,
producer, lockfiles and frozen qualification artifacts stay unchanged.

## Baseline and tests

Before implementation, retain existing builder/producer regression output and
a missing-module compile refusal. Add integration cases for the synthetic
positive; all statement/purpose/selection mutations; known key-role reuse;
wrong key/domain/signature; malformed JSON and bounds; every supplied artifact
substitution; and mutual refusal with the old observer receipt. Seed
`0x07403b22` covers 64 content mutations and four concurrent independent models.
Compile-fail examples prohibit clone, deserialization and conversion to the
existing verified observer build. No timing assertions or real credentials.

Run three Linux focused repeats, Windows focused cases, builder/producer/OIDC
regressions, doctests, package check and strict Clippy on Rust 1.94.0, and MSRV
1.88 checks on Windows/Linux. Register the schema in COMPAT and assert its
closed authority boundary in the local contract/root tests. Run formatting,
root contracts and documentation/performance checks; require-ship must remain
red. Retain raw captures and exact tested-source/unchanged hashes separately.

## Next independent decisions

Production issuer/pin selection and authenticated artifact acquisition remain
separate from this asserted model. A bootstrap must identify the original
executable, streams/runtime and reviewed context before granting secrets or
job-wide token permission; a matching digest alone does not establish custody.
Actual token coordination, original replay/fence/start, all-writer cessation,
completion/cells 2–4 and server/pilot/qualification remain closed.
