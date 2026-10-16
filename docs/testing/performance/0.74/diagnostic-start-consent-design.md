# Explicit first cell start consent

A causal Heartbeat is a response to a fresh post-Reserved challenge, not consent
to execute. The distinct start-consent artifact signs an explicit start-cell-one
claim. Current implementation provides signing bytes and byte/signature
inspection only; it neither derives the claims nor authenticates their original
runtime context. The [preregistration](diagnostic-start-consent-plan-20261016.md)
and [local evidence](local-runs/start-consent-20261016/README.md) describe that scope.

## Format and signature

StartConsentBody has eighteen required top-level fields; SignedStartConsent
wraps it with envelope schema and lowercase 128-character signature hex. Unknown
and duplicate fields at any nested level, missing/null required values, wrong
schemas, noncanonical/partial LF JSON and oversized input refuse. The body and
envelope ceilings are 4096 and 8192 bytes. All signatures cover
hydracache-diagnostic-collector-start-consent-074-v1, one NUL, then the complete
canonical sorted-key body JSON plus LF. Old request, collector-intent and
challenge domains remain unchanged and cannot substitute.

The action is exactly start-cell-one. The Starting projection has revision two,
zero completed cells, no process/cgroup or outcome, cleanup false and both
admission flags false. Its positive ordered boottimes satisfy the existing
controller/total budget grammar. The positive Unix signing interval is at most
sixty seconds; a future live check must require issued <= observed < expires.
Current inspection validates interval shape only, never actual freshness.

Source/preset remain the diagnostic worker artifact's existing SOURCE_COMMIT and
PRESET, not the current tooling commit. A positive worker-policy epoch and valid
hash grammar are required. None of these signed strings selects trust or proves
that a referenced object exists.

## Required comparisons by the future original owner

The consuming implementation must derive these values from retained original
objects, not accept matching caller assertions. Digest conventions follow the
existing collector and causal protocols:

| Field | Original bytes or value required by the future owner |
| --- | --- |
| consent_request_id | New unique first-cell consent id; compare against original ids and durable replay state |
| reservation_request_id | Original Reserve request id |
| controller_attempt_sha256 | SHA256 of complete retained canonical LF controller-attempt record |
| collector_reserved_sha256 | SHA256 of complete retained canonical LF collector Reserved record |
| reserved_state_sha256 | SHA256 of canonical original Reserved state JSON without LF, matching the causal commitment |
| challenge_nonce_sha256 | Existing collector_challenge_nonce over the original complete challenge; this includes its separate domain and NUL |
| authenticated_response_sha256 | SHA256 of the exact original received immutable KernelPacket bytes, without normalization or reconstruction |
| worker_policy_sha256 and worker_policy_epoch | Original checked worker owner's pinned policy digest and current admitted epoch; not an envelope-selected policy |
| collector_unit_sha256 | SHA256 of existing collector_unit_projection for the original Reserved state, including its canonical LF |
| controller | Exact original configured repository, run, attempt and actor |
| starting_state | First Starting projection derived/frozen under the same kernel receiver, authority and fence, then checked as an adjacent original transition |
| issued_at_unix_seconds and expires_at_unix_seconds | Bounds checked against original live kernel context, preserving all enclosing admission windows |

These comparisons are not implemented by the standalone signature inspector.
It accepts a public key parameter as a cryptographic primitive and returns
ordinary clonable/serializable data. It cannot enroll that key, attest kernel
credentials/time, discharge bootstrap predicates, acknowledge a state, prevent
replay, create output or launch/release a process. Repeated inspection and a
syntactically valid historical window intentionally succeed.

The future receiver must bind consent to its original kernel-message context,
then keep all current source/policy/epoch/fence checks around one consuming
handoff. Durable intent and uncertain/lost-ack replay protection precede any
root-private output or authenticated original process/streams/cgroup start.
Autonomous watchdog and revocation/cessation of every writer precede completion
and later cells. No installed-host or qualification admission follows here.
