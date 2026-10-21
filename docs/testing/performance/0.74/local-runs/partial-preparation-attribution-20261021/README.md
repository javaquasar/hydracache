# Offline attribution of refused preparation prefixes

`facts.json` reconstructs the two already sealed refused prefixes from
`single-session-rounds-bound-20261021`, evidence commit
`06b55d5e136f8a4289aa3b4c7759bb0ea721a314`, inventory SHA256
`91a6e589235f2552ae9335680f47d4a20a418efeefcc3bc9fd2a05ba96f12afe`.
The offline method is frozen at `42eefbec2242b8123799b2b50fea3ed338ccdf2e`.
No timing profile, workload, runtime/observer or admission input changes here.

Each column below is one individual refused prefix, not a cohort range or a
compiler comparison. `outputs` completes within each otherwise failed attempt.

| Recorded component | GNU 1.94 | MSRV 1.88 |
| --- | ---: | ---: |
| Outputs wall | 7.461490 s | 6.178922 s |
| Outputs parent process CPU | 7.360031 s | 6.147695 s |
| Sum of eight outputs step walls | 7.417084 s | 6.152511 s |
| Outputs wall outside these steps | 44.406 ms | 26.411 ms |
| ReservedLive wall | 6.348382 s | 5.296315 s |
| ReservedLive share of eight-step sum | 85.5913% | 86.0838% |

All eight output steps complete 46 times. Parent output authentication calls
remain 782, of which 736 occur in ReservedLive. The parent kernel call signature
is identical in these two output subphases and reconciles to their steps. Each
step's kernel root/edge call counts and child wall subtraction balance. The
original exits stay 101; GNU's failed Causal step joins the expiration at
10,175,057,064 ns, while MSRV's failed FinalClocks step joins 10,006,497,560 ns.
Later phases/samples are absent, not inferred from complete output subphases.

## Inside the dominant step

ReservedLive kernel wall outside its instrumented children totals
3.921874/3.245885 s (GNU/MSRV). Boot plus Namespace account for
1.988862/1.719685 s within that same kernel accounting. Their call counts are
129,904 and 259,808 in each prefix's ReservedLive output step. These are wall
spans, including their instrumentation, not syscall-exclusive CPU or evidence
that reads can safely be cached or removed.

ReservedLive's wall outside the instrumented kernel spans is
2.426508/2.050431 s. ActiveAuthority separately records 0.993633/0.834129 s
outside its live children, at 14,812 calls. Authentication records
81.994/82.488 ms within ReservedLive. The live, authentication and kernel
observers overlap: do not add these numbers together, subtract them again from
the residual or assign the residual entirely to authority/allocations.

The 44.406/26.411 ms outside output round steps is an aggregate, not an fsync
measurement. Starting effects retain all surrounding rounds; this packet does
not isolate filesystem effects. Source `consuming_held_cost.rs` also stops
phase wall/CPU before report aggregation and JSON emission. Their cost and
other inter-phase work are outside these phase measurements but can still
consume admission lifetime. Phase sums must not be presented as issued age.
Child CPU is excluded; no observer overhead is subtracted or assumed negligible.

## Decision and next control

The observations select ReservedLive as the next cost-control target, rather
than blaming the later owned-root/enrollment failure site or weakening crypto,
fsync or lease limits. First isolate observer bookkeeping cost at the same
numeric call/edge topology, then separate the per-call Boot/Namespace and
authority work. Such a control would be synthetic and non-admitting; its result
could not replace fresh live validation or a healthy complete preparation.
No new optimization is accepted from these partial prefixes.

The analyzer has 18 passing controls on Windows and Linux, including changed
counts/edges/times, missing/reordered rows, false completion, scope relaxation,
boot/age/site changes, output overwrite and retained-recipe reconstruction.
Five additional reviewer mutations reject numeric-for-boolean gate substitution,
invented completion/new profiles, runtime change and overhead subtraction.
All twenty-two preceding packets verify. Original windows remain 300/60/10;
no new profile, service, clock, idle-policy, server or product action occurs.
Evidence integrity can pass while complete-cohort/native admission remain false.

After final checks use `review.ps1 -Mode Review` in this directory. It reconstructs
facts from the retained analyzer copy and immutable input, verifies source/log
bindings and the derived packet inventory. Never modify or reseal the input.
