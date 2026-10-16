# Causal challenge before the Starting handoff

Baseline 7dff8a6bb04f3786016e699020c87d5ee8889c4e. The existing progress
preflight authenticates one received heartbeat but cannot distinguish a queued,
previously signed message from a response to the acknowledged Reserved state.
Close that gap without changing its guards, renewing admission or opening start.

Consume the acknowledged original collector reservation into one challenge owner.
After unchanged original checks, obtain 32 bytes from the existing ring system
CSPRNG, then sample the original kernel clock. Encode a separate bounded canonical
LF challenge containing random hex, original controller-attempt and Reserved
record hashes, frozen state hash, boot and receiver-observed issuance BOOTTIME,
and revision 1. Its domain-separated SHA256 is the nonce_sha256 required in the
existing signed DiagnosticRequest v1 Heartbeat. Register only the additive
challenge format/domain in COMPAT; never reinterpret the old request signature.

Retain the same authority/fence and immutable challenge. Byte copies can compute
the nonce but cannot reconstruct the owner or prove acknowledgement. No caller
nonce, clock, peer, path, descriptor or RNG input is accepted publicly. Entropy
failure refuses without fallback or retry. Subsequent exports never regenerate
randomness or replace issuance time. Enforce an exclusive ten-second challenge
age using fresh original kernel time; all old 60-second holds and signed/OIDC
windows remain unchanged.

Consume the challenge and one kernel packet into a distinct causal preflight.
Move the same reservation into the existing progress guard with the retained
issuance sample as its fixed receive-order boundary. Run that unchanged guard,
then require the authenticated immutable packet's nonce to equal the complete
challenge digest. Check the challenge age again after original-chain checks.
Failure retains the original owner/fence and packet, cross-refuses and cannot
retry. The result cannot accept another packet, renew, deserialize, extract or
publish Starting. This proves a response bound to a post-ack challenge commitment,
not that a controller downloaded or reviewed every Reserved byte, continuous
liveness, durable replay history or execution authority.

Capture a targeted existing baseline and API compiler red first. Test canonical
schema/size/domain/hash grammar, seeded field substitutions, actual entropy,
unchanged signing-copy exports and files, held fence, fresh signed response,
a genuinely prequeued signed heartbeat dequeued after challenge issuance,
entropy failure and sticky failure after owned-object drift. Use private injected
RNG/clock arithmetic only for explicit test seams; distinguish them from live
kernel observations. Check consuming/no-start/no-replacement API constraints.

Run current/MSRV focused tests, kernel/projection/API regressions, default-parallel
original-input regression on the current compiler, affected check/strict Clippy,
static/format/doc/registry/local contract and expected-closed ship checks. Keep
real-clock tests separate from builds and whole-workspace formatting; preserve
all negative results without loosening budgets or substituting serialized success
for the default-parallel regression. Preserve earlier evidence packets unchanged.

This is a prerequisite to the separate consuming lifecycle authority handoff.
That protocol must explicitly discharge admission predicates while preserving
live configuration/epoch/context, challenge/progress and deadline obligations;
existing guards must not simply stop checking expired inputs. Durable Starting,
new write/SIGKILL boundaries, root output, authenticated original start, watchdog,
writer cessation, production enrollment and server/qualification admission remain
closed. No product/workflow/server operations or push in this slice.
