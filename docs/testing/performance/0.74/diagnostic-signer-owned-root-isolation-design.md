# Ordinary root restrictions without child procfs

The separate fixed `root-native`, `root-x32` and `root-i386` diagnostics connect
ordinary selected-root execution with the existing restriction policy. The old
`prepare` and `root-exec` modes keep their narrower behavior. This is local
nonproduct launch mechanics, not a credential producer, protected handoff or
authenticated worker-start path. The [contract](diagnostic-signer-owned-root-isolation-contract.toml)
keeps those admissions closed.

## Shared installer and parent responsibility

The original ordinary isolation entry still checks its procfs status, identity,
sole main task, empty environment, descriptor inventory, filter limits and
post-install state. Only its fixed syscall-mutating block and native/x32/i386
exercises move into a private source included by both terminating entries. No
public library installer, supplied-filter API or policy exception is added.
The 64 instructions and 512 policy bytes remain unchanged.

The selected root contains only the four admitted images and directories, no
procfs. The raw child closes all host proc descriptors before original exec.
The parent retains its own original process directory/pidfd, selected images and
namespace. At loaded stop it already checks root/cwd, credentials, zero capability
sets, no_new_privs, task/FD identities, executable, environment and bounded mappings.
It additionally retains original environ and FD-directory readers in the parent
before no-dumpability, never passing these descriptors to the child.

## Actual installation observation

The parent resumes only through syscall tracing. Native architecture, exact entry
and exit shape/order, successful results and a bounded 128-event observation are
required. The fixed sequence is eight syscall pairs: core-limit installation,
set/readback no_new_privs, TSYNC seccomp, SET_DUMPABLE(0), installed-marker write,
getpid and SIGSTOP. Unexpected calls, stops, duplicate entry, missing exit, second
install, foreign ABI or failed result refuse and terminate the original child.

At seccomp entry, the parent reads the actual 16-byte sock_fprog and all 512 BPF
bytes with ptrace from the previously validated stack range. Checked arithmetic
and aligned-word bounds reject a foreign, short, oversized or overflowing span.
Header length must be 64 and the complete bytes must equal the unchanged policy.
Only mode FILTER/flag TSYNC and syscall success are accepted. Kernel status after
the installed stop must show mode2 and exactly one filter more than before.

An attempted direct GET_DUMPABLE readback after installation returned EPERM under
the existing prctl tail. It was removed, not allowed by weakening policy. ND0
evidence is the observed successful SET_DUMPABLE(0), exact installed policy and
existing native restoration-denial exercise. No successful direct query is claimed.

After ND0, original environment is read positionally through its retained FD and
the retained original FD directory is rewound/read with bounded getdents64. Every
read checks procfs, readonly/non-O_PATH/CLOEXEC flags and original inode/type;
there is no fresh-open fallback. Live kernel root/cwd/ns/exe/status/task/mappings
are still checked. FD object identity was checked before the strict installation
sequence, which permits no open/close/dup/exec/clone calls, so post-ND0 inventory
is not presented as a new descriptor-object capture. The original reads remain
observations, not runtime custody or peer authentication.

## Terminal exercises and cleanup

Three distinct private version1 messages are fixed 16-byte loaded, installed and
native-done markers. A marker cannot replace the syscall/BPF/context proof. Only
after installed validation does the parent resume the existing native20 EPERM
exercise or alternate ABI. Native must exit0 with exactly one done marker;
x32/i386 must terminate coreless SIGSYS without done output. Reap/ECHILD and pipe
EOF precede final root checks and fixed-object cleanup. Failure drains at most
48 bytes for these modes; root-exec retains its previous16-byte cleanup bound.

Fixed `root-kill-installed` and `root-drop-installed` modes stop at that same
validated boundary and exercise explicit original-pidfd SIGKILL or ordinary Drop.
Drop control retains only original parent/pidfd handles for read-only terminal,
ECHILD and root-name-absence checks afterward. No replacement process/tree is
adopted. These controls are child loss/cleanup, not creator-loss proof.

## Tests and remaining risks

Normal-process tests exercise the real current-toolchain binary and all five
new fixed modes. MSRV's extra-libm image deliberately refuses the unchanged
runtime graph. Units test event/state/dirent decoding, reader provenance/flags,
phase behavior and memory bounds. A private pre-exec owned-child stack fixture
uses test-only POKEDATA to check header and 256 seeded BPF mutations; production
observation only reads memory. This fixture is not a loaded-runtime attestation.

Full ordinary-entry creator-loss faults, protected one-shot handoff, independently
authenticated runtime custody and production inputs remain separate. The prior
overlapping-broad-load timeouts remain retained; isolated default-parallel success
does not establish load robustness or root cleanup after owner SIGKILL. P2/P3
worker start/all-writer cessation, completion/later cells, server rehearsal and
release qualification are not enabled by this diagnostic.
