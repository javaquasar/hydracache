# Local raw diagnostic receipt verification

Preregistered on clean `fa4173b8`. This slice consumes bytes only: no process,
host, SSH, install, workload, product build or qualification. The source,
lockfiles, four raw P0 inputs and qualification manifest remain frozen.

The verifier takes an externally verified build capability, exact cell intent,
caller-supplied terminal summary, and original stdout/stderr bytes. It checks
identity before parsing. Each stream is capped at 8 MiB, combined at 16 MiB.
Overflow cannot be serialized as a complete retained packet; a future spool
backend must retain its bounded prefix and independent overflow/size receipt.
Bounded malformed or failed output remains hash-bound in a rejected packet.
Stderr is opaque retained diagnostic data, not parsed or assumed empty.

Stdout must be one complete newline-terminated JSON object. A recursive parser
rejects duplicate keys before typed strict mirrors reject unknown fields. No
repair, tail recovery, rebasing, retry or threshold change is allowed. The
existing observer envelope is mirrored without changing its producer. Its
source field is coordinator provenance, not independent Git proof. P0 plaintext
reports cannot claim security parity, allocation profiling or admission.

Successful-content validation requires the fixed input, typed workload digest,
10,000 samples and original 200,000-ns calendar. Counts, goodput, good fraction,
latency boundaries and overflow counts are reconstructed. Quantiles reproduce
the pinned HDR 7.6.0 three-significant-digit, lowest-value-1 projection:
clamp values to [1, highest], ceil(q * count) rank, upper equivalent bucket with
shift max(0, floor(log2(value)) - 10). This bounded specialization is not a
general HDR implementation. CPU provider/scope, derived ratios, ordered quality
reasons and unchanged one-second minima are reconciled. Pending high-water is
range-checked, not independently reconstructed from scheduler events.

RESP additionally checks dialect, topology, HELLO, deterministic connection
selection, unique offer IDs, per-connection increasing wire ordinal/response
time, timestamp boundaries and byte-oracle evidence for every success. Wire
records for failed operations need not exist, but cannot fabricate replies.
Failed report/process, malformed/inconsistent report, and valid but CPU-unusable
content are distinct outcomes. None permits numerical product claims.

A canonical schema-1 packet binds the build receipt digest, cell intent,
terminal summary, both raw stream hashes/sizes, recomputed decision and closed
admission flags. Offline verification recomputes the packet from trusted build,
expected intent/summary and original bytes; it never trusts a success field.
The packet is a byte envelope only, not filesystem sealing, durability,
authenticated process exit or live recursive cgroup cleanup proof. Filesystem
spool pinning, crash-safe no-overwrite publication, bounded live backend and
controller-loss fixtures are separate subsequent slices. No production caller
is added; synthetic positive fixtures are explicitly not real measurements.

## Implementation and negative preparation observations

`diagnostic_receipts.rs` provides strict recursive duplicate detection, typed
report mirrors, fixed-input/sample/CPU/HDR/wire reconciliation and canonical
packet replay. `CellIntent` gains strict serde solely for this packet; no field,
existing lease document or IPC schema changes. The build capability exposes
its checked identity read-only. No dependency or observer source changes.

The first compile failed E0277 because `CellIntent` had no serializer. After
adding it, an initial filtered command ran only one histogram unit test and
zero integration tests; it is not six-test evidence. The unfiltered integration
run passed five but failed one incorrect negative assertion: adding one ns to
CPU wall time can be valid. The test now uses wall time shorter than observation;
the verifier and floors were not weakened. A documentation patch context failed
without changing files. These are preparation failures, not product attempts.

## Clean-source repeat and remaining boundary

Implementation `ceaa8b07ffef714fe9b3662791941c565aa65e93` was clean before/after
[the captured repeat](local-runs/diagnostic-receipts-ceaa8b07/manifest.json).
Windows passed 62 checks, local Linux 96, root contract/evidence 83 and separate
governance 23. Eight new integration checks and three library checks passed on
both OSes. All-target check and strict all-feature clippy passed for supervisor
on both OSes and xtask on Windows. Scoped fmt, doc/local-contract, links/book
passed. Require-ship remained expected-red, exit 1: C74/qualification incomplete.
The packet retains three exact combined Tee captures and their SHA/Git blobs;
none is observer timing output. This is focused, not full workspace verification.

Floating projection reconciliation allows only four machine epsilons relative
to max(1, expected), for JSON round-trip precision, not performance regression
tolerance. HDR checks include pinned bucket edges and seeded rank/overflow
properties. The terminal summary and synthetic reports do not independently
prove a real CPU provider, event sequence, byte oracle or process lifecycle.
No product performance or allocations were measured. Old packets, observer
source, P0 files, lockfiles and qualification manifest remain unchanged.

Next: pinned bounded spool reads and crash-safe no-overwrite durable publication,
including overflow-prefix receipts; then live owned-tree/backend/watchdog and
non-product loss fixtures. Actual clean baseline build, real builder trust,
immutable install/start coordination and explicit uncertain-ledger reconciliation
remain separate prerequisites. This byte-only packet cannot release a host
reservation or enable installation, pilot, full D3 or release admission.

The later packet guard initially failed E0277: xtask uses SHA-256 0.11 whose
digest array lacks LowerHex. Per-byte hex encoding fixed that guard without a
dependency change. This post-capture preparation error does not alter the clean
implementation repeat or its raw logs; the packet guard is checked separately.
