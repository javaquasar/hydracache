# Local diagnostic authenticated request boundary

This slice follows the clean local lease model at `872788f4`. It is not a
production route, installed service, live backend, product pilot or numerical
qualification. The fixed P0 source, configurations, limits and historical
packets remain unchanged. No arbitrary command, path or workload is accepted.

A separate strict schema-1 request family signs the entire typed request under
`hydracache-diagnostic-request-074-v1`: operation, request id, single-use nonce,
expected revision, lease/boot/binary/build-receipt identity, fixed source/preset,
repository/run/attempt/actor and bounded authorization window. Campaign-domain
signatures cannot authorize diagnostics. Linux accepts peer identities obtained
from the existing kernel-credential transport, never from packet fields.
Portable tests use explicit peer identity inputs and do not prove Linux IPC.

Every operation is signed. A canonical bounded durable request ledger uses the
same short host fence as the lease state. Revision validation, request intent,
model mutation and completion publication occur under that fence, not a
check/unlock/mutate sequence. The original controller tuple and full lease
identity are fixed by the first reserve request; takeover is refused.

An exact completed retry returns its original receipt, without executing a
heartbeat or another mutation. Changed body with the same request id or reused
nonce is refused, including cross-lease reuse. An incomplete request intent is
uncertain: neither that request nor fresh requests may execute until a future
explicit reconciliation procedure proves the outcome. No automatic recovery,
eviction, retry or implicit lease extension is allowed. A pending file, malformed
or noncanonical ledger, future version, linked file or exhausted capacity fails
closed. The ledger is integrity checked, not tamper-proof against its owner.
Responses are unsigned local receipts, not independently authenticated remote
attestations or fresh status reports on replay. The 128-entry cap is global to
the host root; exhaustion intentionally stops admission. Archive/reconciliation
and key provisioning are not implemented by this slice. Direct coordinator
library calls remain a local test seam, not an authenticated public service.

Local tests must cover signature tampering/cross-domain/time bounds, malformed
packets, peer/principal denial, cached replay, nonce conflicts, revision races,
controller drift, intent-before-mutation crash gaps, ledger limits/corruption and
a temporary Linux SOCK_SEQPACKET fixture. The handler uses the existing mock
backend; no real unit is created or stopped. Production main/config/server
dispatch remains unchanged. Build trust, real recursive cgroup cleanup,
autonomous watchdog scheduling, raw receipt validation/sealing and separately
approved deployment remain later requirements.

## Implemented local boundary

`diagnostic_ipc.rs` implements signed reserve, heartbeat, status and cancel against
the local model, with a private canonical request ledger. Authentication precedes
lease/ledger filesystem access; revision validation, durable intent, state mutation and cached
completion share one host fence. Internal fenced coordinator methods avoid a
nested lock acquisition or a revision-check/unlock/mutate race. Production main,
server/config and campaign protocol are unchanged, as checked by the root guard.

Tests cover every signed field, campaign-domain refusal, exact expiry, malformed
or oversized packets, unknown fields, denied UID/group/principal, original actor
and run-attempt ownership, nonce conflicts, eight concurrent same-revision
requests, replay after terminal release and non-refreshing status/replay.
Injected loss inside tree stop retains the already durable intent and refuses
both replay and fresh requests; an explicit fixture models the heartbeat
mutation/response-publication crash gap. Corrupt, future, noncanonical, linked,
pending and capacity-exhausted journals fail closed. Backend error text never
enters the bounded response. Linux fixtures use real temporary sockets for both
kernel-peer acceptance and denial, with zero product starts.

The initial pre-implementation test failed with missing-module E0432. The first
strict Windows lint found three redundant borrows introduced by extracting the
fenced methods; these were corrected without changing any workload or threshold.
This is preparation/security evidence, not allocation attribution or performance
measurement. No live unit/cgroup, watchdog, production key enrollment, binary
build trust, raw workload packet or host admission follows from these tests.
