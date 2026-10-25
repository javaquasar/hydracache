# Preservation sweep call counts and object locality

The retained successful `diagnostic_artifact_fixture_074` target necessarily
performs at least 3116 direct `git show` helper launches and 3116 current-input
reads in seven preservation loops. Its twenty cases took 108.31 seconds in the
earlier package replay. Source/verdict reasoning establishes call counts, not
their elapsed-time share or the cause of the package's 780-second timeout.

## Proof from the original source and verdicts

`sweep_review.py` reads the fixed transport/package packet with index SHA256
`e9b8a923723360bce644c9de558113bf084630d1c4a72916dd0bfd3d418fc553`.
The reviewed source is the exact 23589-byte file from
`3e9bcc06ebce3439495200162cb0c1900b8eda53`, SHA256
`5819f89ec58a899e03d7c0f14c6af91f4be6248a26f8578186cfc47b5900a805`.
It is not substituted with the current checkout's source or a repaired transcript.

Each manually reviewed loop asserts its historical list length, traverses all
lines plus an explicit tail and has exactly one current read and one `git show`
in unconditional `assert_eq!` operands. There is no early break, filter or retry.
The retained syntax inventory distinguishes real test/call items from quoted
source text. The helper calls `Command::new("git").output()` on each invocation;
the selected `show` branch does not perform its `ls-files` inventory projection.
All seven cases have passing verdicts in a completed, unfiltered target.

| Preservation stage | Runtime paths | Tail paths | Required loop iterations |
| --- | ---: | ---: | ---: |
| actual consuming fixture | 446 | 4 | 450 |
| immutable fixture | 437 | 5 | 442 |
| launcher | 438 | 4 | 442 |
| role files | 440 | 4 | 444 |
| role binding | 441 | 4 | 445 |
| held trust | 442 | 4 | 446 |
| consuming held join | 443 | 4 | 447 |
| Total | 3087 | 29 | 3116 |

This is a fixed-source manual proof with machine-checked identity, anchors,
syntax evidence and verdict joins, not a general Rust control-flow analyzer or
dynamic process trace. It excludes additional helper work, the other thirteen
cases, nested historical reads and fresh filesystem/index validation. The
selected target's duration cannot be divided among these calls without further
attribution. Multiplying 3116 by an earlier, differently implemented transport
microbenchmark would not establish that attribution, especially with four cases
running concurrently.

## Different lookup keys can address the same objects

A separate bounded read-only collection reads seven literal full-commit trees
using `git --no-replace-objects ... ls-tree -r --full-tree -z`. It does not read
blob contents, run tests, fetch missing objects, change refs/configuration or
operate a server/service/clock. The offline `tree_locality.py` reader verifies
record bounds, regular blob modes, paths, counts, commands and raw hashes.

The selected sweeps contain 3116 distinct `(revision, path)` pairs but only 465
different blob OIDs. Of those OIDs, 435 appear in all seven trees; there are
2651 references beyond each object's first reference. An initially empty exact
revision/path cache fed **only these seven loops' requests** therefore has zero
hits. Other cases, nested helpers, concurrent warming and an already-populated
cache are outside that model; the actual package cache hit rate is not measured.
Object overlap does not prove that object-level reuse is safe.

These are newly captured local no-replace object addresses, not telemetry from
the old package's Git children. The old helper's replacement-ref/configuration
state is not retroactively attested. SHA1 Git addresses here identify tree
entries; they are not production authority pins, SHA256 content proofs, measured
bytes saved or an accepted cache. Any later object/persistent-process proposal
must independently preserve source ownership, conditional reads, trusted pins,
live ref/config observations, ABA/fallback refusals, errors and deadlines.

## Prepared next attribution

`helper-attribution-protocol.toml` fixes a separate diagnostic scope before its
implementation or dispatch: all original twenty cases, four case threads, the
unchanged 780-second bound, original helper graph/assertions, exact-source
observer erasure and a separate unobserved reference. It distinguishes whole
Git-helper work, `Command::output` including spawn/wait, decode/conditional
projection and whole current-input read. Spawn versus wait and child CPU cannot
be inferred by splitting an opaque `output()` call after the fact.

The future diagnostic must reconcile the complete case catalog and counts,
retain its first outcome and source/executable identity, and measure observer
overhead separately. It does not admit a key cache, object cache, persistent Git
child or product candidate. No diagnostic target or package replay is launched
by this protocol. The full package remains incomplete: 111/135 completed targets,
the unfinished `performance_contract_074` and 23 unstarted targets are not closed
by this successful earlier target or the new offline checks.

## Checks and retained refusals

Thirty-one Python controls pass, including the nineteen earlier package-reader
controls and twelve new sweep/locality/protocol controls. They reject changed
source/counts/identity, missing verdicts, changed syntax evidence, malformed or
aliased trees, incorrect command/exit/hash records and Boolean/integer confusion.
Syntax and indentation checks, doc-check and local non-promotable structural
performance-contract checks pass. No Rust implementation changed in this step.

The first new command-shape test has an incorrect argument index; its failure
and exact test source remain, followed by the corrected oracle without a command
change. The first tree preflight refuses inherited `GIT_PAGER` before executing
any tree command. That refusal remains; a separately named child environment
removes only `GIT_PAGER` and retains the original strict override predicate.
All seven reads then succeed once. Neither refusal is relabelled as success.

The [evidence packet](local-runs/preservation-sweeps-20261025/index.json) retains
the derivation, original source/syntax/verdict inputs, both first refusals, seven
tree captures, exact reader/coordinator sources, checks and independent staged
readback. The article and continuation plan distinguish call/object facts from
unmeasured time and safety hypotheses. No normalizer, original live check,
frequency/order, 300/60/10 origin/window, dependency, qualification manifest or
frozen 0.73 artifact changes.
