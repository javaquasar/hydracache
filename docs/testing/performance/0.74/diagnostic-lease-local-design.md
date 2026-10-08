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

## Implemented local slice

`diagnostic_lease.rs` now owns the portable reservation, canonical bounded
schema-1 marker with state digest, crash-pending refusal, monotonic heartbeat and
four-cell state machine. `host_execution.rs` checks this marker in acquisition,
recovery and absence observation. A valid diagnostic-only marker returns no
active *campaign* to campaign maintenance, not a swallowed lock error. Other
Busy/corruption behavior is unchanged. Reservations expire at the exact
preregistered boundary; a late heartbeat cannot revive the controller lease.

Cell intent fixes source, config hash, binary identity and deterministic unit
name. Runtime is at most 60 seconds and capped by the remaining total budget at
start. Overflow and invalid receipt stop the sequence; cleanup failure or
changed cgroup inode never frees the host. Terminal archival is durable before
marker removal, same-id reuse is refused, and recovery after the archive/marker
crash gap reuses byte-identical terminal evidence without a second spawn.
This terminal file is a local state receipt, **not** a sealed raw measurement
packet. Raw spool retention is an explicit backend obligation, simulated in tests.

`diagnostic_unit.rs` builds Linux property data only: exact preset argv, CPU 1,
control-group kill, no restart/delegation, runtime and stop ceilings, fixed spool
paths, 8-MiB per-file size, memory/task/FD ceilings and confinement. It does not
invoke DBus or verify deployed systemd support. Its diagnostic unit namespace
is intentionally not enrolled in the existing production unit dispatcher.
Loopback-only network policy, authoritative unit/process/cgroup observation,
bounded DBus calls, read-only build/config verification and raw receipt parsing
remain requirements of the future live backend, not properties proved by this
constructor. No partial file or exited leader is treated as empty-cgroup proof.

The portable tests use logical clocks and a mock backend, including eight
concurrent campaign/diagnostic admission races. Linux adds linked/pending path
refusal, fixed property construction and an actual supervisor socket fixture
whose three maintenance methods skip a diagnostic-only lease with zero backend
calls. No test needs a product workload, live systemd unit or rented host.

Development failures are retained as negative preparation observations: the
pre-implementation target failed with a missing module; the first deadline
test tried a late heartbeat and failed correctly; the first Linux compile
found a missing exhaustive error-code arm for `DiagnosticConflict` (now the
existing conflict code 5). One Linux invocation used a shell without Cargo in
PATH and did not execute tests; the corrected login-shell command pins 1.94.0.
No numerical attempt, workload, CPU floor or qualification threshold changed.

## Exact-source evidence and remaining boundary

Implementation `f4f68ad734bcfb454f4a5531c68d3af5fc25a7d5` was clean before and
after the repeat captured in `local-runs/diagnostic-lease-f4f68ad7/manifest.json`.
Windows passed 18 diagnostic and five existing host-execution checks. Local
WSL/Ubuntu on Rust 1.94.0 passed 20 diagnostic, six existing host-execution and
19 server checks. Raw log hashes and sizes are independently guarded; command
output was captured by PowerShell, not asserted to be native Linux stdout bytes.

Both OS package all-target checks/strict clippy and formatting passed; the root
passed 67 performance-contract and 13 release-evidence tests. Doc-check, local
performance-contract, 17 governance checks, documentation links and mdbook were
green. Ship admission is expected-red for unresolved C74 and qualification.
These are focused gates, not full workspace/release verification or numerical
observations. All 0.73/frozen qualification files and historical packets remain
untouched.

Still required locally: authenticated diagnostic IPC with replay/nonce/revision
fencing, the bounded live backend, independent binary/config/build-receipt
verification, strict timing-receipt parsing and raw packet sealing, recursive
cgroup/proc identity checks, bounded DBus operations and empty-tree observations,
plus non-product controller/supervisor-loss fixtures. A manually advanced model
does not itself schedule a watchdog or autonomously kill a real workload.
The property builder's runtime/stop ceilings likewise are not a rehearsed hard
tree deadline. A later deployment needs separate permission and renewed exact
provisioning/lifecycle/overhead evidence; old installed receipts cannot certify
this changed supervisor. Product candidates and qualification remain closed.
