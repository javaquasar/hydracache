# Owned fixture first exec observation

Baseline is `09c9d439934203e1b8e4383c1fc378f8df1f6f2c`. The operator approved
the [local bootstrap prototype](diagnostic-signer-bootstrap-decision.md).
This first slice evaluates child ownership and kernel exec stops only. It does
not install seccomp, hand off even dummy credentials, certify a runtime root,
authenticate a signing job or implement worker P2/P3. Those follow as separate
slices; no further permission is needed for the remaining approved local-only
prototype, but actual trust enrollment and external operations remain closed.

## Exact scope and mechanics

Add private `diagnostic_signer_first_exec_fixture.rs` and its tests, included
only under `cfg(all(test, target_os = "linux", target_arch = "x86_64"))` in lib.rs.
No normal-library launch symbol, public API, CLI, helper mode, IPC, dependency,
workflow or durable schema. Extend root performance-contract coverage of that
exclusion. Runtime semantics outside this private fixture remain unchanged.

Copy the local ELF shell into an owned nonroot temporary directory. Check a
read-only regular single-link nonempty descriptor, executable/nonwritable mode
and a bounded ELF64 little-endian x86_64 header. This rejects scripts and obvious
wrong shapes, not arbitrary malformed ELF or unselected loader/runtime inputs.
The kernel exec must succeed independently. The copied shell is a mechanics
fixture with system loader/libraries, not the admitted signer or trusted runtime.

Prepare all arguments, fixed environment and pipes before raw fork. The child
uses only libc operations before exec: close/dup2, no_new_privs, PTRACE_TRACEME,
SIGSTOP and descriptor execveat with AT_EMPTY_PATH, then _exit on failure.
Never allocate, run Rust destructors or take locks in the post-fork child.
Avoid std::process::Command pre_exec stops because its exec-error pipe can make
spawn wait while the parent still needs to resume the stopped child.

The parent owns exactly that unreaped child, obtains its original pidfd, and
waits only for its PID. At the initial SIGSTOP install TRACEEXEC and EXITKILL,
then continue to the first exact PTRACE_EVENT_EXEC stop. Compare the stopped
procfs executable's dev/inode and metadata with the retained selected binary
descriptor before continuing. User code has not run at that exec stop; a pipe
readiness message is not substituted for the kernel event. The owner is private
and thread-bound because ptrace ownership follows the tracing thread.

After resume, the fixed shell can acknowledge, re-exec the same descriptor
object through /proc/self/exe, exec /bin/cat, or exit on input EOF. Any observed
second exec stop or unexpected wait status latches refusal, signals only the
owned retained pidfd and reaps that exact child. This kills at an exec stop,
not a pre-syscall seccomp denial, and is not a continuously running watchdog.
No secret/dummy handoff method exists. EXITKILL is requested, but actual tracer
loss is not proved in this slice. Drop cleans only the owned helper; it never
signals a service, cgroup or adopted PID.

## Tests and falsification

Capture the eight-case executable baseline first, then a missing-owner compile
refusal. Cover bounded descriptor/header rejection, exact first exec/object,
no helper output before continuation, foreign inode comparison, same-inode and
different-inode second exec stops, sticky no-resume refusal, graceful EOF exit,
owned drop cleanup and CLOEXEC. Exhaustively classify the lower 16-bit wait
status space, with wrong event/high bits/exit/signal/group stops refusing where
an exec event is required. Seed `0x07403b25` covers 16 fresh original launches;
four independent tracer threads each create, observe and clean their own child.
Use logical pipe/event signals; five-second deadlines only bound fixture stalls.

Run three focused Linux repeats, legacy executable/bundle/process-IO tests,
affected current/MSRV checks and strict Clippy, portable Windows regression,
root contracts, format/docs/registry and local non-promotable performance gate.
Require-ship remains expected red. Preserve tested source, raw captures and
unchanged/frozen hashes; keep preregistration, fixture code and evidence commits
separate. No performance measurement or qualification follows.

## Following approved slices

Next evaluate an exact post-launch exec policy covering execve/execveat,
alternate ABI and all threads; independently account for policy installation
rather than trusting a ready string or Seccomp=2. Then test a bounded dummy
handoff and actual owned tracer/child loss. Runtime selection/custody must be
joined before any production consumer. Kernel/root/runtime trust remains an
explicit assumption, not memory attestation. A fixture success cannot open
signing, receiver replay/fence, worker start, all-writer cessation or later cells.
