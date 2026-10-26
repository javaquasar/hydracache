# Native inclusive CPU accounting for short Git calls

This local diagnostic measures lifetime CPU of non-launcher descendants during
four ordinary short `git show` calls. It addresses the launcher-only attribution
gap without delaying Git on stdin, discovering a child after it has exited, or
invoking the native executable directly. It is not a package replay, product
candidate or performance acceptance test. V6 must not be repeated.

## Accounting scope and execution

An isolated Rust test worker joins a new private Windows Job atomically at
creation, before running. The existing CI Win32 implementation supplies that
creation primitive unchanged; its normal kill-on-close configuration is NOT
used. The new accounting subclass leaves the Job unlimited and does not enable
kill-on-close or breakaway. Only the worker is initially suspended for membership
verification and resumed once. Git itself is never suspended.

The worker executes `Command::new("git").args(["show", SPEC])` four times, with
the original repository cwd and inherited environment. SPEC, the independent
23,589-byte body and its SHA256 are fixed in the
[protocol](git-job-cpu-protocol.toml). A reference worker uses unchanged
`Command::output()`. The accounted worker uses the existing unchanged pipe helper
with null stdin, separate stdout/stderr pipes, retained direct-process handle and
terminal direct-launcher counters. Both require identical body bytes, empty
stderr and exit zero on every call. There is no cache, batch, alternate Git image,
additional Git option, per-command Job attachment or conditional-read rewrite.

Job membership is an explicit observer change, not a claim of invisible
instrumentation. Both workers have that membership; the reference controls the
pipe helper's functional semantics, not its CPU overhead. The isolated worker's
CLI must be exactly the selected diagnostic case, `--exact`, `--nocapture` and
`--test-threads=1` in the fixed order. Ordinary test execution and listing cannot
launch this diagnostic workload. No environment opt-in is introduced.

After worker exit, its original creation handle supplies whole-lifetime CPU.
Two consecutive equal Job snapshots must show no active process and exactly
nine lifetime members: one worker, four direct launchers and four other
descendants. Any other count refuses rather than guessing. For the accounted
worker, subtract the whole worker lifetime and four whole direct-launcher
lifetimes from terminal Job user/kernel CPU separately. Negative residuals
refuse; no clamping and no subtraction of a late first sample are allowed.

The residual includes native Git work from birth, but Job counters do not
authenticate each descendant's image or distribute CPU across calls. Independent
on-disk launcher/native SHA256 pins before and after execution are observations,
not mapped-image attestation or an ABA defense. Label this **non-launcher
descendant CPU**, never exact per-image or per-call native CPU. The old launcher
profile and held batch control retain their original scopes and results.

## Bounds and first outcomes

One precommitted diagnostic session runs reference then accounted: eight actual
short Git calls total. A common 120-second origin starts before Job creation and
is never renewed for the second worker. The separate cleanup window is five
seconds. A four-GiB free-space floor is checked before and after; each completed
worker log must fit 16 MiB. The finite worker's fixed output is not an arbitrary
output producer; the enclosing source-preserving wrapper retains its own guard.

Healthy exit never requests termination. On refusal, cleanup may terminate only
the private diagnostic Job, waits within the existing cleanup origin, attempts
both original-handle releases even after another cleanup error, and preserves
the first refusal independently of cleanup errors. The Job is not production
writer authority or a host-wide fence. Abrupt death of this observer does not
give it kill-on-close guarantees, deliberately absent from this accounting test.

The original environment preflight is reused read-only, but the V6 source review,
package dispatcher and native owner/service paths are not called. Source/HEAD,
worker build receipt/image and independent Git pins must remain unchanged. All
raw first results remain evidence; there is no retry-to-pass. The existing
780-second package guard and 300/60/10 live windows, predicates and observation
frequency remain untouched. No server, clock, service, dependency or product
change is included. CPU numbers from this one control cannot support a speedup
or overhead acceptance claim.
