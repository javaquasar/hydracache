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

Local tests must cover signature tampering/cross-domain/time bounds, malformed
packets, peer/principal denial, cached replay, nonce conflicts, revision races,
controller drift, intent-before-mutation crash gaps, ledger limits/corruption and
a temporary Linux SOCK_SEQPACKET fixture. The handler uses the existing mock
backend; no real unit is created or stopped. Production main/config/server
dispatch remains unchanged. Build trust, real recursive cgroup cleanup,
autonomous watchdog scheduling, raw receipt validation/sealing and separately
approved deployment remain later requirements.
