# Original signed mount and bounded task composition

## Preregistered scope

Baseline: `4c3f7d463d8886bcd607b45e4dcd5e81d8bb7d20`. This local W11
slice joins the existing original signed mount reader to the bounded original-task
reader. It is diagnostic safety work, not an accepted performance candidate.

The hypothesis is that one exclusive borrow can preserve the original process,
signed policy, account files and namespace checks without exposing a replacement
process, task set or numeric policy. Fixed and fixture origins remain distinct.
Opening must observe Signed/OpenTasks/Signed/Tasks/Signed; subsequent checks
observe Signed/Tasks/Signed. OpenTasks includes the primitive's bounded original
observations; no object escapes before the post-open signed checks. Task numeric
assertions are derived privately from the signed policy's exact supplementary
group set, never a primary-group union. Both retained task namespace identities
must match the original signed policy. Existing task budgets remain 128 tasks,
16 directory calls and 65536 directory bytes per scan.

Any first typed error stops the sequence and refuses the owned task guard and the
entire original signed mount/account/context/policy/leader-credential chain. This
survives wrapper drop. Previously refused inner inputs must be recognized before
account IO. A healthy drop must leave the original signed mount usable. No refresh,
reset, conversion, selector or cross-thread transfer is permitted.

## Planned local checks

Record the unchanged baseline and an absent-API expected red, then test exact
orders and seeded first-error schedules, constructor failure, post-open signed
precedence, policy/namespace mapping drift, prior inner refusals, healthy drop,
revocation, real multi-thread membership additions/removals/exit and independent
concurrent readers. Run the supervisor package tests, all-target checks, strict
clippy, MSRV 1.88 checks on Windows and WSL Linux, focused repository contract and
evidence/governance checks, formatting and documentation checks. Repeat the focused
Linux composition tests using separate raw log filenames.

## Proof boundaries

The owned nonroot helper and synthetic signed account fixture are local witnesses,
not a controlled fixed-host positive enrollment or a production issuer. Observed
stable membership is neither an atomic snapshot nor continuous confinement; a
task appearing and disappearing between observations can be missed. Namespace
identity does not prove mount contents, opener credentials or initial host context.
Real foreign namespace changes and nonleader credential mutations remain unproved.

Durable epoch/refusal authority, rollback model, production preparation/start,
watchdog, writer revocation and exact cgroup cleanup remain separate work. No host,
workflow, product workload, qualification input or admission flag is changed here.
No throughput, CPU, allocation or memory claim is authorized by these safety tests.

## Reviewed local results

Implementation `f624df7f9c9da31f18f0ee2841506253df382bc6` (tree
`50c944a2c81f1311232083d858886e5a5d53d928`) closes the local original-signed
composition gap. The [packet](local-runs/diagnostic-signed-tasks-20261011/README.md)
retains 41 exact raw captures, source/tree pins and a read-only verifier.
The unchanged baseline passes 13 signed-mount and 11 task-set tests. The absent
public task-binding API is an expected compile red before implementation.

Eleven new unit tests cover exact opening/later orders, 256 modeled first-error
schedules at seed `0x7632026`, non-transferable typed guards, original healthy
drop, constructor/post-open failure, real additions/removals/exit, exact mapping,
prior inner refusals, revocation/restoration and independent concurrent chains.
They pass in three separate serial repetitions. The existing Linux API test
target adds one fixed/fixture signed-task API check.

The reviewed scoped Linux suite passes 394 tests with one existing ignored; the
Windows supervisor package passes 189 portable tests, not new Linux-task cases.
Root contracts/evidence/governance pass 101/13/23. Supervisor check, strict clippy
and MSRV 1.88 pass on both platforms; xtask strict clippy, Linux strict rustdoc,
formatting, doc-check, local performance contract and book build pass.
Require-ship remains expected red.

The retained initial wrong API path/root test target/book directories and strict
test iterator lint are command/fixture-code findings, not product hypotheses.
Their fixes do not weaken any predicate, change a threshold or produce a numerical
performance claim. These checks are working-tree local safety observations, not a
new full-workspace verifier, immutable candidate receipt or positive host enrollment.
