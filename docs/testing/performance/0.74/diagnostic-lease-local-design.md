# Local diagnostic lease and bounded coordinator

The human approved local implementation after `8a1f4ab3`, not installation or
numerical execution. This is a new diagnostic evidence family; existing P0
inputs, seals and qualification remain immutable. The installed supervisor is
unchanged and still has the external-lock hazard documented in
`rental-pilot-coordinator-design.md`.

The local implementation uses the existing host lock only during a transaction.
A separate versioned `active-diagnostic.json` records a baseline-only lease;
campaign acquisition/recovery must refuse it, while campaign maintenance may
skip a validated diagnostic-only marker without keeping the lock held. Malformed
or simultaneous campaign/diagnostic markers fail loud. Both known lifecycle
fixture contexts are checked under the same transaction before reservation.

The coordinator persists an intent before any start, permits exactly the fixed
embedded/direct/RESP2/RESP3 order, and recovers a starting intent by observing or
stopping its deterministic unit, never by spawning it again. The boot identity
and monotonic clock bind controller liveness, a 60-second cell limit and
300-second total limit. Receipt overflow, invalid output, controller loss and
deadlines stop the entire exact unit/cgroup. Failure to prove an empty owned
cgroup keeps the reservation. A durable terminal receipt precedes release;
failed attempts are retained and no retry is admitted.

Local tests supply a deterministic backend, not live systemd or trusted artifact
provenance. A fixed Linux transient-unit spec may be constructed as data, with
control-group kill, no restart/delegation, runtime/stop/file-size/resource bounds,
fixed binary/config argv and CPU 1. A process-group or exited leader is not
cleanup proof. A live backend must verify unit/cgroup and process-generation
identity, handle tree descendants and bound/retain raw receipt spools.

This first slice deliberately does not add production IPC/CLI operations,
signed provisioning, authentication/enrollment, a live cgroup backend or a
production maintenance call that drives diagnostics. Those require subsequent
local security/integration work before any separate host permission. The new
durable marker is registered in `docs/COMPAT.md`; an older supervisor cannot
be used concurrently with a diagnostic marker because it does not know that
reservation. Cleanup and empty-cgroup proof precede any rollback.
