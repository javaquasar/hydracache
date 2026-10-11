# Original authority and signed intent composition

This prerequisite binds an expected signed intent to the original runtime
authority chain without creating a second worker policy guard. Baseline is
`f96912725f8d7fa4b3501f548d5bd0d518d5a483`. It does not implement fixed intent
publication, host fencing, live state/peer/clock admission or worker execution.

The existing checked intent exclusively borrows CheckedWorkerPolicy. The original
signed tasks/authority chain already holds that same worker; making an independent
checked copy would not authenticate the original runtime objects. Instead, a new
fixed/fixture pair exclusively borrows the matching original authority guard.
The shared byte verifier checks signatures and immutable projections against the
worker already held inside that guard; its pure internal pin result is not public
authority. No guard, runtime object or fixture origin can be cloned/promoted.

Authenticate and bind untrusted initial bytes before creating the new wrapper.
A malformed, wrong-signature or mismatched unbound intent cannot revoke a healthy
worker, preserving the existing projection contract. Then check original
authority, signed intent and original authority. The authority observation retains
the existing journal/signed-runtime/journal bracket. Capture exact envelope hash,
Reserved state, controller configuration, peer and signed controller identity;
subsequent observations cannot replace them or renew the envelope with a new
signature. Asserted clock advances only within the original signature window.

A failure in the already bound projection latches the original authority/runtime
chain and publishes Policy refusal only to the still safe, independently exact-bound
original authority journal. Unsafe or advanced storage is never repaired or
appended. Refusal publication failure preserves the original projection error
plus typed uncertainty. Existing authority failures retain their existing error
and refusal behavior. Later calls refuse before further byte, kernel or journal
I/O, even after restoration or wrapper drop. Successful drop leaves inputs healthy.

No new wire/durable schema, CLI, IPC, installer, key pin, epoch or request mode is
added. The fixed publisher still needs original trusted storage, supervisor-owned
fence and persisted Reserved state; the worker-owned campaign lock cannot become
trusted root authority by passing a path. Output/start/watchdog/owned cleanup and
real operator issuer/initial-host admission remain before a worker pilot. Existing
frozen qualification inputs and their plan-digest refusal remain unchanged.

Local tests use synthetic issuer/controller keys, owned kernel task helpers and
nonroot authority journals. Cover valid repeated observation/healthy drop; invalid
unbound signature/state; bound envelope/state/controller/peer/expiry drift; original
worker failure; advanced or replaced authority; partial/late refusal publication;
first-error retention and no I/O after refusal; independent concurrent chains;
fixed/fixture API distinction, exclusive borrowing and compile-fail non-promotion.
Retain baseline and absent API red before implementation, repeated scoped checks,
lint/check/MSRV/docs and genuine qualification/ship negatives. These local results
do not attest a production root, real issuer or continuous kernel confinement.
