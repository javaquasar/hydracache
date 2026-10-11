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
