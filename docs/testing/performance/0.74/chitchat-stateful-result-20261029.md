# Q stateful component costs and fixed-caller applicability

Q is **blocked at the general-stream resource gate**. The prior original
239-case allocation subgate remains green and immutable; it did not qualify
all sequences. This result does not prove a production/native-API regression:
the component corpus varies budgets and continues after refusal, whereas the
real DeltaSerializer has one fixed MTU and documents stop-on-false. Root and
all release/host admission remain unchanged.

## Method and preserved implementation

Restore exact B/P/Q from the [Q packet](local-runs/chitchat-commit-20261029/index.json),
index `853f8691d7e9e1f0d2c1143908fddaefdb36e0fd24a507cbf7ac4424a3c61a71`.
Do not change their runtime methods, old tests, lock, Zstd features/allocator or
qualified native C object. Only test visibility and an additional TLS call
counter/stack trace are added. Byte-equivalence controls strip these diagnostic
surfaces and compare complete production bodies and old observer to their source.

Nine recipes are fixed before execution: immediate exact-tail finish; roomy
continuation; cheap refusal/continuation; effectful refusal/continuation; two
tight continuations; repeated cheap refusals; accepted multiblock; small raw
tail; and effectful refusal without cache. Thresholds 16/64/1000, explicit
budgets/payloads and seed `0x0741028` are recorded in source and full raw rows.
This is a component matrix, not a production request/MTU distribution.

One GNU Rust 1.94 release **B/P/Q/Q/P/B** cohort, two observations/scenario/process.
Every process first has matching actual-image controls. Full constructor-through-
drop regions observe logical Rust, tagged physical Rust and native C requests.
Lengths/capacities and complete wire bytes use fixed stack storage; JSON is
formed outside the region. All native errors, failed/external/live and logical
live counters are zero. Calls, workloads and within/cross-process observations
repeat exactly. Balanced frees do not qualify allocator retention or RSS.

All nine P/Q decision sequences and complete wires agree. Old B differs in two
known unsafe-admission scenarios; they are retained and excluded from B/Q cost
comparison, never presented as identical work. Seven B/Q scenarios are comparable.
Before execution the method requires no increase in any of twelve allocation
fields or summed retained Vec capacities at a matching checkpoint; no lower-C/
higher-Rust tradeoff can turn that gate green.

## Results

For immediate exact-tail finish, B/P/Q execute **3/6/3** compression calls. This
preserves Q's original benefit. In four continuation cases, B/P/Q instead execute
**3/6/4**: Q discards the cached encoding when a later accepted append changes
the tail, then compresses it again at finish. Each adds two native C allocations
and 26,512 requested C bytes relative to B. In the roomy example C allocations
are 6/12/8 and requested bytes 79,779/159,555/106,291. This is a component work
increase, not measured CPU latency or a production throughput conclusion.

On effectful refusal Q restores logical output/pending contents and cached bytes,
but Vec truncation retains temporary output capacity:

| Scenario | P capacities: output/pending/scratch | Q capacities | P/Q summed capacity |
| --- | --- | --- | --- |
| Refusal after a cached tail | 256 / 4128 / 64 | 8192 / 4128 / 64 | 4448 / 12384 bytes |
| Refusal without cache | 64 / 132 / 16 | 256 / 132 / 16 | 212 / 404 bytes |

For the cached-refusal sequence, Q/P logical reallocations are 9/4, requested
bytes 26,110/10,396 and peak live 12,463/4,527. On the no-cache sequence they are
6/4, 1,688/1,304 and 483/291. Physical tagged-Rust observations independently
retain their corresponding regressions. Q may do less C work than P, but that
does not compensate for the retained-capacity/peak/reallocation failures.
Writer stack size is separately 80 bytes for B/P and 96 for Q; it is not counted
as heap retention. No RSS/PSS, allocator-retention or CPU claim follows.

The predeclared **general-stream resource gate is negative**. Q is not integrated
and no CPU/MSRV/Windows/RSS rescue cohort is run. There was no adopted product
mutation to roll back. Both the old positive and this negative result/code stay.

## Applicability audit: why this is not yet a caller verdict

The full reviewed DeltaSerializer body contains one private `mtu` field, one
constructor initialization and one `self.mtu` read, passed to the actual Q method.
It requires MTU >= 100 and derives threshold as min(16384, MTU); these component
thresholds/budgets do not substitute for that constructor. Its documented caller
stops after false. Budget-changing/continued-after-refusal traces therefore
cannot be called equivalent actual DeltaSerializer work.

There is also a direct cache guard argument. Let E be committed output and L
the pending tail after exact-tail acceptance. The branch computing a cached tail
required **E + L + 4 > M**. Under the same budget M, any subsequent nonempty item n
has old floor E + L + n + 4 (or +7 when crossing the threshold), which is greater
than M. Q rejects it before cache backup/invalidation. Thus the cached-continuation
C counterexample needs a changed budget. The source-bound offline arithmetic
control verifies this implication without changing or running the product.

This does not prove all actual-caller costs acceptable, or erase the refusal-path
capacity behavior. The next required gate binds real with_mtu/node/KV/builder/
finish/final Delta serialization at fixed budgets and stop-on-false. The
[separate prepared method](../../../../tools/chitchat-stateful-074/fixed-caller-plan.md)
retains six MTUs/five sequence families and requires a frozen exact request table
before execution. No actual-caller performance cohort has run. This is the
remaining applicability question for the same Q, not a new optimization idea or
a changed numerical threshold.

## Checks, custody and unchanged boundaries

Each B/P/Q image passes six selected controls: original allocator/C/tag controls,
original 239-case semantic control and two new stateful controls. Six cohort
tests then execute the actual observation body: 24 selected native executions
in total. Catalogs are B 121 and P/Q 131; the other 114/124 unique cases per image
are not claimed as covered. All three all-target/all-feature checks and strict
Clippy pass. Eight offline accounting/source/applicability controls pass, including
independent faults in all twelve allocation fields and capacity, payload/budget/
order/counter/semantic drift. Rustfmt and local non-promotable performance contract
pass. This is not full package/workspace or MSRV/Windows qualification.

Raw bodies, actual image/catalog/flags, object bytes, original ZIP input, source
versions, original controls and retirement receipts are in
[the stateful packet](local-runs/chitchat-stateful-20261029/index.json): 841 logical
members / 335 objects, index SHA-256
`0b195601989e9109725812ef5f8da76b3b2c9b4b42aed57e8e7992b6a25f35fa`.
Compiled images/build caches are excluded; archived code is not executed by
verification. Final byte/doc receipts are retained separately.

All product live predicates, frequency/order, original 300/60/10, tests 5/10/45,
package 780/workspace 4800 remain exact. Local 1200/native 1100-second envelopes,
four-GiB backing/native floors and five-second polling/retirement remain exact.
No services/clocks, server, supervisor, release installation, product workload or
frozen 0.73 artifact is touched. Root lock/deny/qualification digests are unchanged;
the last three Root Zstd findings and full release closure obligations stay open.
