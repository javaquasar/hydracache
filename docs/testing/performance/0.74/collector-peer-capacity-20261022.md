# Collector peer primary capacity candidate

## Decision and implementation

Keep the isolated allocation candidate from the
[preregistered experiment](collector-peer-capacity-plan-20261022.md), compared
with baseline `da44628a85779977d5d1cb2f32e324239165eeff`. GNU 1.94 and MSRV 1.88
pure controls confirm removal of vector growth requests. This is not a full
preparation timing or server-admission result.

Candidate source: `6e3cea6313b492d50a58eb2986449045a805978f`;
preregistration: `bebd4a1c`.

The private peer helper now reserves supplementary.len()+1 u32 slots, extends
from the original slice and keeps the same primary push, sort_unstable, dedup,
uid and owned DiagnosticPeer. It does not skip push when primary is already
present, add an input limit, change representation or reuse a previous peer.
The test-only adapter forwards directly to that actual private helper.

The complete collector-input wrapper is unchanged. Sender-before, origin sample,
hold, peer construction, sender-after, final unix-seconds extraction and refusal
latching keep their order. Fresh-check counts/order, original 300/60/10-second
windows and upstream admission limits remain. No new unsafe or live capability
is introduced. For valid u32 slices length+1 fits usize; ordinary Vec allocation/
layout failure is still infallible allocation, not a recoverable admission code.

## Matched allocation comparison

The candidate and unchanged baseline use the same 31 cases, 256 constructions
each: empty, primary absent/present and duplicate-primary shapes at the original
lengths. Current baseline rows and stage rows match their earlier raw seal.
Both compilers report:

| Per compiler, 7936 main constructions | Baseline | Candidate |
| --- | ---: | ---: |
| Allocations | 7936 | 7936 |
| Reallocations | 7680 | 0 |
| Requested bytes | 1798144 | 627712 |

The independent one-construction stage controls for each case conserve the
whole-operation counters. In the candidate, the initial reserve owns one request;
copy, push and sort/dedup request no additional allocation. These 31 stage
constructions per arm are not pooled with the main corpus.

After push/dedup the candidate capacity remains input length+1: 33 instead of
64 at input length 32, and one instead of four for empty input. It does not
shrink to the deduplicated length. An absent primary with 32 supplementary
groups still yields all 33 canonical groups. Generic inputs above the upstream
limit test leaf compatibility, not live admission.

Counters are Rust allocator requests, not usable allocation size, RSS, retained
memory or CPU. Copies, sorting, branching and allocation still cost work. The
synthetic mixed corpus is not a production frequency distribution, and old
trace counts cannot turn these savings into current preparation headroom.

## Semantic and source controls

Five candidate controls exercise the actual helper, matched stage requests,
the baseline seed `0x074c0ee1` with 1024 canonical-union cases, boundaries and
owned output, and both synthetic wrapper arms over the same 32 combinations
of initial refusal and sender/sample/hold faults. Values are compared with the
complete frozen peer body and an independent BTreeSet oracle. Input nonmutation,
uid, duplicates, integer extremes and no truncation remain.

Both wrapper arms keep first-error priority, peer work between sender checks,
time extraction only after the closing check, latching and refused reentry.
Source token comparison recovers the complete original wrapper body. Fake field
receivers are not real liveness, clock, filesystem or writer-cessation proofs.

Whole-file guards reverse exactly the allocation expression and private hooks
in both changed old supervisor files. Historical canaries require the entire
earlier collector source before accepting the successor; they do not exempt
the filename. The baseline controls and admission source remain unchanged.
Mutation controls reject changed union operations, sender/hold checks, latching,
time extraction, fixed capacities or broadened test visibility.

## First host refusal and checks

The first Linux cohort passes all pure controls. Its host canary refuses its
own expected adapter token string: the expected text omitted the doc attribute
and used different punctuation spacing. The original canary and refusal remain.
Only that exact expected string changes in the canary-fixed host cohort; the
adapter, candidate and live predicates do not change. Linux allocation controls
are not replayed to repair a host source-text assertion.

Final focused checks pass: 37 source canaries; 21 pure tests per Linux compiler;
affected all-target check and strict all-feature clippy on host/GNU/MSRV;
workspace format; doc-check; unchanged 186-gate registration; local
non-promotable performance contract. This is not full milestone/ship verification.

The [packet](local-runs/collector-peer-capacity-20261022/summary.json) retains
the first outcomes, corrected host outcome, initial/final sources and method
snapshots. The [offline reviewer](../../../../tools/collector-peer-capacity-074/review.ps1)
checks stage conservation, baseline identity, ten earlier raw seals and all
442 members without modifying their frozen source-pinned reviewers or the
qualification manifest.

The packet has 49 members plus its inventory; raw inventory SHA-256:
`846b5e7d6d07a47bd03c64be0a207b24b197b39ceddb25e4fc3b06cd049b05ae`.

## Remaining boundary

Both first Causal(Time) ages remain 10042390247/10010835739 ns. No ignored/manual
fixture, original-owner/live profile, clock/service change, server/SSH/Actions,
installation, supervisor or product workload is run. Full material revalidation,
cleanup/EOF/fence, authenticated delivery/all-writer cessation, SIGKILL/lost-ACK/
reconnect and server gates remain independent and closed. A later timing
comparison needs separate admission and must preserve those first refusals.

Next reconcile the three leaf candidates retained since the remaining-leaf
audit with its unresolved offline cost inventory. Their allocator controls
cannot establish a combined speedup or substitute for the still-missing
full-path gate.
