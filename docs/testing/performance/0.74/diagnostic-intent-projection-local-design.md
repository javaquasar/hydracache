# Signed diagnostic intent projection: local byte contract

Preregistered against `1b45dc2b9ee2df21ecd244361b0ec471313f9516`.
This is a prerequisite for authenticated production intent, not its completion.
There is no filesystem, IPC listener, issuer enrollment, start API or backend.
The existing Reserve/Heartbeat/Status/Cancel protocol is unchanged.

The new Linux-only worker-policy child accepts a separately configured controller
key and sorted unique nonroot client/group and nonzero actor allowlists (1..32).
Its separate schema/domain cannot reuse a worker-policy or campaign signature.
Body and envelope are strict canonical sorted-key JSON plus LF, bounded at
4096/8192 bytes, rejecting duplicate/unknown fields. The signature covers the
domain, NUL and canonical body including request/nonce hashes, controller IDs,
an inclusive-issued/exclusive-expiry positive window of at most 60 seconds,
full reserved-state digest, original policy digest/epoch, boot and unit digest.

The unverified reserved projection is derived only through the existing state
validator and fixed diagnostic unit builder. Starting and recovery states refuse;
the asserted observation must precede the ten-second controller-loss deadline
and leave a positive whole-second total budget. Unit hashing preserves ordered
properties, explicit variant tags, environment/argv order and command fields.
It hashes expected configuration, not effective runtime or retained file identity.

Construction rejects already-refused workers first; validates controller config,
peer assertion, canonical document, signature and time before revalidating the
original worker. Bad controller packets do not revoke a healthy worker. Binding
requires the asserted state boot to equal the original worker host boot, exact
policy pins, and the freshly derived state/unit digests. Successful construction
exclusively borrows that original worker and retains the exact envelope, state,
controller configuration and peer. There is no Clone, serialization, extractor,
reset, refresh or conversion to execution authority on the checked guard.

Revalidation checks the sticky latch first, original controller configuration and
peer/state next, then authenticates the exact retained envelope and revalidates
the same worker. Any failure latches the intent. Worker revalidation failures also
latch the original worker through its existing API. Controller-only failures do
not revoke an unrelated worker policy. Restoring inputs cannot revive the intent;
dropping it does not undo a worker refusal. Independent requests remain possible
because this is not a durable replay ledger or a one-start capability.

Tests will cover all four cells, canonical/schema/size/signature/principal/time
refusals, signed field mutations, trust/state/peer/boot drift, original-worker
sticky failures, unit variant/order sensitivity, independent concurrent checks
and API compile-fail checks. All keys and contexts are synthetic. No host,
qualification input or release admission is changed; no performance claim is made.

Remaining: fresh authenticated kernel peers/clocks, durable fenced reserved state
and signed intent with replay/crash/reboot rules, original material/output/runtime
composition, production root preparation, watchdog and exact owned cleanup,
then separately approved enrollment. A valid digest is not launch permission.
