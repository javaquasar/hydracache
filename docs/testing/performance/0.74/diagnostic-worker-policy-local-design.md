# Signed local worker policy verification

## Approved authority model and local scope

The user approved the fixed local account model after the
[authority audit](diagnostic-host-policy-audit.md). This first implementation
verifies policy bytes only. Baseline is `089e94a73891d06b8fddc1722c47fd01ee5399f9`.
It adds no NSS lookup, file reader, host installation, production preparation,
signing CLI, live helper/IPC mode or start route. The verifier is portable; actual
fixed-file and namespace observation remain separate Linux work.

## Separate schema and external trust

Use `diagnostic-worker-policy-074-v1` for the body and
`diagnostic-signed-worker-policy-074-v1` for the envelope. Canonical JSON uses
sorted keys and one final LF. Body limit is 4096 bytes; envelope limit is 8192.
The Ed25519 message is the distinct ASCII domain
`hydracache-diagnostic-worker-policy-074-v1`, a zero byte and canonical body
including LF. Strict signature verification uses the existing pinned
[ed25519-dalek 2.2.0 API](https://docs.rs/ed25519-dalek/2.2.0/ed25519_dalek/struct.VerifyingKey.html#method.verify_strict).
The lowercase 128-digit signature is not a source of its verification key.

External trust supplies the issuer public key, body SHA-256 pin and nonzero
policy epoch. The body contains no key or alternative trust root. Require the
fixed repository, account/group `hydracache-perf`, diagnostic-only purpose and
`local-files` source. Retain exact nonroot UID/primary GID, at most 32 sorted unique
nonroot supplementary GIDs; their union with primary GID also fits 32. Never
reinterpret the supplementary list as NSS memberships. Bind approved hashes of
the complete passwd/group documents for later fixed-file inspection.

Bind canonical nonzero machine ID and boot UUID plus separately named user/mount
namespace identities (device/inode). These context values are explicit caller
assertions in this slice, not kernel observations or initial namespace proof.
Namespace identity objects require nonzero device/inode; verification compares
them with the asserted host context. Context validation does not enroll a host.
The issuer/key pin and latest epoch must come from separately reviewed operator
configuration, never from the signed document or existing receipt v1.

## Original policy and refusal

The checked runtime guard privately retains original policy, trust, asserted
context and envelope digest. Revalidation accepts current external trust/context
and bytes but cannot refresh any original field. Trust changes return Revoked;
context changes return Context; envelope substitution returns Drift. Invalid
bytes/signature or any failure latch refusal. A later valid original cannot
restore the guard. Refusal is checked before new validation and no file/helper
operation exists. Do not expose a policy extractor, serialization, reset or
conversion into existing account/process/start capabilities.

The epoch is an equality pin to the externally supplied current epoch, not an
internally durable monotonic registry. Replaying the same valid policy in its
same admitted asserted context is permitted idempotent verification, not a new
launch. An older pin supplied again to a new guard cannot be detected here;
durable operator pin/epoch management and real boot/context revalidation remain
required. No wall-clock correctness or implicit grace window is introduced.

## Tests and gates

Record the existing Windows library/artifact baseline and missing-module API
refusal before implementation. Test strict JSON, duplicate/unknown/future fields,
canonical bytes, budgets, external pins/keys/epochs, weak keys, signature changes
and foreign domains, fixed identity, numeric/group boundaries, exact asserted
context, first error/sticky refusal, restoration and independent guards.
Seed valid resigned mapping/file-digest mutations and retain the seed.
Positive signatures use deterministic local test keys only; no real issuer
enrollment or account-file observation is claimed.

Run portable and Linux targeted tests, affected all-target check/all-feature
strict lint, scoped format, root contracts/evidence/governance, documentation,
links/book and expected-red require-ship. Register the new schemas in COMPAT and
retain raw hashes. Preserve frozen qualification input bytes and all existing
numeric, NSS, namespace and production receipt APIs. The next slice is bounded
fixed-file inspection followed by actual admitted-context enforcement, not
production permission from successful policy decoding alone.

## Implemented verifier and retained local results

Implementation `fd93def1013c78a60a0765844213207c71c737e1` adds the portable
`diagnostic_worker_policy` module, strict unverified wire models, independently
constructed `WorkerPolicyTrust` and privately constructed `CheckedWorkerPolicy`.
The guard exposes only original policy digest, refusal status and revalidation.
Existing account/numeric/namespace APIs, receipt v1, production routes,
provisioning assets and workflows remain unchanged. The account-file hashes are
approved expectations only; no file inspection is implemented in this module.

The [packet](local-runs/diagnostic-worker-policy-20261010/manifest.json) retains
the 30-pass Windows baseline, E0432 missing-module refusal with native exit 101,
twelve-case first green and reviewed thirteen-case coverage. Final Windows scope
passes 43; Linux passes 269 with one existing system-bus case ignored. Three
additional Linux repetitions pass all thirteen cases. Seed `0x7512026` checks
256 valid re-signed UID/GID or account-document-digest changes per repetition:
each verifies under its own new pin and refuses under the original pin. This
distinguishes trust-pin refusal from invalid policy or signature construction.

Tests also cover weak/wrong keys, other signature domains, malformed signatures,
strict canonical/duplicate/future fields, budgets, nonroot exact group/union
rules, exact asserted host/boot/namespace context, trust/key/pin/epoch revocation,
oversized revalidation, first error, sticky restoration refusal, idempotence and
independent concurrent guards. These positives use local synthetic operator keys
and assertions; none enrolls a real issuer, host, namespace or process.

Root contracts/evidence/governance pass 128 checks. Windows/Linux scoped
check/strict lint and format pass, including repeated xtask check/lint after
review strengthened the schema/domain/budget assertions. Doc/local performance
contract, links/book pass; require-ship remains expected native exit 1. The frozen
qualification digest is unchanged. No performance measurement, host mutation,
product workload, qualification or full workspace milestone gate ran.

The subsequent [fixed local account-file reader](diagnostic-worker-files-local-design.md)
borrows this checked policy under the same current trust/envelope/context gates.
It adds separate Linux file consistency without changing the signed schemas or
turning asserted context into live host proof. The policy-byte results above are
the historical first slice; real issuer/context/start enrollment remains closed.

The later [observed Linux context guard](diagnostic-worker-context-local-design.md)
compares actual fixed identity documents and typed reading-thread namespaces
with those original expectations. It does not alter this verifier's assertion
API, choose external trust pins or turn local observations into initial-host
authority. Composition with account files and credentials remains separate.
