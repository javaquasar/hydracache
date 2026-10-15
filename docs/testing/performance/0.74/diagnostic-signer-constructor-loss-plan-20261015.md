# Original parent loss before EXITKILL installation

Preregistered against `e4eaea4cd4af5e09082cf8cb8b759ff40ae09d7c`.
This is one local W11 launch-lifetime hypothesis, not production enrollment.
The preceding tracer-loss packet proves six stops after options installation;
the raw-fork constructor currently has no parent-death guard before that point.

## Hypothesis and handover

Capture the original parent TGID immediately before the existing fork. In the
child, before descriptor/root/credential changes, set PR_SET_PDEATHSIG(SIGKILL)
and compare getppid with that captured original TGID. Setting the signal alone
is not retroactive when the parent already died. A mismatch exits 81; failed
guard installation or clearing exits 82. Neither can reach exec or dummy input.
Keep the guard through TRACEME and the initial stop. The original parent must
successfully set the existing EXITKILL options before resuming the child. Only
after that resume may the child clear PDEATHSIG, before user-namespace or
credential changes can reset it. There is no unguarded handover to exec.

The kernel defines parent death relative to the creating thread, whereas
getppid identifies its thread group. This experiment kills the entire original
parent process. It does not prove a creating-thread-only death before arming
while other parent threads live, nor TID reuse, independent launcher custody,
credential transitions before handover or arbitrary multi-threaded signers.
Those remain explicit separate limits. References:
[PDEATHSIG](https://man7.org/linux/man-pages/man2/PR_SET_PDEATHSIG.2const.html),
[getppid](https://man7.org/linux/man-pages/man2/getpid.2.html), and
[ptrace](https://man7.org/linux/man-pages/man2/ptrace.2.html).

## Closed fault cuts and exact terminal oracle

Use the same original RootedExec builder and unchanged GNU helper. An optional
private test hook pauses at exactly five cuts: before arming, armed before
parent check, checked before descriptor setup, TRACEME before initial stop, and
initial stop before SETOPTIONS. No public launch, path, PID or FD API is added.
The ordinary builder has no hook and still executes the guard.

An independent isolated subreaper owns the original tracer Child/pidfd and
receives the original child pidfd plus four original readonly directory FDs.
Reuse the existing bounded kernel-credential/SCM_RIGHTS parser and exact nested
root checks; the new report has a separate magic and closed cut/fault fields.
Require exact child PPid, expected TracerPid (zero at the first three cuts),
and original tracer TID/TGID before injecting the fault. Receive descriptors
into ownership before rejection. No unvalidated received pidfd is signaled.

Kill only the original tracer pidfd. At the before-arm cut the child resumes
on its private gate EOF, arms the now-nonretroactive guard, detects reparenting
and must exit CLD_EXITED/81. At the four armed cuts require
CLD_KILLED/SIGKILL, independently consume the exact pidfd waitid record and
require ECHILD. Only then chmod/remove original temporary directories. Emergency
group cleanup cannot satisfy any positive proof. No child is signaled on a
successful path, no replacement is created and no dummy is sent in loss cases.

Healthy controls at all five cuts release the private gate and finish the
existing ACK/close/zero-exit protocol under the original owner. Separate closed
fault controls disable only arming at the armed cut (natural exit 83 on gate
EOF), or only the original-parent check at the before-arm cut (natural exit 84
after the missed parent). They terminate before TRACEME/exec and must never be
accepted by the positive death oracle. They do not claim a live orphan.

## Bounds and focused gate

Keep five-second per-progress/gate bounds, 30-second outer bound and 8192-byte
captured output. Send one setup progress before construction and one bounded
report (32 bytes, five FDs, 128-byte ancillary area); closed gate tokens are one
byte with no descriptors and checked original kernel credentials. Oversized,
unknown/duplicate fields or wrong credentials refuse before injection. Hooks
perform only prepared scalar libc operations after fork; no Rust allocation,
destruction, lock or formatting in the raw child. Keep the ordinary test process
subreaper state unchanged. No new dependencies or helper build/profile changes.

Before mutation retain a fresh eleven-case tracer-loss baseline and current
check. Add focused constructor cases, finite terminal/frame mutations, seeded
fresh cases and independent monitor concurrency; run three fresh repeats, all
123 previous local signer regressions, Rust1.94 strict lint/check and MSRV1.88.
Run Windows portability/exclusion, xtask source contracts/check/lint, scoped
format, documentation, gate registry and local non-promotable performance
contract checks; require-ship remains expected-red. Retain failed observations
without changing the positive oracle or increasing the frozen bounds.

Expected files: rooted-exec builder/child, one private constructor guard/hook,
one child module below the independent loss observer and its focused tests,
minimal parser visibility for reuse, xtask source guard, contract/design/gates/
compatibility register, source-bound local evidence and the profiling article.
The actual private key, worker issuer, runtime/root selection, production CLI,
server/service/account/cgroup, workflows and qualification manifest are unchanged.
Commit source separately from evidence/article; verify committed packet bytes
on Windows and Linux before a nonforce feature-branch push.
