# Remaining preparation cost ranking

The next isolated hypothesis is the fresh directory metadata projection in
`PinnedDirectory::same`, not another open or filesystem-buffer rewrite. Saved
evidence identifies a remaining LocalContext wall span of about 0.901 seconds
in outputs. A source-derived inventory places 24 of its 88 metadata requests
per successful observation at directory comparisons. This selects a measurable
scope; it does not predict CPU savings. No candidate or new server run occurs
in this offline analysis.

## Evidence boundary and tree conservation

The input is the [parent-device residual profile](authority-parent-cost-result-20261025.md),
not a repeated healthy preparation. Its exact source is
`e5284749e84a1a025ea1ef47f2b20e744a72bed1`, test image SHA-256
`1d43e73b00403b0ee834d11b392f754b124b40e5e6151d1a28cc22a020f5afc1`.
The upstream index SHA-256 remains
`afb69bd536c0399dc378095dd6d3420a4bbb451cf0ce79675370a00c08c7551b`.
The reader verifies member hashes, the measured 2565-input source manifest and
eight current source files before interpreting the trace. This is a selected
source-boundary check, not a new whole-package or release verdict.

The [new reader](../../../../tools/preparation-ranking-074/review.py) checks 56
round-slot inventories across seven observed phases. Forty-four slots are active;
twelve are empty. Each node's calls must equal its incoming edge calls plus root
calls. Its inclusive wall minus outside-children wall must equal its outgoing
direct-child wall. Incoming time cannot exceed the node's inclusive time;
root wall must equal the total outside-children wall within that kernel tree.
Root coverage must fit its slot, slot totals their phase, and per-kind slot
counters their corresponding phase counters.

These equalities establish consistent aggregation within this instrumented
view. Kernel, live and authentication views remain separate; none is an
exclusive process-CPU partition. Wall minus process CPU is not an IO attribution,
and observer overhead is neither removed nor assumed zero. Duplicate JSON keys,
invalid numeric types, repeated/malformed edges, reordered slots, unbalanced
counts/time, changed source/pins and promoted scope all refuse explicitly.

## Ranked observations from the retained outputs phase

Outputs records 6.680365 seconds wall and 6.596969 seconds process CPU. Its
ReservedLive slots account for 5.640556 seconds wall, about 84.43% of that phase,
with the original 46 calls/completions in every slot. All round-slot wall totals
sum to 6.591623 seconds; the remaining 88.742 ms is outside those timed slots,
not an inferred IO cost or opportunity to remove a validation round.

| View | Span | Calls | Wall outside instrumented direct children |
| --- | --- | ---: | ---: |
| Kernel | Boot | 153088 | 1.420837 s |
| Kernel | Namespace | 306176 | 1.137560 s |
| Kernel | Origin | 76544 | 0.789160 s |
| Kernel | Document | 232576 | 0.423464 s |
| Live | KernelContext | 8464 | 3.721872 s |
| Live | ActiveAuthority | 17066 | 0.959482 s |
| Live | LocalContext | 4232 | 0.900602 s |
| Live | WorkerSource | 16928 | 0.328767 s |

Rows from different views overlap and must not be added. In particular,
`KernelPacket::require_worker_context` creates a live KernelContext span around
the same body as its kernel PacketContext span. The latter records 3.721425
seconds inclusive, with the same 8464 calls. Ranking the live wrapper as another
3.72 seconds of unowned work would double-count the kernel operations already
being investigated. The small enclosing-wall difference is not a CPU estimate.

LocalContext is the separate `Probe::observe` path. It repeatedly checks named
ancestry, retained/current namespaces and bounded machine/boot documents. Its
source contains no Origin/Boot/Namespace numeric spans from the other view.
WorkerSource is the original fixed trust-source reread path. These are useful
remaining attribution scopes; neither may cache a successful earlier read.

## Source-derived observation inventory

