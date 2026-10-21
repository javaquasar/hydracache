# Authority fd path ownership control

Baseline is `d90228c71288a4e45076fb12785cec1ba91ab089`. The preceding actual-phase
attribution isolates one temporary String for each `/proc/self/fd/{fd}` path.
This candidate changes only that ownership inside Directory::inventory. The
decimal formatter, fresh read_dir, observed BTreeSet, UTF-8 handling,
duplicate/budget/parity/canonical guards, error order and all metadata/read
checks remain. Original 300/60/10 windows and frozen qualification inputs stay.

Write the same prefix and i32 fd formatter into a 25-byte stack Cursor. The
14-byte prefix and at most 11 decimal bytes, including i32::MIN, fit exactly;
the output is ASCII. Borrow the formatted slice only for the same read_dir.
No path cache, syscall change, new encoder, observed-name optimization or
record-buffer reuse belongs in this candidate. Durable and API bytes stay equal.

Before runtime mutation freeze reference/proposal controls and recipes. Check
all decimal boundaries, i32 extrema and 4096 seeded descriptors (0x074fd025).
Compare real retained-directory scans, malformed names, budget/parity errors,
regular-file descriptors, fresh mutations and sticky refusal using the existing
filesystem controls. Exact source adapters must retain every historical
assertion, refuse unknown/duplicate substitutions and preserve unrelated drift.

Measure pure formatting for descriptors 0, 9, 10, 99, 100, 12 and both i32 extrema:
32 warmups, three alternating 1024-operation samples, owned reference versus
stack proposal. Runtime acceptance belongs to exact source canaries, filesystem
tests and the unchanged eight-case full-inspect control, not a pretend actual
formatting arm. GNU 1.94 and MSRV 1.88 each get one Before and one After capture
from committed inputs and hashed test images. Retain the Before packet before
implementation. Never replace or retry a failed measured cohort into healthy.

Full inspect at depths one/seven must save exactly two matching fd-path
formatting costs; both inventories construct that path. All six non-authority
cases, read/write observations and tracked kernel counts must remain equal.
Reference/proposal costs must repeat across samples, stages and compilers.
Reject and retain a negative result and revert only this candidate if semantic
or conservation checks fail. Rust requests do not establish CPU/latency,
retained-memory, copied-byte, full ActiveAuthority or cache API gains.

Run focused tests, affected all-target checks and strict all-feature lint per
compiler; historical/current source assertions and repository format/docs/
registry/local performance checks before final capture. Freeze non-overwrite
recipes first. Reconstruct observations with structural JSON comparison;
verify all fifteen prior immutable packets and seal this packet once after
review. Old source-specific ignored breakdown probes are not new-runtime
acceptance tests; their sealed observations remain bound to their old source.

Local nonroot synthetic controls only. No server runs, install, supervisor,
clock/service changes, new live matrix, product workload, qualification or push
through the red native gate. Original delivery, writer cessation, lifecycle,
SIGKILL/lost-ACK, reconnect and trust/enrollment gates remain independent.

## Local outcome

Before `3635e560599a4987bcc5f9652ec170d33323e6bd` is retained before mutation
in `89691d5e`. After/runtime is `86dbd967555b0dadd0161d8d6cb151c760222b13`.
The [paired packet](local-runs/authority-fd-path-20261021/README.md) records
one capture per compiler/stage. All three samples on GNU 1.94 and MSRV 1.88
agree: pure formatting becomes zero heap requests, versus one allocation and
28 requested bytes in the reference, for all eight descriptor cases.

Full inspect conserves twice that saving: allocations/reallocations/bytes
25/0/1718 become 23/0/1662 at depth one; 101/0/8500 become 99/0/8444 at seven.
Read requests remain 4/28; all six other cases and tracked kernel counts stay
equal. Reference/proposal/workload/qualification blobs are unchanged. No
measured invocation is retried or replaced.

Both lanes pass 201 authority tests, ancillary checks, affected all-target
check and strict all-feature lint. All 379 historical/current source assertions
and repository gates pass. The exact adapter restores the complete previous
runtime; every old assertion remains. This accepts only the local request
reduction. Separate boots do not establish continuity or repair old live time
failures. Delivery, writer cessation, lifecycle, SIGKILL/lost-ACK, reconnect
and real enrollment remain independent closed gates; there is no server or
product admission and no push.

Evidence `e0ca280b398395bda189aec1cc3841c4eb738957` seals 90 members plus
inventory SHA256
`ff4d9e59bffefb40eb1712f253eaebce701a34b832a5417211ec88b144fa2440`.
All fifteen prior packets remain intact; eight structural JSON and ten scope
mutations are refused. All 91 Git entries preserve raw bytes. The packet is
immutable, and frozen qualification SHA256 remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
