# Owned first exec and seccomp composition plan

Baseline: `3b5df398037234d533059c53a5b5b7c4dc650525`. Continue the approved
local bootstrap with one owned child passing a fresh descriptor exec and exact
filter installation. This is mechanics only: no dummy/real credential, runtime
custody admission, producer consumer, host operation or qualification.

## Fixed implementation boundary

Add a private Linux x86_64 library-test module, its tests and an embedded fixed
Rust helper source. Build the helper locally with Rust 1.94.0, GNU target,
opt-level 3, panic abort and no external crates. Compilation precedes any child
launch and uses a cleared environment with explicit local compiler/linker paths.
Retain compiled bytes once per test process and copy into owned temporary 0555
files. The system GNU loader/libraries remain fixture inputs, not independently
admitted runtime custody. This does not change the production build profile.

Reuse only existing private test-build policy/encoding/info decoding and file
selection/stamp helpers through pub(super). Existing predicates and standalone
tests remain unchanged. Normal library symbols/public APIs/CLI/dependencies,
signed and durable formats, reviewed pins and workloads stay unchanged.

Prepare fixed enum arguments, empty environment and stdio before raw fork.
The child closes nonselected inherited descriptors, sets no_new_privs/core
limit, requests TRACEME, stops and descriptor-execs only the original helper.
The thread-bound owner retains the original pidfd and exact-child wait/reap.
Require exact initial/first-exec stops, selected executable metadata, empty
output, one original task and only selected stdio descriptors before continuation.

Syscall tracing then follows bounded GNU startup (at most 256 entry/exit pairs,
five-second stage stall bounds). Before installation, refuse task creation,
another exec, ptrace, prctl and alternate ABI at entry, before dispatch. Handle
only the optional exact successful original execveat exit before startup; do not
skip arbitrary stops. At the first seccomp entry require native mode/TSYNC and
one original task. Capture the actual header address once from kernel entry;
read only bounded 16-byte header and 272-byte policy in that child's selected
read/write nonexecutable stack mapping, freeze both addresses and check exact
bytes/pointer again after zero install exit. Unknown/partial info or mappings,
overflow, nonstack/equal-byte heap inputs, wrong flags/length/bytes refuse.

Unlike the fork-only proof, post-exec stack addresses cannot be inherited from
the parent. This captures actual kernel syscall inputs under trusted fixed
helper/creator/kernel assumptions. It is not independent provenance for an
arbitrary child-supplied pointer, atomic external-writer or runtime attestation.
No public PID/address/filter factory, register guessing or memory mutation.

Require exact PR_SET_DUMPABLE/zero entry and successful exit before probes.
The helper attempts only fixed native self/different exec or execveat; require
kernel EPERM and its zero exit_group/terminal result. Additional failure cases
are missing TSYNC, changed policy/length, equal-byte heap header/program and
pre-install exec attempt. No fatal ABI probe or allow-all task-creation canary
is added; standalone alternate-ABI coverage remains separate.

## Tests and retained evidence

Capture existing eight seccomp and eleven first-exec baseline cases. Add a red
missing-owner test, then actual joined positives, phase/replay refusal,
metadata/equal-byte foreign inode, installer fault cases, pre-install exec
refusal, descriptor/environment invariants, owned child SIGKILL at first-exec
and installed boundaries, and seed 0x07403b27 with eight fresh probes and four
independent tracer threads. SIGKILL signals only retained owned pidfds; tracer
loss/EXITKILL and handoff-loss remain later work.

Run three focused repeats, standalone first-exec/seccomp, bundle/executable and
producer regressions, Windows portability, current strict check/Clippy,
MSRV 1.88, scoped format, normal-library symbol exclusion, root contracts and
docs/registry/local checks. Require-ship remains expected red. Preserve every
development failure separately, bind source/frozen hashes and raw logs, update
the article with the actual conclusion, then commit and push the feature branch.
Next validate bounded GNU runtime inputs/custody before dummy handoff; worker
P2/P3, all-writer revocation, later cells and server admission stay closed.
