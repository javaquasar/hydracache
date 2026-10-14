# Owned first exec fixture boundary

The first approved bootstrap slice creates its own nonroot Linux helper and
requires an exact kernel exec-event stop before continuing the selected
executable. An actual same-inode second exec now stops separately and refuses
the owner before new helper code runs. This is private test-build mechanics,
not a production signer launch or authenticated worker original-start.

## Original launch and ownership

The module exists only in Linux x86_64 library test builds. It copies the local
ELF shell into a unique temporary directory, checks a read-only single-link
0555 descriptor owned by the effective nonroot fixture UID, and reads a bounded
64-byte ELF identifying header. A deliberately truncated header-only binary
passes that shallow identification but fails actual kernel exec, so this is not
a full ELF/runtime parser. Loader and libraries remain the system fixture's
runtime, not independently admitted signer inputs.

All arguments, fixed environment and descriptors are prepared before fork.
The child performs libc operations and exits without Rust unwinding on error.
It redirects only owned input/output to stdio and closes every other inherited
descriptor except the original selected executable before its initial SIGSTOP.
The copied child descriptor table is not the parent's table; close_range uses
zero flags, with no unshare or parent mutation. Unsupported kernels refuse.
The environment contains only fixed SELF/PROGRAM fixture values, not inherited
runner credentials or loader overrides. This is not job-wide permission removal.

The thread-bound owner retains the original pidfd and waits only for its own
unreaped fork child. Initial SIGSTOP must be exact. It requests TRACEEXEC and
EXITKILL, then descriptor-execs through execveat/AT_EMPTY_PATH and requires the
exact PTRACE_EVENT_EXEC status. At that stopped original child it compares
procfs executable metadata with the retained original binary, including inode,
ownership/mode/link/length and mtime/ctime. Output remains empty until the first
continuation. No caller PID, arbitrary command, public descriptor or adoption
factory exists.

The fixture protocol acknowledges, re-execs the same /proc/self/exe object,
execs a fixed /bin/cat, or exits on EOF. Any observed second exec or unexpected
status refuses the owner and terminates/reaps only its original helper.
Drop uses the original pidfd; only pidfd acquisition failure may clean the exact
still-unreaped fork PID. No wait-any, ptrace attach/detach, memory/register
mutation, service/cgroup cleanup or signing consumer is added.

Kernel exec stops occur before the new image runs user code; TRACEEXEC and
EXITKILL semantics are described in the [ptrace manual](https://man7.org/linux/man-pages/man2/ptrace.2.html).
An observed second exec is stopped after kernel exec, not denied before the
syscall. The owner is not an autonomous watchdog, and actual tracer-loss
EXITKILL behavior has not been tested in this slice. No dummy credential handoff
API or authority conversion exists.

## Why CLOEXEC did not solve descriptor inheritance

Initial concurrent tests sometimes returned exec errno 26, ETXTBSY. An exact
two-child regression reproduced the failure: the parent opens a writable
descriptor for the second child's binary, forks the first child and leaves it
at the initial stop, then closes its own writable descriptor. Without early
closure, the stopped first child still retains its inherited CLOEXEC duplicate;
the second child cannot exec that binary. This needs no timing-dependent race.

The fix closes all nonselected descriptors before the initial stop, preserving
only stdio and the selected executable. The same regression then succeeds while
the first child is still stopped. Closing on successful exec was too late for
this boundary. The [exec manual](https://man7.org/linux/man-pages/man2/execve.2.html)
documents the writable-executable refusal; the
[close_range manual](https://man7.org/linux/man-pages/man2/close_range.2.html)
distinguishes immediate closure from CLOEXEC marking. Neither result certifies
production runtime custody or freedom from all possible writers.

## Local checks and remaining approved work

Eleven fixture cases cover original exec/output ordering, foreign equal-byte
inode, header/FD/type/mode/link rejection, shallow-header kernel refusal,
same/different second exec, unexpected signal, output overflow, CLOEXEC/drop
cleanup and the inherited-writer regression. The wait classifier checks 327680
lower-status/event combinations and unsupported high bits. Seed `0x07403b25`
covers 16 fresh launches and four independent tracer threads. Five-second
deadlines diagnose a stalled fixture, not a performance threshold.

The [contract](diagnostic-signer-first-exec-contract.toml) and
[preregistration](diagnostic-signer-first-exec-plan-20261014.md) keep normal
library, producer, server and workflow consumers absent. No signed schema,
reviewed pin, Cargo dependency, product workload or qualification input changes.

Next implement the approved local post-launch exec policy and its installation
proof for execve/execveat, alternate ABI and all threads; a ready string or
Seccomp=2 cannot prove the exact policy. Dummy handoff and actual tracer/child
loss tests follow. Independently selected runtime/root/issuer/job custody must
precede any production consumer. Worker P2/P3, replay/common fence, all-writer
revocation, completion/later cells and server admission remain separate.
