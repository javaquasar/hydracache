# Secure observer and memory diagnostics (local only)

Preregistered against clean `95aef30130c5c06344cc228dc4e2a5bf2120ec7c`.
The unchanged observer baseline passed all 57 tests serially before this work.
This is instrumentation, not a new product candidate or a retry of sealed B0.

RESP must use the actual runtime mTLS factory and production accept loop. Each
socket authenticates before HELLO/preload; no plaintext fallback or certificate
subject-to-tenant mapping is introduced. Independent HC2/RESP stores may share
ephemeral PKI. Receipts compare SHA-256 certificate fingerprints only, never
private keys, tokens or raw certificates. Such equality proves fixture transport
material, not identical application authorization, negotiated cipher, batch
atomicity, tenant mapping or performance. Those differences remain admission
blockers for a numeric cross-surface comparison. Shutdown must join the listener
and observe zero active production connections.

The allocation-only executable is separate from the unprofiled library/test
executables. It delegates unchanged layouts to System, uses one current-thread
runtime and measures whole-process successful allocation calls/gross requested
bytes/outstanding requested layouts. Actor, scheduler and transport allocations
are included; they are not server-only allocations/op. Windows process working
set or Linux VmRSS includes code/runtime/TLS and is not heap resident memory.
Product logical entry/value-byte diagnostics are reported separately. Missing
allocator active/resident/retained remains unsupported, never zero or green.

Before any measurements, commit implementation and retain the exact source,
tool lock and binary hashes in an execution seal. Execute each finite cell in a
fresh process, three repeats, unchanged phases/bounds. Stop on the first failure;
retain it and do not rerun a failed cohort into a passing receipt. No A/B speedup
or native nonregression claim follows from this diagnostic series. The existing
qualification manifest, old contracts and sealed packets remain immutable.

Implementation uses `RespControl::start_mtls` and `NativeControl::start_hc2_mtls`
with shared certificate material. TLS AUTH failures are redacted; a virtual-time
test retains the original five-second bound. The FIFO actor now explicitly owns
write-then-flush completion for buffered transports, retaining read progress and
cancelled owners. This corrects instrumentation, not product output batching.
The first dirty-source secure run passed three tests but failed large-payload
preload. The missing final TLS flush was isolated with a BufWriter/one-byte
duplex regression; after the actor fix the same large-payload case passed without
changing payload, seed, limits, deadlines or assertions. That initial failure is
not discarded or counted as a passing numerical attempt.

`memory-diagnostics-074` is enabled only by `allocation-diagnostics`. Its two
arguments are a preregistered surface and payload size. The 128-workload-call
count applies to GET/SET only; preload/delete/refill use the 16-key dataset,
plus transport-specific read verification. Idle/shutdown have zero workload
calls. Preload includes ephemeral PKI, runtime/store and transport creation.
These phase allocation totals are not comparable per-op server costs. JSON and
RSS queries are outside counting epochs. The 60-second safety bound is an async
deadline, not an OS kill guarantee against blocking code; the external runner
must also bound process ownership and stop the finite cohort on first failure.
