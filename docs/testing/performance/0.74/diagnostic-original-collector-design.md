# Collector requirements under the original controller owner

The [original collector preflight](diagnostic-original-collector-plan-20261016.md)
checks a collector signature using the acknowledged original request's retained
kernel sender, receiver clock, controller configuration and worker policy owner.
It removes caller peer/time/trust arguments from this composition, but does not
prepare outputs or prove an authenticated durable Reserved/Starting state.
The [contract](diagnostic-original-collector-contract.toml) keeps those boundaries
closed before the root constructor and original-start producer.

## Ownership and observed inputs

ControllerRequestAttempt::bind_collector_requirements exclusively borrows the
acknowledged original attempt. It neither moves out nor replaces the controller,
OIDC/build chain, worker, authority owner, kernel packet, storage or common fence.
There is no second CheckedWorkerPolicy and no second journal opener. Repeated
healthy read-only preflights are possible; this is not once-only start admission.

The private kernel snapshot revalidates the original live sender, samples the
original receiver's clock, checks the unchanged packet hold and revalidates the
sender again. Peer groups are the canonical union of retained supplementary
groups and per-message primary GID. No group is truncated to fit an allowlist;
an oversized union is refused by the existing bounded signature verifier.
Snapshots are private intermediate facts, not reusable public credentials.

The original kernel worker brackets the private collector byte check with its
existing policy/journal/host/packet observations. All IntentCheck inputs except
the signed state come from retained owners. The controller key/allowlists come
from the same original source used to authenticate the Reserve request. No
caller can substitute a peer, timestamp, host, issuer, digest or authority epoch
through the public collector preflight API.

The outer attempt revalidates before and after the check. Body request, nonce,
controller IDs and complete validity window must equal the original request;
the state's complete identity must match it too. Only initial revision one,
cell zero Reserved projections are accepted. The collector domain, output
digest, canonical grammar and worker pins remain checked by the separate
[collector byte verifier](diagnostic-collector-intent-design.md).

## Refusal and retained state

Any first error refuses controller, kernel, worker, authority and original
attempt storage without appending a durable refusal. Even an already-refused
entry propagates the existing latch without additional I/O. Later validation
returns Refused before reading named records or parsing replacement input.
Restoring fixture paths cannot heal the owner. Dropping the wrapper does not
release the fence: the original attempt still owns it until its own drop.

Signed bytes and state are copied only after successful bounded verification.
The guard accepts no replacement on revalidation, exports no underlying owners
or descriptors, and cannot clone, start, prepare root outputs or publish state.
Existing partial/complete request bytes still block fresh empty-fence admission.
There is no new durable/wire format or actual write boundary in this slice.

This composition deliberately treats malformed collector input more strictly
than the standalone byte verifier: the owned original attempt is permanently
refused, not left eligible for a different candidate. Its in-memory latch is
not reboot revocation, a global nonce ledger or snapshot rollback resistance.

## Scope before root preparation

The supplied Reserved state is signed under the original controller, not loaded
through a durable original Reserved/Starting owner. Its monotonic state fields
remain projection data; they are not minted as kernel observations. Production
issuer/job/host enrollment is also unchanged. Therefore the preflight is not a
root-construction or execution permit. The next join must retain the appropriate
owned lifecycle state and trusted root ancestry before any exclusive output
creation. Original executable/environment/pipe/process/manager/cgroup binding,
autonomous watchdog and all-writer cessation remain necessary before completion
or later cells. Historical Kernel(Invalid) is still unexplained.

Historical static guard comparisons exclude only the exact new kernel and
request-attempt module registrations. All pre-existing predicates, observation
order, latches, format readers and budgets continue to compare to their fixed
baselines, with changed-guard/deadline canaries intact. The new read-only modules
have a separate contract gate. This is scoped source consistency, not formal
whole-program equivalence, signed execution evidence or a performance result.

## Observed initial state under the same owner

The separate [observed reservation path](diagnostic-observed-reservation-plan-20261016.md)
consumes ControllerRequestAttempt into OriginalObservedReservation. Transfer
itself performs no I/O and grants no permission. The first successful observation
checks the retained request/storage and original worker/kernel authority, samples
the receiver's retained origin and constructs initial Reserved from its boot and
CLOCK_BOOTTIME. Revision is one, completed cells zero, and all three initial
clock fields share that one sample. The state is frozen in memory; later
observations update only a private clock-order checkpoint, not lease origin,
state revision or controller progress. Packet hold is always measured from the
original receive point, and the original request window still applies.

state_to_sign returns a checked clone for the existing collector signing format.
Changing that data cannot change the owner. The distinct ObservedCollectorPreflight
accepts signed bytes only, checks them against the frozen state through the
existing original collector verifier, and revalidates without replacement input.
Neither a supplied-state guard nor a decoded record can construct this guard.
All errors retain the same fence and permanently refuse the original chains;
subsequent refused calls do no I/O. Healthy wrapper drop retains its owning
reservation and cannot expose the consumed request again.

These observations establish an in-memory origin, not durable Reserved/Starting
acknowledgement. The next publication protocol must consume this original owner,
retain its original request document and fence, and exclusively create a distinct
versioned record. The old one-document attempt inventory and legacy publication
transfer must not be relaxed for general adoption or reinterpreted as collector
authority. Its new write boundaries need a preregistered real SIGKILL/lost-ack
matrix before root output creation. Production enrollment and execution remain
closed; no existing signed or durable format changes here.

The subsequent [collector Reserved publisher](diagnostic-collector-reserved-design.md)
now provides that separate consuming publication with a new COMPAT-registered
record. This observed owner and its read-only wrapper alone still cannot
acknowledge durable state. The publisher retains the original request/fence in
its own exact two-document engine; Starting and root output construction remain
separate unimplemented consumers rather than conversions from supplied data.