The [reviewed graph](../../../../tools/preparation-ranking-074/local-context-graph.json)
describes only a successful `Probe::observe` on the exact pinned source. Two
iterations each bracket two document reads with namespace checks: four brackets
and four positional document reads altogether. Each bracket performs one named
path walk, one fresh namespace opening, two namespace comparisons and two thread
checks. Each named walk includes six retained directory-stamp comparisons.

| Request category | Per successful observe | Derived for 4232 outputs spans |
| --- | ---: | ---: |
| File metadata requests | 88 | 372416 |
| Of those, directory-stamp comparisons | 24 | 101568 |
| Opens | 48 | 203136 |
| Descriptor duplicates | 12 | 50784 |
| Filesystem checks | 44 | 186208 |
| Namespace type ioctls | 16 | 67712 |
| Thread ID queries | 8 | 33856 |
| Positional document reads | 4 | 16928 |

These are derived request counts, not directly observed syscall counts. In
particular, the pinned standard library's metadata implementation/fallback
must be audited before replacing its projection. A failed observation can
short-circuit this successful-path inventory. The 24 directory comparisons are
a subset of 88 metadata requests, not an additional category to sum into them.
Their fraction is not the fraction of LocalContext CPU they consume.

The [next protocol](../../../../tools/preparation-ranking-074/next-directory-protocol.json)
is prepared, not executed. It permits only one runtime call-site candidate:
freshly project all five DirectoryStamp fields at `PinnedDirectory::same`.
Constructors/security predicates, file/document stamps, namespace observations,
named opens, FD lifetime and error precedence stay unchanged. Pinned GNU errno,
EINTR and fallback behavior must be checked before choosing the implementation.
Field/error/mutation controls precede an eight-process four-arm comparison of
root/proc directory leaves, the whole namespace bracket and whole LocalContext.
All baseline controls must fit [0.97,1.03] and every candidate/reference upper
bound must be at most 0.98. Integration and its installed-image cohort remain
separate gates; no first failed preparation may be retried into a pass.

## Checks and retained result

Twelve new Python controls pass, including 128 seeded acyclic graph expansions
with seed `0x074ca117`, count/time conservation canaries and changed source/pin/
host/scope refusals. An independent staged-blob verifier recomputes the result
from hash-addressed inputs, the saved reader and saved graph, preserving the
original input bytes rather than trusting a summary.

The [evidence index](local-runs/preparation-ranking-20261025/index.json)
contains 42 members/36 objects, SHA-256
`7f0db4bc76bd616232c2824e35cc22ef1d910b0a093c3e50eaf09b46fbc68e70`.
It retains the exact original profile/dispatch/manifest/frame inputs, eight
source snapshots, tool/protocol/graph, analysis and successful local controls.
No Rust/runtime/dependency code changes on this step. Native package tests are
not rerun or retrospectively relabeled as this analysis's checks.

The [final checks packet](local-runs/preparation-ranking-checks-20261025/index.json)
retains staged review, successful doc-check and the local non-promotable
performance-contract check: fourteen members/twelve objects, index SHA-256
`7994a31364088d8a3b11589b2ba72a117d2e7bc67aa85e1be9c98d5d1fdc1ffd`.
The full package/security gates remain open; these focused checks do not
authorize a push, qualification or release.

The profile remains a failed attribution-only attempt: enrollment expires at
age 10005376028 ns, with placement, fixed-slice and consuming unobserved. Its
separate first full functional attempt remains failed at age 10057459559 ns,
zero cases complete. The two rejected wrapper experiments are not repeated.
All live checks, frequencies, error order and 300/60/10 windows remain intact.
There is no new server action, clock/service/AppArmor change, installation or
product workload. Original delivery, all-writer retirement, completion/later
cells and real enrollment remain open. Qualification stays prepared-not-authorized,
C74 unresolved and expensive runs disabled; manifest SHA-256 is unchanged:
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
