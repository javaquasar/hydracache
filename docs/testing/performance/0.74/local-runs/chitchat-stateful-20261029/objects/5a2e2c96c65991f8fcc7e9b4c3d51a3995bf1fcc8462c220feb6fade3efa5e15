# Q: retain speculative MTU append work

Start from 9c99852db5d317c02f8163f58a2dc7eb73ca0f0b. The operator's continuation
after the C-cost decline authorizes this next isolated MTU-repair candidate.
Preserve B/M/S/D/P evidence and Root's unchanged dependencies. One hypothesis:
consume budget checking and append together, retaining actual encoded blocks on
acceptance instead of recompressing a discarded prediction.

Q keeps the old-estimate rejection floor, so no additional admissions appear.
It serializes once into the existing pending Vec and compresses only full blocks
which an accepted append needs anyway. If their encoded lengths plus a raw upper
bound for the final pending block fit, commit without compressing that tail.
Otherwise compute the exact tail once and retain its compressed prefix in the
existing scratch until finish. A later real append invalidates that cache; a
rejected append preserves it. No shared context/pool, allocator/feature or public
API change. DeltaBuilder application keeps the original ordering after append.

Checkpoint pending/output lengths, scratch length and optional tail metadata.
Before overwriting a previously cached tail, back its meaningful bytes up in
the output tail; remove that backup on acceptance. Refusal/unwind restores the
original pending prefix, output and cached encoding. Capacity can grow and must
be observed, not called pure. Old-budget rejection precedes serialization and
all buffer changes. Existing public append/finish preserve canonical block order.

Freeze tests before implementation: original 239 admission/wire cases and seed,
all old serialize/delta/state/wide/malformed/oversize controls, plus actual Q
controls for full/partial blocks, exact-tail fallback, continued appends, rejection
after effects, panic and cached-state restoration. Deterministic stateful/property
sequences compare actual decisions/wire with the separate immutable S oracle.
The previous fits/count helpers remain explicitly test-only references, not Q
execution. Unsupported item limits, minimum MTU and original assertions stay exact.

First record new regression failures on the unchanged P control through a test-only
adapter. Then implement Q. Use matched release B/Q/Q/B with the qualified native
C observer/object and fixed payloads, two observations/case/process. Controls
must precede the first cohort. Require all 239 original admissions/wires, exact
within/cross-image repetition, and no per-case increase in Rust logical/physical
or native C allocation/reallocation/requested/peak fields. Inspect extra stateful
cases independently; single-item success cannot qualify arbitrary cached reuse.

Retain the first negative result and decline only Q if a gate fails. No unchanged
retry-to-pass, corpus/threshold changes or claims from zero balance alone. CPU,
RSS/PSS, retained/cache capacity, public distribution, real consumers and full
Root gates remain separate even after these subgates pass. No release adoption,
server, product workload, installation, service or clock changes occur.

Original live predicates/frequency/order and 300/60/10, tests 5/10/45, package
780/workspace 4800, owned local 1200/native session 1100, four-GiB floors and
five-second poll/retirement remain exact. Source copies, registry, object, flags,
image/catalog and actual body joins must be verified and archived.

## Recorded boundary

The first prepare imported an unrelated `prepare` module through an inherited
search path. The separate r1 context exposed a missing profile in the explicit
child environment. Both first refusals remain; r2 uses an exact module path and
explicit child profile. No measurement ran in either failed context.

P's first seven-test baseline has four passes and the three expected failures:
duplicate compression, absent cached tail, and cheap-branch panic restoration.
Only then does Q preparation derive its consuming implementation. Q passes all
72 selected serialize/delta/oversize/state tests (128 total catalog entries).
Both GNU 1.94 all-target/all-feature check and strict Clippy peers pass.

The one B/Q/Q/B cohort preserves all 239 admissions/wires and has no differences
in logical Rust, tagged physical Rust, or native C allocation fields. This is
allocation-subgate-only; MSRV, Windows, CPU, RSS and stateful retained-capacity
qualification remain open. Six offline accounting/source controls pass, including
independent faults in all twelve predeclared dominance metrics. Root is unchanged.

Exclusive invocations are `python -B tools/chitchat-commit-074/launch.py P PHASE`
for prepare/format/stage/c-control/build-P/regression-P, followed by Q prepare/
format/stage/c-control/build-Q/focused-Q/build-B/control-B/control-Q/cohort-1..4/
check-B/check-Q/clippy-B/clippy-Q. Completed labels cannot be rerun or overwritten.
Final offline checks use `gates.py controls|format|performance-contract|doc-check`.
Analysis uses explicit `HC074_COMMIT_PROFILE=Q`; `retain.py` seals first contexts.
The current source derives dispatch from byte-pinned reviewed predecessor tools;
it never executes a tool read from the evidence object store.
