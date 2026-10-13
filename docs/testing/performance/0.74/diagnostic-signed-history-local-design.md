# Signed diagnostic history inspection

Baseline is `8549010a649a4a7394071a3dbaff67c794fc6a31`. This P1 slice
checks bounded supplied bytes for cells one through four. It does not publish
state, consume another intent or authenticate cell completion. The
[intermediate plan](diagnostic-server-readiness-intermediate-plan-20261012.md)
continues to govern fixed lifecycle and server admission.

## Preregistered checks

Accept an original early-attempt record and at most four contiguous record and
commit byte pairs, using the existing canonical retained-intent schema. Check
budgets before decoding. Authenticate every signed Reserved projection against
the original checked controller, peer and worker policy; revalidate the current
original intent before and after inspection. Historical verification uses the
signed issue time only to verify that historical signature, never to renew it.
Reject a historical issue time after the asserted current time.

The anchor must describe revision one, zero completed cells, a valid Reserved
state and no previous hash. Cell one must equal the anchor bytes exactly.
Every subsequent record binds the previous complete raw record hash, the same
lease, boot, binary, provenance and reservation origin, and its exact cell index.
Reject duplicate retained request IDs or nonces, gaps, partial/wrong commits,
unknown schemas, noncanonical bytes and invalid signatures or state projections.

Require nondecreasing observed/controller clocks and at least three revisions
between successive Reserved cell projections: Reserved to Starting to Running
to the next Reserved. Additional heartbeat revisions are allowed. Apply the
same necessary lower bound to the supplied current Reserved state. An empty
history requires that current state equal the anchor; otherwise the prefix must
end at the current cell or immediately before it. This checks plausibility, not
the existence of omitted transitions, successful outcomes or writer cessation.
Do not use revision counts as a substitute for a retained transition trace.

Return cloneable observation data only: retained pair count and anchor/last
hashes. No storage reader, recovered acknowledgement, mutable lifecycle guard,
execution conversion or permission to append is returned. Any inspection error
latches the supplied original intent guard; it does not claim durable refusal
across process restart. No filesystem or process operation belongs in this API.
Existing fixture and fixed publishers remain unchanged.

## Preregistered local proof

Capture existing journal regression as baseline and a missing-entry red test.
Cover empty and all four prefixes, additional heartbeat revisions, exact anchor,
hash links, pair/count budgets, schema/canonical encoding, fully re-signed state
and origin drift, duplicate IDs/nonces, revision gaps, clock rollback, current
state mismatch, original input drift, future historical issue time and expired
current intent. A compile-fail test prohibits start conversion. Repeat exact
source tests three times, including a seeded matrix of legal chains and signed
mutations. Run Linux/Windows regressions, current/MSRV checks, warnings-denied
Clippy, formatting, Rustdoc and documentation checks. Preserve raw failures and
successes with source hashes and a read-only evidence verifier.

## Closed boundaries

No new durable format, filename or migration is introduced. Supplied byte
inspection does not establish inode ownership, a retained host fence, actual
peer credentials, a trusted clock, production issuer pins, global nonce
uniqueness or protection against whole-host snapshot rollback. Fixed cells two
through four still cannot execute or acknowledge completion. P1 remains open;
P2 through P5, product qualification and ship gates are unchanged.
