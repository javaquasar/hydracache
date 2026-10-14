# Owned fresh exec and filter installation boundary

The third approved local bootstrap slice follows one original child through a
fresh GNU helper exec and exact seccomp installation, then observes denied
self/different/descriptor exec. Owned child SIGKILL is tested at the first-exec
and installed boundaries. This is test-build mechanics, not runtime custody,
dummy handoff, a production signer or worker-start authority.

## Preparation and first exec

The embedded standalone helper is built before launch with local Rust 1.94.0,
GNU target, O3/panic-abort and no external crates. Compiler discovery and build
environments are cleared, with explicit local paths; remapped source paths keep
fixture setup identities reproducible. Compiler output is capped at 8 MiB.
The compiler, linker and system GNU loader/libraries are trusted local fixture
inputs, not independently authenticated production runtime selection.

Cached compiled bytes are materialized through an exact /usr/bin/tee child,
using a cleared environment and a private output path. The parent writes only
the input pipe, closes it, waits for copy success, compares resulting bytes and
freezes mode 0555 before launch. It holds no writable executable-file descriptor
that another raw fork could inherit. The retained image and output are private
temporary objects; copying accepts no caller command or arbitrary output path.

The original launcher prepares fixed enum arguments, empty environment and
selected stdio before fork. The child uses only libc/_exit until descriptor exec,
closes every nonselected descriptor, sets no_new_privs and core limit, requests
TRACEME and stops. A thread-bound owner retains the original pidfd and exact-child
wait/reap. Initial stop and PTRACE_EVENT_EXEC must be exact. The selected original
file metadata must match procfs executable metadata, output must still be empty,
there must be one original task and only descriptors 0/1/2 at that exec stop.
The existing shallow ELF identifier is not a complete runtime parser.

## Installation across a new address space

The owner switches to syscall tracing after first exec. It handles only an
optional successful original exec exit before bounded startup entry/exit pairs.
Before installation, another exec, task creation, ptrace/prctl, exit and syscall
numbers outside the fixed 0..1023 range refuse at entry, before dispatch. Unknown
architecture, signal/event or sequence also refuses. This prevents high-word or
x32 forms from bypassing the pre-install native-number check. A maximum of 256
startup pairs and five-second stall bounds diagnose unsupported/stalled fixtures;
neither is a performance threshold.

At the first native seccomp entry, mode and TSYNC must match. The owner captures
the actual header address once from kernel syscall information, reads a bounded
header in the original read/write nonexecutable stack mapping, and captures its
program pointer. Both addresses remain frozen while header length/pointer and
all 272 policy bytes are checked at entry and after exact zero exit. Missing
flags, altered policy/length, equal-byte heap header/program, unknown captures,
out-of-range/overflow memory and unsupported stack mappings refuse before the
installer is continued. Bounded procfs mapping reads are observations, not
runtime attestation; see the [maps manual](https://man7.org/linux/man-pages/man5/proc_pid_maps.5.html).

The post-exec addresses cannot be preselected in the parent's old address space.
This proves actual syscall inputs for the trusted fixed helper, not independent
provenance of an arbitrary child's pointer or atomic protection from external
memory writers. It deliberately does not enlarge the standalone fork-only
contract. Original task and executable checks bracket installation; exact
PR_SET_DUMPABLE/zero entry and zero exit precede the native probes. No ready
message substitutes, no memory/register mutation or privileged dump occurs.

The same owned child then attempts fixed execve/execveat. The owner requires
EPERM from the kernel, zero exit_group and zero terminal exit with empty output.
Premature/repeated installation, wrong probe/state and terminal replay refuse
and latch the owner. Error/drop terminates and reaps only the original child.
EXITKILL is requested but actual tracer-loss behavior is still a later test;
the [ptrace manual](https://man7.org/linux/man-pages/man2/ptrace.2.html) describes
the requested kernel controls.

## Why early closure was not sufficient during preparation

A repeated joined run failed first exec before installation. Bounded errno
diagnostics and an owned stopped descriptor-table snapshot reproduced errno 26,
ETXTBSY (0x9a00). The parent had released its image writer, but a snapshot child
still held the inherited writable description. Early close_range in normal
children does not guarantee every concurrently in-flight fork has reached it.

Moving image writes into the copy process removes that writable description
from the parent's table instead of depending on scheduling or retries. The
snapshot can remain stopped while fresh exec/installation succeeds. The same
test deliberately supplies an external inherited writer and still observes
ETXTBSY: the fix isolates owned preparation, not whole-writer revocation. The
[preregistered amendment](diagnostic-signer-joined-writer-amendment-20261015.md)
retains the failed repeat and deterministic red; no seed, timeout or numerical
threshold is adjusted.

## Checks and remaining prerequisites

Nine fixture cases cover joined positives, both exec forms, entry refusal,
installer memory faults, phase/range/replay constraints, metadata/foreign inode/
nonempty output, child SIGKILL, the writer regression and seed 0x07403b27 with
eight fresh probes plus four independent tracer threads. Existing standalone
policy and alternate-ABI tests remain separate and unchanged in behavior.

The [contract](diagnostic-signer-joined-contract.toml) and
[main preregistration](diagnostic-signer-joined-plan-20261014.md) keep normal
symbols, producer/server/CLI consumers, schemas, dependencies, reviewed pins and
frozen qualification inputs unchanged. Shared helpers gain only test-module
pub(super) visibility; production libraries contain neither fixture nor helper.

Next validate bounded GNU runtime inputs and independently selected custody
before one-shot dummy handoff, then actual tracer/child-loss and handoff faults.
This single-thread helper cannot admit an already multithreaded signer, a mapped
memory claim or an external writer. Production issuer/root/job custody, worker
P2/P3, replay/common fence, all-writer cessation, completion/later cells and
server/qualification admission remain separate and closed.
