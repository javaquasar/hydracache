# Local diagnostic recovery refusal

An incomplete diagnostic intent must not become permission to replay execution.
This local slice composes existing schema-1 diagnostic state and IPC journals
with the original fixture signed-task authority bridge. Baseline is
`fcd4d011806d728ffa1d332a4f313ec81e4fda35`, tree
`686d35a822d5afe26016db308bfb60d40465c014`. The
[contract](diagnostic-fixture-recovery-local-contract.toml) precedes implementation.

## Inspection and refusal rules

Inspection retains the existing cooperative fixture host lock and root objects.
It performs bounded, descriptor-relative, no-follow reads of the two fixed
documents, checks private ownership/mode and one link, and checks their original
objects and bytes again. Existing decoders enforce canonical bytes, schema,
hashes, state invariants and IPC entry uniqueness. No format is migrated.

Either pending publication name, an IPC intent without a response, or a valid
Starting, Running or Stopping state refuses recovery. Missing state, wrong
asserted identity, foreign boot, corrupt documents, conflicting campaign/context
and unsafe metadata also fail explicitly. A valid Reserved or Terminal state
with no pending IPC transaction is only a quiet observation, not permission to
start, adopt a live process, or release retained output. IPC hashes are not
production signature verification or authentication of worker authority.

Only the fixture-origin bridge exposes this check. It first revalidates its
original signed chain and authority under the original pins, then checks the
expected lifecycle boot against its already retained original host. Inspection
holds the cooperative fence while the bridge handles the observation. A lifecycle
failure latches the entire original chain and appends an existing Policy refusal
only after fresh safe exact-bound authority inspection. The first lifecycle
error survives; an unacknowledged append also reports publication uncertainty.
Failed later calls do no I/O. Unsafe or foreign authority storage is never
repaired or appended. The helper does not alter lifecycle documents or outputs.

## Local verification and remaining boundary

Record unchanged journal, signed guard, diagnostic state and IPC tests first,
then a missing API test before implementation. Cover all lifecycle stages,
pending files (including dangling links), unacknowledged IPC entries, canonical
and schema failures, identity and boot mismatch, budgets, links, FIFOs, unsafe
modes, competing locks, document/root/lock replacement, retained bytes, durable
refusal/reopen and each existing logical authority publication fault. No backend,
systemd, process start/stop or production key is available to the new route.

This is fixture-only refusal composition, not authenticated production start
intent. It does not close a crash before any durable intent exists, enroll a
fixed root path, prove physical power-loss durability, or handle whole-host
snapshot rollback. Controlled root enrollment and a separately reviewed durable
production intent protocol remain required. Frozen qualification inputs,
manifest, thresholds and 0.73 artifacts remain unchanged.
