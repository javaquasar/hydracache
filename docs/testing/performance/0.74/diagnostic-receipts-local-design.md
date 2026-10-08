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
