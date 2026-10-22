# Owner-bound fresh authority scratch: allocation hypothesis, not runtime adoption

The [preregistered experiment](authority-scratch-reuse-plan-20261022.md) confirms
that temporary fresh-read allocation can be amortized in the synthetic model.
It does not change the supervisor, authenticate delivery or open server admission.
The [raw packet](local-runs/authority-scratch-reuse-20261022/facts.json) retains
the first reference preflight, candidate observations and tooling refusals.

## Implementation and ownership

`tools/authority-scratch-reuse-074` is a dependency-free test image, not a runtime
dependency. Its original model/receiver and old nine controls remain unchanged.
The separate candidate borrows exactly one Leaf; expected raw bytes/File/stamp
remain independent. It retains only a successful temporary Vec, clears its length
before fresh reading and reuses capacity only when equal to the new metadata
length. A mismatch takes the original exact-capacity allocation path. Every error
evicts cached storage, including held/open/before/parent failures before reading.

All original metadata/regular/parent/budget/after checks, generic take/read_to_end,
stamp/content comparisons and fresh File lifetime remain. Scratch is neither an
authority latch nor a cross-owner pool. Its borrowing discipline does not prove
OS descriptor custody. The source canary restores complete original read/validate
bodies after removing only the explicit storage/retention substitution.

## First observations

Both Windows MSVC compilers, Rust 1.94 and MSRV 1.88, agree on the nine cold-owner
cases: 256 full revalidations per arm/case, 2304 calls per arm, seed 0x074a17b3.
Reference makes 2048 allocations requesting 1222656 bytes. Candidate makes eight
allocations requesting 4776 bytes, with no reallocations; only the first nonempty
read allocates. Candidate cache drops are captured separately: eight deallocations
of 4776 bytes, not pooled into main calls or error cohorts. Leaf setup and all
formatting/trace output are outside captured operations in both arms.

The saving has a storage cost. Scratch retains one extra body-capacity allocation
per successful owner (65 or up to 2048 bytes for these record cases), in addition
to independently retained expected bytes. Its wrapper is 32 bytes, alignment 8,
on these targets. Those capacities/layouts are not RSS, allocator arena release,
physical copied bytes or total server memory. Fresh reading and comparison still
deliver/process the same content; no copy-elimination claim is made.

Two fixed cases each retain seven paired intervals, alternating arm order, with
64 warm calls and 512 timed calls. Medians below are nanoseconds per 512-call
interval, not per-operation latency or CPU/op:

| Compiler | Body bytes | Reference | Scratch |
| --- | ---: | ---: | ---: |
| 1.94 MSVC | 65 | 89000 | 63600 |
| 1.94 MSVC | 2048 | 672600 | 664500 |
| 1.88 MSVC | 65 | 90400 | 71200 |
| 1.88 MSVC | 2048 | 840300 | 815200 |

The predefined two-compiler median rejection rule is not triggered. However,
compiler/host checks ran concurrently: wall intervals are preliminary and noisy,
not isolated speedup evidence. All first samples, including slower candidate
pairs and outliers, remain. No original-owner profile or measurement was retried.
The earlier reference-only preflight is separate from paired candidate intervals;
its fixed helper was committed before the candidate existed.

## Controls, repairs and decision

Each candidate lane passes fifteen tests (the preflight is filtered, not rerun),
including the unchanged nine old controls. Paired comparisons cover 64 fault
masks; 1512 IO schedules with changed declared/body lengths, fragments, partial
errors and Interrupted; 28 metadata/stamp/device cases; 256 seeded content
changes; cold/warm storage, exact owner identity and independent cache drop.
Exact outcomes and complete receiver traces match in every paired Rust control.
Early refusals retain their original observation prefix while discarding already
cached memory. This is intentionally different allocation lifetime, not different
authority/error precedence. Token retirement is not all-writer termination.

The offline reader validates corpus membership, request weighting, trace/order/
bounds/error/lifetime invariants, first receipts and every old baseline row. Its
eight mutation tests join fifty-five previous offline tests; 52 source canaries,
format/check/strict clippy and focused documentation/registry/performance checks
complete the local gate. Eighteen previous seals and 1000 members are verified.

The first runner path and whitespace-only source-canary failures are retained
with original source/logs and a separate tooling repair. A development offline
test also incorrectly expected an observed read request to equal its bound; its
fixture repair is explicitly recorded. No failed measurement was replaced.

Decision: **retain an unpromoted storage prototype**, not a runtime optimization.
Next perform an offline integration/lifetime and retained-capacity budget audit
for real Leaf/registry owners before proposing any runtime candidate. GNU and
full-path/copy/memory evidence remain absent; further execution requires its own
explicit cohort and permission. Original windows/refusals and qualification stay
unchanged. Material/cleanup, authenticated delivery/all-writer termination, full
SIGKILL/lost-ACK, reconnect and server admission remain closed. No WSL, clock/
service changes, server installation, supervisor stop or product run occurred.

Source history: plan `2cf622f9`, reference `187ac5c7`, candidate `cefa6709`,
tooling repair `c9fc9ec5`, offline reconstruction `63a7cd27`.

The raw inventory seals 76 members (excluding inventory.json itself), SHA-256
`b5a90f56551c11378422879ae861a05c8fa3c9bb6448e9dc8c1f255ee3f20159`.
All raw working/staged bytes were checked for equality before the evidence commit;
all fourteen final local steps and their log hashes passed.
