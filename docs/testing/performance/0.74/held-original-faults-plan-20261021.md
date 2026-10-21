# Original held holder local fault checks

Complete the four remaining cases of the unchanged original consuming-holder
fixture on GNU 1.94 and MSRV 1.88, at the directory encoding runtime candidate
`3d3a2180c6b54ea9156ae2bac609f36966359908`. The preceding six-sample healthy
cohort remains separately sealed; it is not rerun or replaced here.

| Case | Original fixture assertion |
| --- | --- |
| revalidate | Repeat live validation, preserve original origins and active timer |
| role-before | Change the held role object's mode before consuming join; require Roles refusal and retained timer/fence |
| role-after | Change the held role object's mode after join; require Roles refusal, then sticky Refused |
| trust-after | Drift signed trust after join; require Trust refusal; restoring the document must still return Refused |

Each case asserts the fence is busy, streams lack EOF while custody is held,
and after holder/failure drop the synthetic attempt cgroup is empty, every
pinned custodian is terminal, streams reach EOF and the common fence remains
busy until the outer owner drops. These are synthetic holder/custodian tests,
not authenticated manager delivery or cessation of arbitrary production writers.

Freeze the method before entry. Schedule one invocation per case in the table's
order on GNU, then the same order on MSRV. Each invocation is a distinct fixture,
not a retry or continuation. Use fresh ordinary-user offline/locked default-feature
release image receipts, the same nine-file synthetic bundle, seed 0x074c051e,
existing cost observers, test function and original 300/60/10-second windows.
No runtime/Rust harness edits or new logging calls inside validation are allowed.

Enclose metadata, all profiles and postflight in one root WSL parent. Reuse the
original process-group capture, 1200-second child timeout, five-second grace,
1210-second child/7500-second session/7530-second outer containment and 32 MiB
log bounds. Remaining budget can refuse dispatch; it never extends admission.
A nonzero harness exit stops that compiler lane; context drift stops everything.
No automatic retries, overwritten packet, changed clock/service/idle policy,
server, supervisor, installation, product or qualification workload is allowed.
Root is confined to the original unique local synthetic namespace/cgroup fixture.

Expected injected refusal is a passing original Rust assertion and exit zero,
not an arbitrary panic, Time refusal or wrapper timeout. Offline review must
bind the exact case/seed/footer and raw exit, retain original first refusals,
check ordered phase prefixes and every round's eight-step kernel conservation,
and bind source, method, default images, bundle, qualification, boot, parent
PID/start ticks and configuration. Complete outputs keep their original signature.
Successful preparation is not full SIGKILL/lost-ACK, reconnect, enrollment or
server admission; those remain independent gates. No numerical speedup follows.

Run deterministic new scheduler/reader falsification controls and the unchanged
runner controls on Windows and Linux before commit/launch. Preserve any refused
prefix; seal the distinct `held-original-faults-20261021` packet only after
independent offline reconstruction and local repository checks.
