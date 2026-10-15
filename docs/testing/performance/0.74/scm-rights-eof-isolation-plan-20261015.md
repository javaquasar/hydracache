# SCM_RIGHTS EOF fixture isolation

Baseline: `61d664e97337f535b51f191561f6914a56f5b082`. An unchanged-source
Linux Rust1.94 root group passes 68 tests before this experiment. The preceding
packet retains a separate unexpected 46/1 EAGAIN failure in the rejected-rights
unit. Its historical writer owner is unknown; successful repetitions do not
explain or waive that failure.

Hypothesis: sibling unit tests can fork while an EOF fixture's CLOEXEC writer
is live. Fork inherits the descriptor; CLOEXEC closes it only on exec. Immediate
EOF can therefore fail even when Channel correctly drops every received right.

First add a test-only fresh exec of the same unit-test binary, selecting exactly
one registered test with a private child marker and an empty environment. Keep
the outer harness's default concurrency and the existing five-second process
guard. Require successful status, empty stderr and an exact completion marker;
zero selected tests or a failed child must not be accepted. No normal executable
mode, authority input, dependency, runtime policy or qualification change.

Inside that isolated test process, queue writer rights and fork an original
child which closes the unrelated endpoints, verifies its inherited writer's
CLOEXEC flag, acknowledges custody and holds it behind a fixed release gate.
The raw child uses only fixed libc calls and _exit after fork. Retain its original
pidfd before assertions; panic cleanup must kill/reap only that original child.
Deliberately apply the old immediate-EOF assertion first and retain the resulting
EAGAIN negative. Then require WouldBlock while the acknowledged writer is live,
release the child, require its exact exit/wait/reap and only then require EOF.
No retry, sleeping correctness oracle, increased deadline or numeric PID recheck
after reap. This proves a mechanism, not the provenance of the historical failure.

Only after that characterization, run both existing rights-cleanup EOF fixtures
through the same test-only exec boundary, preserving their assertions and all
Channel code. Add a control retaining actual received rights: WouldBlock must
remain distinguishable from EOF until all those rights close. Also retain a
deliberately failing EOF control to demonstrate child failures are propagated.

Run focused units, repeated default-parallel canonical groups and the unchanged
nine process-guard tests independently of heavy compilation. Check/strict-lint
the affected package on current/MSRV Linux and Windows, format, validate the
gated registry/static performance contracts/docs and confirm local admission
remains non-promotable. Retain commands, outcomes, source identities and every
negative in a separate unsigned consistency packet. Update the article with
confirmed mechanism versus historical uncertainty. No server, product workload,
push, authenticated original-start, P2/P3 writer-cessation or release admission.
