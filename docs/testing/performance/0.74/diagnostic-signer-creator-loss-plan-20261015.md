# Original creating thread loss before EXITKILL

Preregistered against `23fdfa78cab391e18768ae2ccd9105679712c621`.
One local W11 hypothesis: retain the actual creating thread's original procfs
directory before fork, then check it after PDEATHSIG arming as well as getppid.
The previous guard checks the parent TGID, which can survive its creating thread.
No production inputs, private key/token, server or workload operation is allowed.

## Original task guard

Inside the existing RootedExec builder, capture gettid and deliberately follow
the fixed `/proc/thread-self` magic link. Require a readonly CLOEXEC directory on
procfs and a fresh status read matching the actual parent TGID and creator TID.
Keep this original descriptor, never reopen a numeric TID in the raw child.
After arming and checking getppid, open only `status` relative to that original
directory with NOFOLLOW/CLOEXEC/NONBLOCK. Use an initialized 8193-byte stack
buffer, at most eight scalar read calls and an 8192-byte accepted bound. Require
one Pid, one Tgid and one State; admit only live R/S/D/T/t/I states. Duplicate,
missing, oversized, future/dead state or wrong identity refuses with exit81.
The raw child uses no Rust allocation, lock, destructor or formatting.

The retained proc object must not adopt a reused numeric TID. This follows the
[kernel procfs object semantics](https://docs.kernel.org/filesystems/proc.html),
not a PID-presence poll. An unavailable required source refuses rather than
weakening the check. The same original owner, GNU helper, runtime selection and
EXITKILL handover remain; no independent runtime custody is claimed.

## Actual thread death and independent terminal proof

Use only fixed current-libtest subprocess roles below the existing independent
loss observer. The original tracer process has a dedicated keeper thread in
addition to its libtest/main and creating threads. At five original cuts
(before arm, armed, checked, TRACEME, first stop before options), the callback
reports the original child pidfd/four root dirs and a separate gate/task pair.
The report uses distinct `HC074-TASK-v001!` magic, exact 32 bytes, cut/mode/healthy/
original TID and zero reserved fields. Keep the original bounded 128-byte
kernel-credential/SCM_RIGHTS parser: at most five FDs per packet, seven total.
One setup byte precedes a two-FD prefix and five-FD report. Require original
TGID/UID/GID, child PPid/expected TracerPid, retained task inode/type/flags and
matching creator TID/TGID. The original task's bounded `children` must contain
exactly the one original child before death; the child cannot exit or spawn
another child while held at these private cuts. This is an original-construction
test bracket, not a general unfrozen-process ancestry scanner.

The monitor requests the original callback to call raw SYS_exit(0), not libc
_exit or a signal: those would end the whole thread group rather than isolate
the creator. All Rust preparation finishes before that syscall. The keeper
observes the retained original task become dead, while the original tracer
pidfd remains nonterminal. At the before-arm cut the monitor releases only the
private gate after independently observing original creator death; getppid must
still match the original live TGID. The new task guard must yield CLD_EXITED/81.
At four armed cuts require CLD_KILLED/SIGKILL from actual creator-thread loss.
No child is signaled and no dummy is sent in any loss case.

The keeper observes the original child pidfd's exact WNOWAIT terminal record
without reaping and reports its bounded code/status. The independent monitor
requires child pidfd readiness while original tracer pidfd remains nonterminal.
Only then may the keeper _exit(0) the tracer process. After reparenting, the
independent subreaper requires that same original-child terminal tuple, consumes
it and requires ECHILD. Chmod/removal follows exact child reap, never emergency
group cleanup. A failed feasibility attempt can retain owned forensic files.

A closed no-task-check control at before-arm skips only the new check. It must
still pass the old original-TGID check and exit86 before TRACEME/exec. This is a
causal counterexample to the previous predicate, not a live-orphan claim and
never positive proof. Healthy controls at all five cuts retain the original
ACK/close/zero-exit protocol and join the keeper normally. Seven exact,
credential-checked descriptor-free progress bytes cover healthy stages. At most
ten packets total; every gap remains five seconds, outer bound30 seconds and
capture8192 bytes. Progress is a hint, never retry/admission.

## Focused gate and unchanged boundaries

Retain fresh previous seven-case baseline and current check before mutation.
Add real creator-only death at all five cuts, the counterfactual, five healthy
controls, raw parser/status identity/state/length mutations, retained-task death
and FD closure checks, eight seeded fresh cases and four independent observers.
Three fresh current repeats, all 130 previous signer regressions, MSRV1.88,
current strict check/lint, Windows nine portable tests/exclusion, xtask source
contracts/check/lint, format/docs/gate registry/local performance contract must
pass. Require-ship stays expected-red. Retain failed predictions and source-bound
raw logs; source commit remains separate from article/evidence and pushed only
after Windows/Linux committed-byte verification.

Expected files: one private original-task guard/parser, minimal rooted-exec and
parent guard wiring, one private observer below tracer-loss plus focused tests,
source contract, contract/design/GATES/COMPAT, local packet, overview and article.
Do not change old helper/policy/runtime, dependencies, qualification manifest,
issuer pins, actual key, workflows, normal CLI, host accounts/services/cgroups
or product code. This does not enroll arbitrary multithreaded signers, all-thread
credential policy, runtime/job/issuer custody, worker P2/P3, completion/cells2–4,
server execution, qualification or ship admission.
