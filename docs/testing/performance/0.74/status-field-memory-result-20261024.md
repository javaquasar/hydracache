# Fresh-field index: requested heap parity, not an allocation improvement

The separate allocation contract passes on installed GNU Rust 1.94 and MSRV
1.88: 264 matched cases per compiler, four independently built images and 1056
parser executions. Baseline/candidate semantic and allocation transcripts are
byte-identical, including held-result and after-drop observations. No tracked
requested bytes remain after dropping any result. The index's CPU improvement
does not remove the parser's existing groups-vector allocation.

The [81-member packet](local-runs/status-field-memory-20261024/index.json) has
40 objects and index SHA256
`9db1d2987a239cf16574e09552cca3862e08720a7cbd44c83a2f0910eeb5f336`.
It retains exact generated sources, commands, image hashes, first compile
refusal, all four transcripts, strict Clippy logs and derived per-case facts.
No ELF or metadata image is checked into Git. Source/tool commit: `7e6c8ee8`;
method was frozen separately in `8c4008a5` before either allocation build.

## What was measured

Use the already retained exact baseline/candidate projection sources and group
collector, with the original eight shuffled documents, seed 0x07405ca1 and
512/2048/8192-byte target shapes. No old CPU cohort is repeated. Runtime source,
fresh-read/check count/order and admission windows remain unchanged.

The new dependency-free harness meters Rust System allocator requests only
inside one single-threaded parser invocation. Dataset generation, mutations,
formatting and output occur outside that interval. Before parser cases, explicit
owned alloc/zeroed/realloc/free operations prove accounting: three requests,
56 gross requested bytes, 40 peak live bytes and zero after both frees. A
returned successful projection is observed while still alive, then dropped
inside the same interval; zero-at-end alone is not the entire oracle.

Each document covers valid input, bad early scalar, duplicate/missing field,
bad group token, credential mismatch and group widths 0/1/4/32/33. All exact
outcomes, first error sites, complete group values and workload witnesses match.

| Case | Allocation/reallocation calls | Gross requested bytes | Bytes held by returned result | After drop |
| --- | ---: | ---: | ---: | ---: |
| Original valid three-group document | 1 | 12 | 12 | 0 |
| Early scalar/duplicate/missing/group-token refusal | 0 | 0 | 0 | 0 |
| Credential refusal after group parsing | 1 | 12 | 0 | 0 |
| Empty groups | 0 | 0 | 0 | 0 |
| 32 groups | 1 | 128 | 128 | 0 |
| 33-group budget refusal | 1 | 132 | 0 | 0 |

These values are equal in all four images and all applicable documents. The
33-group error still allocates before the unchanged budget predicate, then
releases its vector before returning the error. That observation is not a
request to reorder the predicate or accept another candidate without a baseline.

## First failure and reproducibility

The first baseline resource build executes no parser: its appended inner
doc-comment is invalid after the exact context/dataset items (E0753). Preserve
the generated source, failed build and original coordinator. A distinct v2
changes only that probe comment to an ordinary comment. No accounting,
dataset or acceptance rule changes. V2 passes rustc warnings-as-errors and
standalone strict Clippy on both compilers, for both arms.

Six offline controls verify exact extraction boundaries, unchanged dataset and
context, source-receipt identity, ambiguity refusal, corrupted object rejection
and reconstruction from the pinned repository archive. A fresh checkout does
not need the old timing ELF to recover the exact source inputs. The coordinator
used for the actual run is retained separately from its later source-archive
fallback enhancement; equality of recovered source bytes is checked offline,
without another allocation or timing execution.

## Claim boundary

This is optimized extracted-parser requested-heap parity, not allocator arena
retention, physical copying, stack high-water use, RSS or whole-runtime memory.
No error-path CPU result or native/RESP throughput is measured here. Temporary
field-index storage is not a retained observation cache. Never extrapolate an
unmeasured full preparation gain from these allocation counts or the earlier CPU
ratios. Original delivery, all-writer cessation, completion, reconnect, real
enrollment and release qualification remain separate open gates.
