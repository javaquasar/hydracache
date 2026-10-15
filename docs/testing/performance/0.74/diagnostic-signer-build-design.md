# Separate signer build verification model

The local model gives controller-signer assertions their own receipt format,
signature domain and issuer role. It verifies synthetic signatures and supplied
artifact bytes without borrowing the frozen observer builder's authority. A
successful `CheckedSignerBuildModel` exposes only a diagnostic receipt digest;
it is neither independently authenticated bootstrap nor an execution capability.

## Receipt and supplied selection

`SignerBuildStatement` binds repository, role, clean source assertions, commit
and tree, exact toolchain/command/target/profile/features/allocator, and five
SHA256/length pairs. These cover binary, root lock, build log, runtime manifest
and bootstrap context. Runtime/context bytes are opaque. Matching them does not
inspect a loader, shared libraries, mapped executable or protected job.

The schema is `diagnostic-signer-build-074-v1`. Ed25519 strict verification uses
`hydracache-diagnostic-signer-build-074-v1`, NUL and canonical statement JSON.
Receipt encoding is canonical JSON followed by exactly one LF, at most 8192
bytes. Unknown/duplicate/missing fields, invalid types, alternate formatting,
future schema, bad signature encoding and any expected-statement drift refuse.
All artifacts must be nonempty; limits are 128 MiB binary, 1 MiB lock, 16 MiB
log and 64 KiB per context. Supplied lengths are checked before hashing.

`AssertedSignerBuildSelection::new` accepts an explicitly asserted local issuer
and full expected statement. There is no default production issuer or loader.
It rejects weak keys and the existing reviewed controller, observer builder and
worker issuer keys. Caller-selected expectations are not independent policy:
building them from an incoming receipt would add no provenance. No current
authority epoch, revocation status or validity window is invented for this
immutable build model; live authorization remains a separate original chain.

## Deliberately closed integration

The checked model has private fields, no Deserialize/Clone and no conversion to
the observer's `VerifiedBuild`. It cannot export a path/descriptor, authorize
signing, enroll a host or start a worker. The portable module has no IO. The
subsequent [original file reader](diagnostic-signer-bundle-design.md) uses a
crate-private digest seam for explicitly asserted Linux fixtures, retaining
inventory only. No production file reader, CLI, receipt producer, workflow,
bootstrap factory, token request, credential access or server route consumes
either result. Existing reviewed
pins, observer readers, producer semantics and signed domains remain unchanged.

An actual bootstrap still needs independent issuer/pin selection and acquisition
of the original prebuilt signer, runtime and reviewed context before exposing
private-key or job-wide token authority. Source IDs, clean flags, logs and a
matching supplied binary are signed assertions, not compilation proof. The
test positive intentionally accepts `not ELF` with opaque synthetic context to
make that boundary observable, rather than hiding it behind a realistic fixture.

## Tests and next integration

The optional Linux [original GNU runtime join](diagnostic-signer-runtime-join-design.md)
now obtains binary bytes from the retained signer and regenerates canonical GNU
catalog bytes for the runtime-manifest slot. It reuses this exact verifier and
domain without changing the old opaque-slot reader. Its selection still remains
explicitly asserted; original numeric-root observations are not independently
enrolled runtime/job custody.

The [preregistration](diagnostic-signer-build-plan-20261014.md) fixes tests and
bounds before implementation. Integration tests mutate every statement field,
every signature position and each artifact; refuse the known old key roles,
wrong domain/key and malformed schemas; and demonstrate mutual refusal with the
retained observer receipt. Seed `0x07403b22` covers 64 content substitutions and
four concurrent independent models. Compile-fail examples forbid clone,
deserialization and observer-build conversion.

Selecting production signer authority and proving original executable/runtime
custody must precede coupling this model to retained request/token coordination.
Original replay/common fence/start, all-writer cessation, completion and later
fixed cells remain independent prerequisites for server/pilot/qualification.
