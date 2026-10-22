# Installed authority runtime: allocation and retention tradeoff

The installed owned-storage runtime removes fresh-read buffer allocations on
repeated inspections, but retains more heap layout bytes while its owner lives.
This is a confirmed local actual-engine mechanism result, not a lower-RSS,
faster-preparation, product-throughput or server-admission claim. Inherits R-3,
R-5, R-7, R-8 and R-10.

## Frozen comparison

The [preregistered plan](authority-runtime-allocation-plan-20261022.md) pins
baseline `33c54b0cda8c330b115009cdf092ad002077cedf` and installed runtime
`66d5f371680d085a24de8f2558e1f049c71b9e7b` (implementation `37db65f6`).
Method commit is `12d7196e26830f6110214cf73d79d373efcdf44d`; evidence commit
is `980bb651e0e0b7f0d18e882cd4ca8a85e77b55fa`.

Exactly eight supervisor source/test files are restored for the baseline.
Complete package membership and bytes, original runtime/test bodies, root
Cargo.toml/Cargo.lock and generated instrumentation are verified. Both isolated
overlays receive the same test hook, probe and System counter; only an uncounted
read-only scratch inspector differs. No repository runtime file is edited.
The old allocation feature and historical model images remain disabled.

Rust 1.94.0 and MSRV 1.88.0 each use default features, optimized release libtest,
offline locked builds, one test thread, and separate per-role target directories.
Each selected artifact must be freshly compiled; baseline/installed executable
paths and SHA-256 must differ. Before measurements all four images pass strict
release lib/tests clippy and four counter/semantic controls each.

One persistent nonroot WSL runner builds, waits for committed method freeze,
verifies compiled probe/counter/driver bytes, then executes five fresh-process
pairs per compiler in BC, CB, BC, CB, BC order. All twenty measurement processes
pass their eight cells: depths 1/7/31/127 with external refusal, plus four further
depth-seven fault cases. Each cell reopens actual filesystem history and performs
32 full inspect calls: 5,120 measured warm inspections in total. Canonical
record/name/payload corpora, seed 74, operation order, root-name length, iteration
counts and logical outcomes match within every pair. There is no added warmup,
clock-based estimator or measured-cell retry.

Build/control receipts explicitly precede committed method freeze; these are
not described as clean frozen builds. The later measurement checkout is clean
at the exact method SHA. Before/after metadata matches, including WSL boot
`df4b2a6b-63e9-4339-8fbc-f1ee9f964986`, kernel, UID/GID 1000, configuration hashes
and raw qualification digest.

## Warm allocation requests

Every count below is identical across all five pairs, separately on each
compiler; both compiler lanes also agree. Values are per full inspect, not per
Leaf and not per request in a product benchmark. All warm realloc counts are zero.

| History depth | Allocations baseline | Allocations installed | Requested bytes baseline | Requested bytes installed |
| --- | ---: | ---: | ---: | ---: |
| 1 | 19 | 17 | 1,606 | 986 |
| 7 | 83 | 69 | 8,112 | 3,378 |
| 31 | 335 | 273 | 32,290 | 11,058 |
| 127 | 1,353 | 1,099 | 132,606 | 45,346 |

The removed allocation count is exactly two per generation: one fresh read
buffer for each record and commit Leaf. At depth 127 this is 254 allocations
(18.77%) and 87,260 requested bytes (65.80%) fewer per inspect in this fixture.
Parsing, inventory, fresh file access, metadata/stamp/content checks and their
remaining allocations still execute. Allocation reduction is not zero-allocation
validation, fewer live checks or proof of proportional CPU savings.

## Retained requested layouts

The counter tracks successful Rust layout requests, releases the old layout on
successful realloc, and records signed net requested bytes. The following net
belongs to the measured owner's lifetime after reopen/warm inspection. It is
not allocator usable size, process-wide memory, stack size or RSS.

| Depth | Baseline live bytes | Installed live bytes | Additional live bytes | Scratch capacity | Entry-vector capacity |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 | 2,263 | 3,075 | 812 | 620 | 4 |
| 7 | 11,017 | 16,135 | 5,118 | 4,734 | 8 |
| 31 | 48,251 | 71,019 | 22,768 | 21,232 | 32 |
| 127 | 197,223 | 290,627 | 93,404 | 87,260 | 128 |

Leaf size grows from 136 to 160 bytes; Entry from 272 to 320. Additional net
retention reconciles exactly to scratch capacity plus 48 times entry-vector
capacity. At depth 127 this is 87,260 + 6,144 bytes, about 47.36% more owner
requested layouts than baseline. Original raw capacity is the same 87,260 bytes
in both roles, and all original File descriptors, stamps and raw digests remain
unchanged through warm inspections. All retained scratch is already warmed by
the original mandatory closing validation during reopen; there is no hidden
extra warmup round.

