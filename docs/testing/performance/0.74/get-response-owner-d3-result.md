# Private GET response ownership: local D3a screen

Decision: **screen passed; full D3 controls still required**. This is neither
acceptance nor default activation. Accepted product proposals remain zero and
C74 remains unresolved. The off-by-default D2 implementation is unchanged.

## Identity and complete attempt set

Contract/instrumentation source was committed before any numerical attempt:
`213e9e0a3c9c50e089e5aed2583913f8c6d10807`. Product implementation is `2379698d`;
the baseline and candidate are separately built release binaries from the same
clean screen source, with the sole `get-owner` wrapper feature off/on. The default
private executor/public borrowed reducer corresponds to `36f60265`; no old binary
is relabeled with the new instrumentation. Source, tool lock, compiler, contract
and both binaries are sealed in `local-runs/get-owner-d3a-213e9e0a/seal.json`.
Compiler is rustc 1.94.0, x86_64-pc-windows-msvc, LLVM 21.1.8.

Nine preregistered cells completed five independently started A/A pairs and five
alternating A/B pairs each: **180 attempts**, zero failed/invalid attempts, zero
retries. A/A uses the same off binary for both named roles. All responses, dispatch
and mutation counts, errors, final values/cardinality, seed and workload hashes
reconcile. Every operation has one separate scripted write and flush; errors are
zero. All 362 files (180 raw, 180 attempt receipts, seal and summary) are copied
byte-identically from the original packet, retained and independently replayed.
Attempt receipts bind raw-byte SHA-256; archive paths are `-text` in Git.

The full canonical connection decode/translation/dispatch/reduction/encoding/
write/flush/close path is counted. Preload, oracle preparation, five warmup batches,
post-window final validation and receipt serialization are not counted. This is
fixed-count, production-clock, no-TTL, concurrency-one, immediately-ready scripted
RESP2 plaintext IO, not a timed socket saturation or expiry screen.

## Repeated allocation and requested-live result

Each row below is identical across all five A/B pairs. Every A/A gross and peak
ratio is 1.00. All five paired ratios are retained with their preregistered log-
scale t(4) 95% intervals. The intervals collapse to the repeated ratio for these
deterministic counts; that is not certainty about production CPU or physical memory.

| Cell | Gross bytes/op off | Gross bytes/op on | Reduction | Window peak above start off -> on |
| --- | ---: | ---: | ---: | ---: |
| GET 4 KiB / p50 | 15,243.952 | 11,147.952 | 26.8697% | 25,138 -> 24,600 |
| GET 1 MiB / p10 | 3,147,376.520 | 2,098,800.520 | 33.3159% | 2,114,095 -> 2,113,563 |
| GET 4 KiB / p1 | 13,557.954 | 9,461.954 | 30.2110% | 25,138 -> 24,600 |
| GET 256 B / p50 | 3,722.952 | 3,466.952 | 6.8763% | 20,005 -> 20,005 |
| SET 4 KiB / p10 | 24,582.270 | 24,582.270 | 0% | 41,096 -> 41,096 |
| GET mixed 4 KiB/256 B / p10 | 8,108.509 | 5,932.509 | 26.8360% | 25,138 -> 24,600 |
| GET 4 KiB / one-byte reads / p10 | 19,024.509 | 14,928.509 | 21.5301% | 25,138 -> 24,600 |
| GET empty hit / p10 | 1,580.009 | 1,580.009 | 0% | 17,245 -> 17,245 |
| GET miss / p10 | 1,579.909 | 1,579.909 | 0% | 17,245 -> 17,245 |

For each nonempty GET, exactly one successful allocation and exactly its payload
size disappear per operation: 4,096 or 1,048,576 bytes for primary cells; 256 for
small GET; mean 2,176 for the alternating mixed cell. Calls/op are 30.002 -> 29.002
for GET 4 KiB/p50 and 30.020 -> 29.020 for GET 1 MiB/p10. These differences match
the independent D1 borrowed-reducer attribution. Empty hit, miss and SET allocation
counts and peak are unchanged. No encoder/native/materialization cost was removed.

Both primary cells pass the unchanged 20% end-to-end gross allocation floor.
Other controls pass the 1.05 gross ceiling, all cells pass the 1.00 peak ceiling,
and every paired next-read/post-close owner increment delta is zero. Every measured
connection releases to its starting requested-live bytes on close. Next-read
owners remain 16,399 or 24,591 bytes above start in both variants; these represent
still-live connection owners, not a zero-retention claim.

Unlike the rejected encoded scratch, the transfer does not retain an output buffer
across commands. Large-value materialization and the encoded frame still overlap,
so gross allocation reduction does not translate into an equally large peak
reduction. The small 538/532-byte peak differences are observations, not a new
stage-by-stage attribution of particular metadata. Do not add D1 stage maxima or
compare this end-to-end denominator with D1's prebuilt-plan totals.

## Remaining boundary

The System profiling allocator counts outstanding requested layouts, not metadata,
hidden realloc overlap or allocator active/resident/retained bytes. RSS endpoint
and lifetime-peak fields are supplemental and cannot prove timed post-idle return
or refill behavior. Scripted write/flush calls are not syscalls. Allocator overhead
does not certify CPU/op, goodput or p99. No numerical native guard is established
by unchanged native source.

Next is a separately sealed phase B contract: independent embedded, direct
ClientSurfaceState, HC/1 and HC/2 controls; unprofiled scheduled latency/goodput/CPU;
real plaintext and matched-mTLS transport; RESP2/3, concurrency and slow readers;
allocator/RSS idle/retention/refill; hosted feature-on semantic CI. No phase B
numbers, real host service mutations, rented infrastructure, workflow dispatch,
6-hour/24-hour qualification or full workspace admission were performed here.
The latest inspected hosted success is an earlier ancestor (`3ce74350`), not proof
for the new feature. Branch pushes alone do not start CI in the current workflow.

The feature remains off, product code is unchanged from D2, earlier terminal
negatives and all frozen 0.73/qualification inputs are unchanged. Passing an early
screen permits further qualification, not a release or portable speedup claim.

Local verification: seven serialized Rust tool tests per variant, 29 Python
replay tests (including old negative scratch/D1 archives), 101 targeted xtask
tests, three docs-script tests, scoped format, tool/xtask check/strict lint,
contract, 17 governance checks, docs registry/sync/links and mdbook passed.
The corrected post-measurement archive-guard compile diagnostic and exact scope
are retained in `local-runs/get-owner-d3a-checks-213e9e0a.json`; no measured source
or numerical attempt was changed. Clean-source canaries are recorded separately.

Clean `d32fbb18` produced W11/W12 ExpectedRed receipts (1,738 / 13,898 ms).
All 362 committed Git blobs match raw file bytes and the packet replays exactly.
Local aggregation still reports zero fast-green/gated-green/ship-ready rows;
28 structural implemented rows are not 28 accepted optimizations. Full receipts
and scope: `local-runs/get-owner-d3a-verification-d32fbb18.json`.
