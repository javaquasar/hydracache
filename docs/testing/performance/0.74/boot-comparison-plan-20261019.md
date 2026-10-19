# Fresh boot comparison allocation reduction

Inherits `docs/RULES.md`. Baseline is `aa84aeb8`; the closing attribution packet
and its incomplete MSRV gate stay sealed. Work is local WSL only, without server
installation, supervisor operations, original delivery or product workload.

Closing attribution counted 6,688 fresh boot observations in a complete round.
The current revalidation constructs a temporary 36-byte String for each fresh
UUID comparison. Test one hypothesis: avoid that temporary ownership when the
caller needs only equality. Keep the owned boot ID and sampled time-point API.
Keep every fresh open, filesystem/object check, positional read, strict UUID
grammar, UTF-8 check, failure site, close-before-decode and trace invocation.
No retained descriptors/results, fewer observations, new timers or changed
300/60/10 origins/bounds; all 46 effect brackets remain unchanged.

First add a frozen revalidation/sample reference and use the existing opt-in
Rust allocation counter. Before implementation, record three paired samples of
1,024 calls for each operation. After implementation, repeat exactly that recipe
on GNU 1.94 and MSRV 1.88. Accept only a repeatable reduction in allocation
requests with identical values, refusal sites and live trace counts. This is not
an assertion of faster preparation, lower RSS, exclusive CPU or product speed.
Keep any negative timing or functional result; never retry until green.

Before the implementation commit, add differential grammar tests for all byte
mutations, lengths and seeded documents, plus real procfs and namespace/object
drift tests. Preserve old source bodies through an exact reversible adapter
which rejects missing/duplicated substitutions and all unrelated predicate drift.
Run formatting, strict lint and source contracts on the complete candidate.

After builds and checks finish, run one fresh facts-only MSRV six-case cohort,
in fixed order: second-dead, first-stopped, second-stopped, outside-alias, owner,
first-dead. Stop on any pre-fault refusal, leaving unexecuted cases explicit.
Use the existing owning external creator-SIGKILL harness, original pidfds,
writer-SIGKILL-before-deadline, EOF and busy-fence assertions unchanged. The local
clock-service stop/restore and pre-attempt settle/probe remain unchanged.
Separate profiled allocation results from this unprofiled functional gate.
Retain exact source, image, recipe and input hashes in a new evidence packet.

Passing these local fixtures does not admit production trust/enrollment,
authenticated original send, durable completion, unlock or server rehearsal.

## Recorded outcome

The [packet](local-runs/boot-comparison-20261019/README.md) confirms three paired
samples per compiler: revalidation changes 2/72 Rust requests/bytes per call
to zero, and sampling changes 5/180 to 1/36. The owned returned boot ID remains.
The decoder stays in its original module after an old mutation guard rejects
its relocation; no old test is weakened. All 306 source contracts pass.

The separate fixed six-case MSRV cohort passes once in facts-only mode, with
actual creator/writer SIGKILL before original bounds, EOF and a held fence.
All original comparisons and deadlines remain intact, clocks are restored,
and historical negatives stay sealed. This confirms a local functional gate
and an allocation reduction separately, not elapsed preparation improvement
or a production/server admission.