Cold reopen provides no alloc/alloc_zeroed-count improvement: baseline/installed counts
are respectively 78/78, 583/583, 2,605/2,605 and 10,708/10,708. Total requested
bytes instead rise from 10,878 to 11,070; 86,786 to 87,362; 392,576 to 395,456;
and 1,621,196 to 1,633,292. The increased Entry vector also costs bytes at earlier
growth capacities, so cumulative requested-byte deltas exceed final inline
retention at depths above one. The benefit is reuse across later checks, not
cheaper cold reconstruction or lower retained memory.
Realloc calls are counted separately in the raw phase ledgers; requested bytes
include their new layouts, and the old layouts are released in lifetime totals.

## Cleanup and maximum publication

External latch, validation failure, policy-binding failure, invalid transition
and injected RecordCreated publication failure all clear every scratch slot.
Depth-seven cases release the retained 4,734 scratch bytes, leave original
history and the larger entry-vector layout alive, and preserve sticky refusal.
After refusal, live requested layouts are 11,017 baseline versus 11,401 installed;
the remaining 384-byte difference is structural capacity, not leaked scratch.

The depth-127 cell separately publishes generation 128 before external refusal.
Scratch grows from 254 slots/87,260 bytes to 256 slots/87,952 bytes. Live layouts
after publication are 227,739 baseline and 321,835 installed; after latch they
are 227,739 and 233,883. Every measured owner reconciles its open, warm,
publication, refusal, sticky and drop ledger to exactly zero. This is requested-
layout lifetime accounting, not proof that malloc returned all pages to the OS.
Raw logs retain all phase counts, reallocations and logical peak deltas. The
publication cut is a logical injected fault, not a new real SIGKILL/lost-ACK test.

## RSS is supplemental and does not support a memory-win claim

At depth 127, whole-process RSS after warm inspection ranges across five pairs:

| Compiler | Baseline RSS bytes | Installed RSS bytes | Paired installed-minus-baseline bytes |
| --- | ---: | ---: | ---: |
| Rust 1.94 | 9,232,384–9,494,528 | 8,970,240–9,236,480 | -262,144 to -126,976 |
| Rust 1.88 | 9,818,112–9,953,280 | 9,822,208–10,088,448 | -126,976 to +270,336 |

The direction is not stable between compiler lanes, despite identical logical
allocation/retention counts. These processes include executable/runtime pages,
fixture production, allocator residency and eight ordered reporting cases.
RSS samples run outside allocation capture; they are not an isolated owner
measurement. Do not attach a lower-RSS claim to the scratch optimization or
infer an OS-level leak from these ranges.

## First refusals, evidence and controls

All six pre-measure development attempts remain separate. First omitted a Cargo
workspace member; second found an old default-feature formatter dead-code lint;
third/fourth exposed incomplete static public include fixtures and new constant
assertions. Fifth passed controls but reused the same image across roles, so it
was not signalled and refused at its finite method wait without measurements.
Sixth failed copying dependency build-script executable modes. Archive/include,
lint, output-prefix, per-role target/freshness/digest and copy-mode controls fix
the harness rather than changing runtime, old helpers, assertions or thresholds.
Seventh first measurements all pass; none of those cells is rerun into success.

The final harness passes eight offline Python controls on Windows and Linux,
three standalone Windows counter tests, Rust formatting, sixteen native controls,
all twenty exact measurement tests and 23 current/historical source controls.
Caught-panic counter controls deliberately test RAII restoration; their expected
panic-hook stderr is retained with passing test results.

The [packet](local-runs/authority-runtime-allocation-20261022/) contains 429 sealed
members plus inventory.json. Inventory SHA-256 is
`496e3628c1975e5e006c1b8a12ecef8211314c3367cff1d0d50b40c7846f9913`.
All 430 working packet files match staged raw Git objects. The previous runtime
inventory (`fafe2b63…`) and all 364 members are unchanged. The packet contains
first refusals, original overlay manifests/probes, dependency-copy inventories,
committed method/runtime archives, executable hashes, all pairs and derived facts.

Offline verification, without rerunning a measurement image:

```text
python -B tools/authority-runtime-allocation-074/seal.py docs/testing/performance/0.74/local-runs/authority-runtime-allocation-20261022
```

## Remaining boundary

Allocation reuse is confirmed; higher retained memory and cold requested bytes
are equally confirmed costs. No runtime change or rollback is made from these
measurements. CPU/end-to-end benefit needs a separately frozen attribution,
without weakening the original validation graph. This result does not complete
authenticated original delivery, all-writer termination, later lifecycle cells,
real SIGKILL/lost-ACK, reconnect monitoring or production enrollment/server gates.

Original live check count/order, 300/60/10 windows, first Causal refusals
10042390247/10010835739 ns and qualification raw digest
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`
remain fixed. No live preparation, clocks/services, server, installation,
supervisor or product action occurs in this experiment.

## Documentation follow-up

Doc-check, the 186-gate registry, the local non-promotable performance contract,
eight offline Python controls and read-only packet/predecessor verification pass.
All maximum-publication phase counts/state also repeat across the five pairs in
each role/compiler. [Follow-up receipts](local-runs/authority-runtime-allocation-doc-followup-20261022/)
are separate from the immutable measurement packet. No full workspace/release
qualification or additional native/live cohort is implied by those doc checks.
