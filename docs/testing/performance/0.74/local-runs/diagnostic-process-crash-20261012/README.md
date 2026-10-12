# Local process interruption proof

P0 validates actual loss of an owned diagnostic journal writer, not product
execution or qualification. Baseline is `ceee4bc7b5b45a7dcb7cc0f018a8e74bb9fbbea8`;
preregistration is `29c002b5a6f1cb12de9967cc60fbf6d4c29db651`.
Implementation is `921050e3ab8be41b136ada1b3a6e6bf30a950693`, tree
`7082a3a93419d076929a978a69384ccd3dee9602`. See the
[intermediate plan](../../diagnostic-server-readiness-intermediate-plan-20261012.md).

## Actual process results

Each of three serial repeats passes all three manual proofs: nine Reserved
cases, fifteen consumption cases and two uninterrupted controls. That is
24 actual SIGKILL cases, two controls and 52 fresh read-only process observations
per repeat, or 72 kills, six controls and 156 fresh observations across repeats.
The checkpoint schedule is seeded `0x074d`; each test invocation uses one thread.

Parent owns every temporary storage/authority/account root and exact process
handle. Readiness is a logical checkpoint with original lock contention. Parent
confirms signal 9 and pins the original direct helper through a pidfd before
death. Helper termination and original lock release follow death/drop. Each
case checks the expected empty, partial, full or sealed document/commit shape,
then compares retained bytes and device/inode/owner/group/mode/link/length/
mtime/ctime fields before death and after two fresh process inspections.
Read-only restarts neither append refusal nor repair, adopt or replay a writer.

After API return the successful guard exists only in the original process.
SIGKILL loses that runtime acknowledgement. Even a complete canonical pair
remains retained context and refuses fresh empty reservation. The before-create
case leaves no durable mutation and permits empty-fence inspection; it does not
establish global nonce replay protection before the first record. P1 must close
that earlier lifecycle boundary. These checks do not exercise a production IPC
acknowledgement transport, power loss, reboot or actual root-host enrollment.

The first process run fails three tests because libtest prefixes readiness with
its test name. A strict start-of-line parser times out despite the child barrier.
The correction accepts only exact libtest framing, rejects ambiguous/malformed
messages and adds two fast parser regressions. The Clippy red capture then
requires a simpler equivalent boolean, corrected without suppression. Captures
03–07 remain provisional; final repeats use the final source. The planned
08 MSRV command was not executed after lint failure; no capture is fabricated.

## Regression and reproducibility

Linux scoped regression passes **599** cases and reports **four ignored**:
the existing real systemd-bus case and three P0 manual proofs, which are executed
separately above. Linux compile-fail doctests pass **38**. Native Windows passes
**196** portable cases; Linux-gated targets report zero there, not coverage.
Root contracts/registry/governance pass **161**. Current/MSRV all-target/all-feature
checks, warnings-denied Clippy/Rustdoc, scoped formatting, doc-check and mdbook
pass. Functional test durations are not performance measurements.

The [manifest](manifest.json) binds 30 raw captures, exact commands and exits,
source/tree/ancestry, named tests, manual dispatch gates, all unchanged production
writers and frozen qualification inputs. Source hashes use Git repository bytes;
raw logs are never newline-normalized. Run the read-only verifier:

```powershell
C:/Python313/python.exe docs/testing/performance/0.74/local-runs/diagnostic-process-crash-20261012/verify-packet.py
```

Three ignored proof gates are registered for the existing Linux manual
gated-proof dispatch. The ordinary suite includes parser checks and a no-op
child-helper entry, but does not perform SIGKILL automatically. New path selectors
and helper construction are crate-test-only. The original state/intent writers,
encoders and durable formats are unchanged.

## Remaining boundaries

P0 local process-loss proof is complete. P1 fixed state and cells two through
four, P2 trusted kernel peer/time and real external issuer/pins, P3 production
output/start/watchdog/revocation/owned cgroup cleanup, and P4/P5 reviewed host
installation and nonproduct lifecycle rehearsal remain open. No server, product
workload, six-hour/24-hour qualification, numerical claim or ship admission is
enabled. Local helper termination is not production empty-cgroup proof.

The frozen qualification manifest remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
All eight inputs are unchanged. Dry-run still refuses the inherited plan digest
mismatch; ship admission still refuses. No thresholds or frozen 0.73 artifact
were changed.
