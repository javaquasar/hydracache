# Original creating thread loss while its parent process survives

The private Linux x86_64 fixture adds the original creating task to the existing
parent-death guard. A live parent TGID alone is insufficient after that specific
thread exits. This is one local launch-lifetime hypothesis under the existing
[bootstrap decision](diagnostic-signer-bootstrap-decision.md), not signer/job
custody or production enrollment. The [preregistration](diagnostic-signer-creator-loss-plan-20261015.md)
and [contract](diagnostic-signer-creator-loss-contract.toml) keep its oracle and
admission separate from the earlier whole-parent-loss packet.

## Original task object and bounded child check

The original builder captures gettid and follows only fixed `/proc/thread-self`
before fork. It retains a readonly CLOEXEC procfs directory and validates fresh
status Pid/Tgid against the actual calling thread. After PDEATHSIG arming and
the original getppid check, the raw child opens only `status` relative to that
same directory. It never refreshes from a numeric TID. The kernel documents
that a retained proc object does not become a later process with the same ID;
operations on the dead object generally fail. See the
[procfs object semantics](https://docs.kernel.org/filesystems/proc.html).

The child uses an initialized 8193-byte stack buffer, at most eight scalar reads
and an accepted 8192-byte bound. Exactly one Pid, Tgid and State must match the
original task; only live R/S/D/T/t/I states pass. Missing, duplicated, dead,
unknown, wrong or oversized fields refuse with exit81. The fresh status file
closes before returning even on parser refusal. There is no Rust allocation,
lock, destructor or formatting after fork. The existing overlap into EXITKILL
remains unchanged, before namespace/credential changes and the GNU helper.

## Real thread death and independent proof

The original tracer has a fixed keeper thread as well as its creating and
libtest/main threads. At each of five pre-options cuts the original callback
sends a two-FD prefix (private gate and original task directory), then the
original child pidfd and four readonly root directories. Kernel credentials
must match the original tracer TGID/UID/GID. A distinct 32-byte report binds cut,
control/healthy flags and original TID, with zero reserved fields. The unchanged
ancillary parser accepts at most five FDs per packet and 128 control bytes.

Before injection the observer checks child PPid/expected TracerPid, task
directory inode/flags/type and matching original status. The original task's
bounded `children` record must contain exactly this one child. All fixture
children are held at known gates/stops with no exiting sibling during this
bracket; this is not a general unfrozen-process ancestry scanner. The
[children interface caveats](https://man7.org/linux/man-pages/man5/proc_tid_children.5.html)
remain relevant outside that fixed bracket.

The observer requests raw SYS_exit from only the original callback thread.
It does not use SIGKILL or libc _exit to stand in for thread-only death; those
would terminate the group. The creating thread completes Rust preparation
before its raw syscall, while the keeper and libtest main remain. The
[exit syscall](https://man7.org/linux/man-pages/man2/exit.2.html) and
[PDEATHSIG definition](https://man7.org/linux/man-pages/man2/PR_SET_PDEATHSIG.2const.html)
explain this distinction. Rust thread bookkeeping is intentionally not resumed
on the loss path; this dedicated owned process later exits as a whole.

At the before-arm cut the observer first confirms the original task is dead
through its retained directory and the original tracer pidfd remains pending.
It releases only the private gate. The old getppid still matches, but the new
task check requires CLD_EXITED/81. At the four armed cuts require
CLD_KILLED/SIGKILL from actual creator-thread loss. The keeper peeks the original
child's WNOWAIT terminal record without consuming it. The independent observer
corroborates child pidfd readiness while the original tracer pidfd remains
nonterminal, before allowing normal exit0 of that tracer process.

After reparenting the independent subreaper requires the exact same original
child's kernel tuple, consumes it and requires ECHILD. Only then may original
temporary root directories be chmodded and removed. No child is signaled and
no dummy is sent in a loss case. Emergency cleanup is failed evidence; it can
leave owned forensic files and cannot manufacture a positive.

## Controls and limits

The closed control skips only the new task check at before-arm. It passes the
old getppid predicate after real creator death, then exits86 before TRACEME/exec.
That counterfactual cannot satisfy the positive oracle and does not claim a live
orphan. Five healthy controls complete the original ACK/close/zero-exit chain
and join their keeper. Seven credential-checked progress bytes remain hints,
not admission. At most ten packets are accepted, every gap stays five seconds,
each outer process stays bounded by 30 seconds and capture remains 8192 bytes.

Eight tests cover the closed role entry, five actual creator-only cuts, control,
five healthy handovers, bounded status/frame/oracle mutations, retained task
death and duplicate/reader FD reconciliation in a separate post-exec role,
eight seeded fresh cases and four independent observers. All 130 prior signer
regressions remain required. The unchanged GNU helper/policy/runtime/dependencies
and qualification bytes are not candidates in this experiment.

The retained object avoids numeric TID refresh; no forced TID-reuse event is
claimed. This does not authenticate arbitrary signer threads, all-thread policy,
mapped memory, independent runtime/job/issuer custody or actual credentials.
Worker original-start and P2/P3 all-writer cessation, completion/cells 2–4,
production host installation/rehearsal and qualification remain separate.
