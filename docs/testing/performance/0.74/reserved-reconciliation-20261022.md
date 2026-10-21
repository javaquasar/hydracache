# ReservedLive allocation candidates and unresolved costs

## Result

The five retained allocation leaves are present in the tree at
`1e5f31d9d443bf6c9b1d901781986501425bf9e9`. Three replace the first three
remaining candidates in the [earlier leaf audit](reserved-leaf-audit-20261021.md);
the other two were already retained at that audit. Eight owners in this leaf
inventory still need attribution or semantic review. This is not a count of
all remaining release tasks.

The separate [offline reconciler](../../../../tools/offline-reserved-reconciliation-074/README.md)
verifies eleven raw inventory pins and all 491 members, compares six imported
readers with their sealed method snapshots, reconstructs the old round facts,
and checks nineteen current source hashes and thirteen unique leaf witnesses.
It does not change the frozen older reviewers to accept a new runtime.
The [preregistered scope](reserved-reconciliation-plan-20261022.md) applies.

## Confirmed allocation changes

Each row is one separate synthetic corpus per compiler. GNU 1.94 and MSRV 1.88
agree within each row. Counts are from sealed earlier measurements, not new
executions. Subset, stage and short-NUL cohorts are not added to the main rows.

| Retained leaf | Constructions per compiler | Confirmed allocator requests before and after | Requested bytes before and after |
| --- | ---: | --- | ---: |
| Owned pidfd pathname | 2304 | allocations 2304 → 0 | 115200 → 0 |
| Groups collection | 1792 | reallocations 2560 → 0 | 237568 → 93184 |
| Short local context document | 2304 | allocations 2304 → 512 | 64768 → 26368 |
| Short shared trust component name | 5376 valid names | allocations 5376 → 1536 | 1274880 → 1221632 |
| Collector peer primary capacity | 7936 | allocations 7936 unchanged; reallocations 7680 → 0 | 1798144 → 627712 |

The machine-readable reconciliation retains the exact baseline and candidate
source pair for every row. The Groups summary lacks allocation-count fields;
the reconciler leaves them absent rather than interpreting absence as zero.
The longer fallback cases remain in the context/trust cohorts. The peer still
allocates an owned result; eliminating growth is not eliminating the result.

These are allocation-request results in diagnostic tooling, not RESP/native
API throughput claims. Different corpora and source pairs cannot be summed as
a combined workload. Requested bytes are not live memory, RSS or allocator
retention. Stack storage, copying, zeroing and canonicalization still cost work.

## What the old trace still establishes

The measured material source remains
`1d6d179bd7150ef564560821a897794e20b24b36`, not the current implementation.
The original GNU/MSRV ReservedLive prefixes contain 56/59 complete calls and
2808784775/2941286486 ns of nested wall time. Their kernel wall outside
instrumented children is 1745205141/1828186002 ns; the remaining step wall is
1063579634/1113100484 ns. The partition is conserved, not observer-corrected
CPU time. Kernel, live and authority views overlap.

The first Causal(Time) refusals remain at ages 10042390247/10010835739 ns, under
the original 300/60/10-second windows. Old call counts cannot convert the new
pure allocation results into current latency, total saved requests, or admission
headroom. Successful material revalidation/drop and the cleanup EOF/fence footer
remain absent from those original attempts.

## Remaining owners and required proof

Review order below follows scope and risk, not an elapsed-time ranking.

| Owner still present | Next proof before a candidate |
| --- | --- |
| Proc document scratch | Separate allocation/zeroing/read costs and invocation lifetime. The zeroed 65537-byte Vec is reused inside one sender validation; do not move it onto recursive stack frames or cache observations. |
| Sample boot ownership | Isolate the owned boot clone from the two fresh Origin checks and clock samples. An inline/shared representation needs a separate lifetime and compatibility decision. |
| Authority fresh record buffer | Preserve flags, ownership, metadata, budget, bounded read to EOF and closing stamp/length. Observed length does not justify skipping growth/EOF checks. |
| Authority inventory ownership | Freeze exact names, duplicates, count, oddness and first-error order before a compact recognizer. |
| Trust snapshot buffer | Preserve its separate 2048/8192-byte limits, offset loop, EINTR retries and named/retained checks. The context one-read helper is not its replacement. |
| Local context temporary dupfd | Establish descriptor lifetime and first-error behavior while retaining named ancestry and procfs checks. |
| Canonical request reconstruction | Separate cloning, hashes, serde and canonical comparisons; this is not one allocation leaf. |
| Fresh kernel traversal | Preserve every fresh namespace, metadata, filesystem, ioctl, boot and ancestry observation. Span counts are not syscall counts. |

The next bounded offline work is a baseline for proc DocumentBuffer construction
and reuse, with synthetic reads and error controls before considering a storage
change. This chooses a reviewable scope; it does not establish that the buffer
dominates current preparation time or authorize a new live attempt.

## Checks and admission

Focused checks cover twelve new reconciliation controls, fifteen round-reader
controls, nine material-reader controls and eighteen arithmetic controls, for
54 distinct offline tests. They
exercise raw seals/membership, path safety, reader/source drift, ambiguous
witnesses, corpus identities/counts, first refusals and forbidden promotions.
All 37 Windows source canaries, xtask all-target check, strict all-feature clippy,
doc-check, unchanged 186-gate registration and the local non-promotable performance
contract are recorded in the separate packet. There is no full milestone gate.

Runtime and observer sources, compatibility, gate registry and qualification
manifest remain unchanged from the starting tree. No new measurement, WSL/live
profile, clock/service change, server operation or product workload occurs.
Material, cleanup/EOF/fence, authenticated delivery/all writers, full SIGKILL/
lost-ACK, reconnect and server gates remain closed.

Reconciler source: `5f05c131154597c2aebbb44eeb6fe55adb06384e`;
preregistration: `3367e4b4`. No Rust source changes require a new formatting run;
PowerShell parsing and diff whitespace checks accompany the focused cohort.

## Evidence and replay

The [facts](local-runs/reserved-reconciliation-20261022/facts.json) retain each
source pair, separate cohort, unresolved witness, original refusal and nested
wall partition. The packet has 42 members plus its inventory; raw inventory
SHA-256 is `d3aa773d6254d5297d303cfc8ba7a397e06ae8a2d4978e39e29c496062b3fb72`.
It includes current source/method snapshots and the first green focused cohort.

```powershell
python -B tools/offline-reserved-reconciliation-074/reconcile.py --output docs/testing/performance/0.74/local-runs/reserved-reconciliation-20261022/facts.json --verify --packet-sha256 d3aa773d6254d5297d303cfc8ba7a397e06ae8a2d4978e39e29c496062b3fb72
```

Replay is read-only and source-pinned. A later runtime must use a separately
identified reconciliation rather than weakening this one to accept drift.
