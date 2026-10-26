# Offline attribution of Git observer variance

The saved observer-cost result contains variance in three distinct CPU owners:
the original worker, its descendants and the external coordinator. Splitting
them explains why a lower whole-Job CPU value is not evidence of cheaper
observation. Recorded direct observation wall spans are short, while spawn/wait
spans and whole-process CPU differ substantially. The data do not identify the
causal API or establish small total instrumentation overhead.

This is an offline analysis of the
[single previous attribution](pipe-fixture-git-overhead-result-20261026.md),
not another measurement. All sixteen workers, sixty-four original calls, first
outcomes and block order remain; no data is trimmed or relabeled as a new trial.
No original collector, Git worker, runtime, validation predicate, observation
frequency, window, threshold or estimator changes.

## Fixed input and reproducible analyzer

The [analyzer](../../../../tools/xtask-package-attribution-074/git_cpu_variance.py)
and [scope](../../../../tools/xtask-package-attribution-074/git-cpu-variance-design.md)
are committed at `385feda3938aabf467857271047e60a5c961619f` before the saved CLI
analysis. It accepts only the sealed input index
`1ba276dcf62f97801f69806ea5efba138c7d47425f30d2563874aa393681fcff`, hashes all
108 input objects and joins individual worker ledgers to their raw logs. It has
no process-launch, clock-reading or retained-code import path. Its CLI writes
an exclusive intent before preflight and preserves the first refusal without
overwriting an existing label.

Seventeen tests cover complete-order refusal, first faults/cleanup, source and
raw-input corruption, worker/launcher identity drift, nonterminal/unstable Job
census, counter regression, underflow, invalid types, missing reference phases,
zero counters and signed CPU deltas. An AST guard restricts imports to offline
standard-library modules. Python syntax/indentation checks and the first
committed-source CLI analysis pass. No Rust source changed or native tests ran
in this stage; their previous focused verdicts remain separate evidence.

## CPU owners explain the sign changes

For each original worker, lifetime Job CPU equals worker CPU plus descendant
CPU. The latter includes all Git launcher/native descendants, not only an
authenticated native image. External coordinator CPU is a process-wide delta
outside that Job. It is neither worker CPU nor a leaf API counter. Adding Job
and coordinator gives only a labelled recorded CPU envelope, not total host
CPU or a common measurement interval for every process.

Each row below is profile minus reference over eight original Git calls per
arm. Units are ms; all signed values and arithmetic are retained exactly.

| Original block | Job CPU delta | Worker CPU delta | Descendant CPU delta | Coordinator CPU delta |
| --- | ---: | ---: | ---: | ---: |
| ABBA 1 | +31.250 | +15.625 | +15.625 | +31.250 |
| BAAB 1 | +15.625 | +15.625 | 0.000 | -500.000 |
| ABBA 2 | -250.000 | +421.875 | -671.875 | +15.625 |
| BAAB 2 | -31.250 | -31.250 | 0.000 | +500.000 |

The third block's apparent Job CPU saving therefore coincides with *higher*
worker CPU; the lower descendant charge more than offsets it. In that block,
the reference worker at slot 3 records 1312.5 ms descendant CPU, while the
profile worker at slot 2 records 453.125 ms worker CPU. These are different
owners, not a negative cost of reading counters.

The reference coordinator at block 1/slot 2 and the profile coordinator at
block 3/slot 0 each record 500 ms. Their respective block differences have
opposite signs. The existing observations do not separate Job setup, process
creation/pre-resume identity, wait, terminal queries, log decoding or handle
close inside that coordinator frame. No particular syscall, Python parser,
scheduler, antivirus, clock issue or Git implementation is established as the
cause by these aggregates.

Profile records additionally permit splitting descendant CPU into direct
launchers and the remaining descendants. Reference records lack launcher
counters, so the analyzer keeps that split null rather than substituting zero
or constructing a native-only paired CPU comparison.

## Recorded profile wall spans

All thirty-two profile calls contribute to each row; reference per-call phases
were not captured. These are elapsed wall observations, not CPU samples.

| Recorded span | Sum ms | Minimum ms | Maximum ms |
| --- | ---: | ---: | ---: |
| Spawn | 1379.0763 | 9.5603 | 516.8340 |
| Handle observation | 0.5534 | 0.0116 | 0.0447 |
| Wait and collection | 3527.1732 | 64.0317 | 495.5811 |
| Terminal observation | 0.0901 | 0.0023 | 0.0044 |
| Handle plus terminal observation per call | 0.6435 | 0.0142 | 0.0476 |

The handle span includes retained-handle duplication and the initial query;
the terminal span includes the final query. Their recorded sum does not include
all profile overhead: command setup, assertions, phase serialization, later
identity checks, Job membership and external coordinator work have other or
unmeasured boundaries. Spawn and wait also include useful execution/scheduling
or collection; they are not removable overhead estimates.

For example, profile block 2/slot 2 has 561.0661 ms summed spawn wall, but only
0.1093 ms summed handle/terminal observation wall. This associates a large
recorded wall span with spawn, not with the direct observation spans. Without
CPU snapshots per phase it does not place the worker's 453.125 ms CPU inside
spawn or prove why spawn was slow. Keep CPU and wall separate; do not subtract
lifetime CPU from wall or FILETIME origins from monotonic durations.

The 144 recorded primitive CPU values have a greatest common divisor of
15.625 ms. This describes the finite saved counters, including retained zeros,
not proven OS timer resolution or permission to alter clocks or thresholds.
The four phase spans do not partition an entire worker/coordinator frame, and
missing reference/coordinator phases cannot be reconstructed from their totals.

## Remaining measurement boundary

The next separate instrumentation design should preserve the existing read
graph, checks, ordering and deadlines while recording boundaries around
coordinator Job setup, existing spawn/pre-resume validation, terminal queries,
decode/validation and original-handle close. Preserve raw clock endpoints and
clock domain, distinguish process-wide CPU from wall time, and keep worker
serialization/assertion costs separate from the existing per-call spans. Such
telemetry would itself require functional/fault and overhead checks; this
offline result does not authorize a new measurement or a product candidate.

Job-membership overhead, reference per-call phases, causal leaf cost and small
instrumentation overhead remain unresolved. The
[68-member packet and independent verifier](local-runs/git-cpu-variance-20261026/README.md)
retain exact input/output/source and recompute the arithmetic without replay.
Index SHA256:
`586480e0b6800d416ea2a83504b1beb3386fc0e33bb67fb78bcad125cf8a95d6`.
Four integrity controls cover index/missing-object refusal and corruption of raw
input, first results and analyzer source.
The separate final-checks index retains passing integrity, verifier syntax,
doc-check and local non-promotable performance-contract receipts with the exact
verifier sources. None supplies a package or release qualification verdict.

No new measurement, V6 repeat, server, service, clock, dependency, product load
or installation occurs. The package gate stays red; C74 and production admission
remain unresolved. All original first refusals, 780-second guard and 300/60/10
origins remain unchanged. Qualification manifest SHA256 stays
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
