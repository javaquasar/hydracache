# Original worker policy and kernel packet context binding

Baseline is `aa7822798b57a9aa211f561d432db803edcefee1`. This P2 step
connects the [fixed trust source](diagnostic-worker-trust-input-design.md) to
actual original Linux context observations. It does not create or sign a real
policy, choose an authority epoch, initialize a journal or enable a server route.

## Runtime ownership and verification

Add a separate `verify_kernel_policy` entry on WorkerTrustInput. It takes only
signed policy bytes, the original mutable KernelPacket and the original mutable
FixedAuthorityRegistry; no caller host assertion. Extract the candidate context
from bounded supplied bytes, then verify it under the existing independently
configured issuer/body pin/epoch and journal bracket. Parsing candidate fields
is not trust selection: only the pinned signature verifier accepts the policy.

Open the existing fixed machine/boot/user/mount Probe under original kernel
packet checks. Compare receiver boot and retained typed user/mount nsfs handles
to the verified policy. Original PID/time namespaces and packet age remain
checked by the existing kernel origin and exclusive 60-second hold bound; they
are not new externally selected policy fields. Retain actual machine/boot files,
reading thread, original namespaces and packet sender pidfd/credentials.

The returned original-thread guard owns WorkerPolicyInput and Probe and borrows
the same packet mutably. Revalidation accepts no replacement arguments and
checks policy/journal, packet, Probe, packet, policy/journal. Constructor and
observation errors permanently latch policy, config, authority and packet;
successful drop leaves original owners healthy. No raw policy/trust/context/FD
extractor, serialization, fixture conversion, controller authorization, replay
admission, start or completion method is added. Named reopening only compares
original objects and never refreshes identity.

## Real production inputs and later joins

Real policy selection needs current machine/boot/ns identities, exact local
passwd/group hashes, actual worker UID/GID/group mapping and an independently
approved nonzero epoch. Read the original authority history before selecting a
transition. First enrollment in an empty provisioned journal is an explicit
operator transaction, not default epoch 1, hidden bootstrap inside verification
or reopening an incomplete attempt. Verify policy and host context before that
transaction; the normal read-only join requires an already active journal.

Do not sign fixture values with the actual local worker issuer. Keep private
keys outside Git and telemetry. Historical host receipts, a WSL boot, observed
root UID and namespace equality are not physical-server enrollment. The fixed
source still must be opened under the admitted production namespace/custody
boundary; this binding does not retrospectively attest its opener.

After this connection, independently load controller principals, verify signed
controller provenance, and consume original request/nonce admission under the
same common host fence and early-attempt acknowledgement. Existing portable
request authentication and recovered journal data cannot supply replay authority.
Authenticated collector original-start, watchdog and whole-writer revocation
follow those producers; completion/cells two through four follow writer cessation.

## Preregistered local checks

Retain worker trust, context and kernel baselines, then a missing-API red test.
Use nonroot private modeled config/journal with synthetic issuer and actual local
machine/boot/ns observations and SOCK_SEQPACKET kernel messages. Test unchanged
originals, malformed/wrong signed context, wrong packet boot/user/mount, wrong
namespace type, PID/time drift, exclusive packet hold, sender/credential refusal,
config/journal/policy failure, original cross-refusal and healthy drop. No local
model is a positive actual-root or real-server enrollment.

Add seeded context mutations, independent parallel bindings and compile-fail
no-extraction/start/thread/replacement/borrow-refresh checks. Run three focused
repeats, context/trust/kernel regression, doctests, current/MSRV checks, strict
current Clippy, format, root contract/evidence checks and documentation/local
performance checks. Keep negatives, raw logs and source hashes. No workload,
qualification input, signing domain, durable schema or provisioning change.
