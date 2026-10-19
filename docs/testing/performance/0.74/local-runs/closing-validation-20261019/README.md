# Closing validation cost and time refusal

Inherits `docs/RULES.md`. Local WSL Ubuntu only, synthetic issuers, copied signed
bundle data and harmless dummy writers. No server, installation, supervisor
operation, original message send, product load or qualification.

The [plan](../../closing-validation-plan-20261019.md) fixes the attempt counts and
the [design](../../closing-validation-design.md) explains the missing time site.
Baseline is `eefcd449`. Diagnostic source is `8ad08237`; selected source and
reproduction recipes are `659855a3c83f5185cd290ab26fdd43122e7cb396`. All runtime/test
source files are identical between those two commits. The earlier
[owned-root packet](../owned-root-manager-20261019/README.md) remains closed with
its original partial MSRV outcome.

## Source and build guards

Three new contracts verify exact hook removal against all 418 previous source
bodies, reject missing/duplicate hooks and original predicate drift, and prohibit
new clocks or execution in the raw observer. The full suite passes 303 checks.
All live checks, call frequencies, 46 effect brackets, original fence/alarm and
300/60/10 origins and bounds are retained. Units exercise exact age expiry,
backward/zero inputs, bounded storage, thread isolation, nesting and unwind.

The first GNU/MSRV build recipes selected `closing_`, ran six passing tests but
correctly refused the declared count of four. Two unrelated existing names
matched that substring. The qualified selector `::closing::closing_` selects
the intended four; expected counts and test bodies were not loosened. Preliminary
logs remain in `gnu194/` and `gnu188/`. Only the `*-confirmed` receipts supply
selected build identity. Each also passes strict lint, two ownership units,
eighteen origin and nine guardian regressions.

GNU 1.94 ordinary image SHA256:
`527eb954a7f28b492796262f7340200cef8f109f9408c103618ed9dcd5398dde`.
MSRV 1.88 ordinary image SHA256:
`8caf881c93fc93f9ba870d2b64492016da079fec6f300bcdbbbb00905337f440`.
No allocation profiler feature is enabled. Runtime diagnostic modes are explicit
and separate: `cost` enables spans; `facts` does not. `baseline-images.log` retains
the earlier immutable images before the compiler caches were rebuilt.

## Six preregistered cost attempts

All six attempts are independent fresh original contexts. A time refusal is an
observed negative, not a successful creator fault. The cost recipe never admits
product work and always reports `functional_gate_passed=false`. Its fixed sample
count does not depend on obtaining green. Failed cleanup would stop further work.

| Compiler and sample | Closing result | Instrumented wall ms | Last custody age ns | Authentication calls |
| --- | --- | ---: | ---: | ---: |
| GNU 1.94 0 | custody expiry | 261.749189 | 10,003,292,291 | 34 |
| GNU 1.94 1 | complete | 243.204056 | 8,887,841,044 | 34 |
| GNU 1.94 2 | causal-window expiry | 117.334169 | 9,953,952,739 | 17 |
| MSRV 1.88 0 | complete | 236.779278 | 8,705,763,171 | 34 |
| MSRV 1.88 1 | complete | 303.410849 | 9,347,358,508 | 34 |
| MSRV 1.88 2 | complete | 218.287104 | 9,058,489,236 | 34 |

`cost-analysis.json` retains exact unrounded values. Failed partial rounds cannot
be averaged with complete rounds to claim lower cost or compiler speedup. The
complete rounds all perform 34 authentication/signature checks, 6,688 Boot and
13,376 Namespace observations; the raw reports identify the overlapping spans.
On MSRV authentication occupies 2.457–2.742 ms of instrumented wall time, while
kernel-context inclusive spans occupy 106.285–152.244 ms. Boot and Namespace
inclusive spans are nested within that work; do not add these columns as
independent CPU or IO costs.

## Exact refusal mechanism

In GNU sample 0 the original custody site
`diagnostic_original_custody_join.rs:121` sees issued BOOTTIME 825,973,503,312
and now 835,976,795,603 ns. Their difference is 10,003,292,291 ns, so the original
exclusive ten-second age predicate refuses. The last existing final-clock sample
was still within the window at 9,985,119,124 ns. Its actual issued/now boot IDs
match. The later raw custody sample has no separately sampled boot ID; none is
fabricated. This reproduces the same `Challenge(Time)` error path seen in the
historical MSRV closing frame without pretending to recover its missing tuple.

In GNU sample 2 the last custody sample remains below ten seconds, but the later
causal-window site itself observes 10,068,131,423 ns and refuses. The causal
observer records this first refusal with matching actual boot IDs. Thus an
earlier healthy observation is not permission to skip a later guard. The
failures precede ready/fault injection; they prove neither creator SIGKILL nor
writer cessation. Both failures and all successful cost attempts are retained.

## Functional cases are a separate cohort

After interpreting the cost series, `remaining/` runs facts only: the four missing
MSRV cases second-dead, first-stopped, second-stopped and outside-alias, then
owner and first-dead as same-source regressions. Each is executed once in a fresh
context. A pre-fault refusal stops the cohort; it cannot manufacture a positive
receipt. `remaining-analysis.json` and the raw carrier/creator logs distinguish
actual SIGKILL proofs from preparation observations. This cohort is not a new
two-compiler matrix, original execution or production enrollment.

All four missing cases pass, as does the owner regression. These five actual
creator-SIGKILL attempts each stop all three dummy writers before original
expiry, shed original sink roles, drain both readers and retain the fence. The
outside alias blocks stdout EOF and can still write until the harness closes
it; that is not claimed to be arbitrary external-FD revocation.

The last regression, first-dead, refuses during enrollment before closing or
fault injection. Its original window observation is issued 1,160,431,594,530
and now 1,170,456,291,564 ns, giving age 10,024,697,034 ns, with matching actual
boot IDs. No closing report or ready capsule exists for this case. The cohort
correctly has no positive `remaining/receipt.json`; `remaining/outcome.json` is
analysis of a partial result. The fresh complete MSRV gate is not passed and
is not retried. Unlike the old unobserved refusal, this new upstream expiry has
an exact tuple. Reducing closing cost alone would not prevent this earlier
failure; preparation and enrollment also consume the original challenge budget.

## Reproduction and evidence integrity

Use fresh labels with `build.ps1`, then `run.ps1 -Stage cost`, inspect with
`analyze.ps1 -Stage cost`, and only then `run.ps1 -Stage remaining`. Recipes pin
source, compiler/image and recipe digests. Each stage stops only the previously
authorized local WSL clock service, settles/probes clocks before admission and
restores/verifies the service in `finally`. No clock is manually set or original
timer rearmed. All compilation and source contracts finish before timed attempts.

The verifier checks raw inventory hashes, actual cases, units, comparison sites,
original issuance binding, unchanged signed data and frozen qualification
manifest. It rejects sixteen mutations, including false historical recovery,
changed deadlines, false MSRV promotion and product/cross-source promotion. Do not reseal a closed
packet. No CPU/op, allocations/op, RSS, product goodput or speedup is claimed.
