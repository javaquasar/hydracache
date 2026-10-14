# Bounded configured OIDC signing keys

The approved local v2 reader supports issuer rotation through a bounded configured
roster. It does not select or install the four keys from the
[public observation](local-runs/diagnostic-oidc-public-source-20261014/README.md).
The [preregistration](diagnostic-oidc-roster-plan-20261014.md) fixes the scope and
bounds against `c9df8c06e12b7bf6fe99ba77d87a43f85b4a63c9`.

## Reader compatibility

Keep the fixed root-only original file at
`/etc/hydracache-perf/oidc-trust-074.json`. The reader accepts canonical LF v1
with its single `key` and 2048-byte document bound. V2 uses
`diagnostic-oidc-trust-input-074-v2`, replaces `key` with `keys`, and accepts one
through eight entries within 8192 bytes. Both require the existing repository,
issuer, workflow ref/SHA, environment, subject and explicit snapshot window.
Unknown versions, mixed shapes, missing/null/duplicate/unknown fields and
noncanonical bytes refuse. Old readers reject v2, even below 2048 bytes.

The private snapshot primitive retains its own observation limit. Only the OIDC
source uses the 8192-byte read bound; the version-specific decoder still refuses
v1 above 2048. Worker/controller readers keep the default 2048-byte bound.
Original no-follow ancestry, file metadata, bytes and read brackets are retained;
the named and retained observations must also have the same limit.

No writer or migration converts or installs trust. A source replacement refuses
the original guard and latches the borrowed original controller/worker chain.
Rotation requires separately reviewed membership, validity and custody plus a
newly enrolled original chain. Revalidation takes no replacement argument. Old-
binary rollback with v2 refuses before admission; explicit reviewed v1 enrollment
is not an automatic downgrade. See [COMPAT](../../../COMPAT.md).

## Key selection and unchanged verification

Kids are bounded nonempty text and strictly increasing. Every member independently
passes the unchanged RSA/RS256/signing-use, 2048 through 4096-bit canonical modulus
and exponent 65537 checks. Duplicate modulus/exponent pairs refuse even under
different kids. Neither invalid unused keys nor ambiguous ordering is tolerated.

The strict JWT header's kid identifies exactly one configured member. Signature
failure never searches another key. Unknown kid never triggers a download,
fallback or trust update. The original compact-JWS signing bytes, fixed principal
profile, request-derived audience, run/attempt/actor, strict claim types, reusable-
job refusal and exclusive token/snapshot/request windows use the same verifier
for both versions. Token lifetime remains at most 600 seconds; original packet
and request freshness remain exclusively bounded by 60 seconds. No clock input
or policy epoch is added.

The runtime guard still borrows the original trust and controller/build inputs,
owns only bounded token bytes, checks the original kernel clock/context and
latches every input on failure. Healthy drop preserves owners. No raw token,
claims, key, FD, start or completion API appears.

## Local proof and remaining job boundary

Synthetic fixtures cover signing at every roster position, wrong known kid with
the correct signature for a different member, unknown kid, shape/order/material
ambiguity, size/RSA/time/header/origin refusal and v1 compatibility. Original
owned-source cases test constructor failure, substitution, valid rotation,
downgrade attempts, unsafe metadata and permanent whole-chain refusal. Seed
`0x07402a11` covers 64 membership/signature mutations and four independent
bindings. Positive cryptographic tests use the already public RSA test fixture;
other modulus projections are explicitly synthetic.

This closes the local single-key schema limitation, not production issuer or job
enrollment. Signer/bootstrap custody, immutable request retention through token
retrieval/signing and authenticated handoff remain in the
[job preparation](diagnostic-controller-job-preparation.md). Root trust,
worker epoch, durable replay/common fence, authenticated original start and
all-writer cessation still precede server/pilot/qualification admission.
