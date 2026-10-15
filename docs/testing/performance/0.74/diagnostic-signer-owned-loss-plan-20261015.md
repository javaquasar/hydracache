# Original owner loss at two existing stops

Baseline `ce5fd37dc13e38e306eeec94a249140422d900da`. Keep the normal owner,
child setup and fixed CLI unchanged. Add only cfg(test) process observations of
the existing private Owner constructor and first-exec transition, using the same
owned fixed /bin/true image as the previous unit tests. This is not a complete
normal CLI loss proof and does not authenticate root/runtime or a signer.

A dedicated reexecuted test monitor becomes a subreaper; the ordinary test
runner's subreaper setting must remain unchanged. A separate owner uses the
unchanged Owner, reports its original child and stops itself after either
Owner::create or Owner::first. At that logical barrier the monitor checks the
owner's exact wait stop, its sole child, PPid, TracerPid and retained child pidfd.
It then SIGKILLs only the owner through the owner's retained pidfd. Require exact
owner SIGKILL, original child pidfd readiness, adopted waitid(P_PIDFD) SIGKILL
without core, reap and subsequent ECHILD. Do not signal the child on the passing
loss path. At the initial cut EXITKILL has not yet been installed; after exec
both protections are present and the test cannot attribute the kill to one alone.

Each cut has a healthy control: resume the owner, let unchanged Owner::drop
terminate and reap its own child, require owner exit0 and monitor ECHILD. Keep
an unrelated live dummy original process throughout each observation and verify
its pidfd remains unreadable before explicit cleanup. Test harness cleanup may
signal retained originals on failure; this is not successful loss evidence.
Use five-second wait bounds, bounded fixed reports, seed 0x0740c07a for report
mutation controls and no sleeps as correctness evidence.

Capture the nine-test baseline and an expected-red missing matrix, then implement
the tests. Repeat on Linux current and MSRV, run strict lint/all-target checks,
normal CLI/child and previous loss regressions, format/root-contract/doc/local
contract gates and expected ship refusal. Retain raw captures and update the
article with the narrow result. Commit preregistration, tests and evidence
separately. Do not change normal code, dependencies, frozen inputs, workflow,
credentials, signing, server/GitHub state, workload, admission or push.

## Harness correction after the first negative run

The first implementation used self-SIGSTOP as the owner acknowledgement. The
exec case observed an already-zombie child before injecting owner loss. A
successful kill(SIGSTOP) call is not an instruction-level acknowledgement that
the calling thread has stopped; the test could continue into its loss-path
assertion and cleanup. Preserve both the failed run and the status diagnostic.
Replace that harness gate with a private bidirectional socket: after reporting,
the owner can only wait for a grant. Only healthy controls receive the grant.
The child remains at the same original ptrace stop throughout. The normal owner
and its child code are unchanged. Also let the monitor own the temporary image
directory, and arm role-parent loss cleanup so a failed observer leaves no live
acknowledgement-gated owner. This is a harness correction, not a weaker expected
terminal status or a new normal launch hook.
