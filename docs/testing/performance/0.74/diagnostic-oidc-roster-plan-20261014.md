# Bounded diagnostic OIDC roster implementation

Local implementation was approved on 2026-10-14 after the public-source
observation. Baseline is `c9df8c06e12b7bf6fe99ba77d87a43f85b4a63c9`.
This adds a versioned configured roster, not production key selection, snapshot
installation, token retrieval, protected-job activation or execution admission.

## Version and bounds

Retain the fixed root-only path `/etc/hydracache-perf/oidc-trust-074.json`.
V1 remains canonical LF JSON with exactly one key and its 2048-byte document
limit. V2 uses schema `diagnostic-oidc-trust-input-074-v2`, the same required
principal and snapshot fields, and `keys` instead of `key`. Accept one through
eight explicitly configured RSA/RS256 keys, strictly sorted by nonempty bounded
kid. Reject duplicate kids and duplicate modulus/exponent pairs, unknown fields,
missing/null/type errors, mixed v1/v2 shapes and unknown schemas.

V2 is bounded to 8192 canonical LF bytes. The original OIDC file observation
may read at most this budget before dispatching to the version-specific decoder;
v1 documents larger than 2048 still refuse. Introduce a private bounded snapshot
limit only for this OIDC source. Existing worker/controller source constructors
retain their 2048-byte budget and all metadata/ancestry/drift guards.

Keep RSA modulus 2048 through 4096 bits and exponent 65537, fixed issuer/job
profile, explicit workflow SHA/subject, exclusive snapshot expiry, original
compact-JWS verification, strict claim types/origin and the original request
60-second window unchanged. A token kid only selects one already configured
member; never try another key after signature failure, fall back to v1, fetch
unknown keys or accept token-carried key material. Existing original-input
borrowing, sticky whole-chain refusal and no-export/no-start API remain intact.

## Compatibility and rollback

Readers before this change reject v2, including when it fits the old byte bound.
Current readers accept unchanged v1 and v2. No writer or migration installs or
converts either version. Replacing any original source refuses its retained guard;
rotation needs separately reviewed configuration and a newly enrolled original
chain, not revalidation replacement arguments. Old-binary rollback with v2 must
refuse before admission; reverting to a reviewed v1 configuration is an explicit
operator action, not automatic downgrade. Register both windows in COMPAT.

## Verification and evidence

Capture baseline OIDC/worker-source tests before implementation, then a missing
v2 API compile refusal. Add focused positive and negative tests using only the
already public synthetic RSA fixture and distinct synthetic public modulus
projections; never convert the retained GitHub roster into runtime trust.
Cover all roster count/order/duplicate/type/budget boundaries, v1 preservation,
unknown/future/mixed schemas, each key position, wrong known kid/signature and
all existing claim/header/time guards. Use seed `0x07402a11` for 64 membership
mutations and four independently owned bindings. Exercise v2 source substitution,
unsafe metadata, healthy drop, constructor refusal and whole-chain sticky drift.

Run three focused repeats, affected original-input regressions, compile-fail
doctests, current and MSRV 1.88 checks, strict current lint and formatting, plus
contract/registry/documentation/local-admission checks on local Windows/Linux.
Retain raw logs, exact source/unchanged/frozen hashes and any negative result.
Separate preregistration, tested implementation and final evidence commits.
No network refetch, real private credential, GitHub mutation/job, host service,
manual process-loss test, expensive workload or release qualification is allowed
by this step. Signer/bootstrap custody and immutable body/token handoff remain
the next producer prerequisites.
