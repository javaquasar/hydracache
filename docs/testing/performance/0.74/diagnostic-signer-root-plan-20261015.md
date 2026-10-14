# Nonroot selected runtime root feasibility

Baseline: `aa54bfd9b64f1ba87bd090f6b17c55e4da8186a8`. The approved GNU
bootstrap requires the kernel interpreter and dependency search to use selected
objects. Library-path priority alone permits fallback; directly executing the
loader changes the first executable object. Evaluate an owned child-only user
namespace and chroot, with unchanged nonzero numeric UID/GID, no mounts and no
host-root operations. Do not change sysctls or retry unsupported mechanisms.

This slice prepares a fixed hierarchical root and tests kernel root selection
before any signer exec. It does not yet compose the joined launch, install a new
filter, execute an inspected ELF, grant a dummy/real credential or admit custody.

## Exact scope and bounds

Add private Linux x86_64 libtest `diagnostic_signer_root_fixture.rs` and tests.
Reuse the bounded ELF parser through a private path module; only `read_bounded`
in the old runtime fixture gains pub(super) visibility. No old guard/acceptance
bit or caller root/PID/command is consumed. Existing joined launch/helper/policy,
normal library, dependencies, CLI/server, reviewed pins and frozen inputs stay
unchanged. Root files are the same four fixture images, at `/signer`,
`/lib64/ld-linux-x86-64.so.2`, `/lib/x86_64-linux-gnu/libc.so.6` and
`/lib/x86_64-linux-gnu/libgcc_s.so.1`. Copy through fixed isolated tee children,
not writable image descriptors in the parent's fork table. Require exact tree,
readonly 0555 directories/files, single-link regular files and original retained
descriptors/stamps/hashes/ELF graph. No symlinks, `/proc`, cache, preload, hwcaps
directories or caller-controlled subtree. Setup source/ownership is still local
fixture assertion and not independent custody or immutable storage.

The launcher forks only its own child and retains its pidfd/thread-bound owner.
Child uses libc/_exit only after fork. Preserve only selected root and private
report pipe, redirect stdio to fixed /dev/null and close other inherited FDs.
Initial TRACEME/SIGSTOP precedes child CLONE_NEWUSER. At the second exact stop
the owner verifies a new namespace and empty mapping files, writes only the
original nonroot UID/GID single-entry maps and denies setgroups, then verifies
those exact mappings. Parent namespace, credentials and root do not change.
Child fchdir/chroot/chdir enters the owned root, closes original root FD,
drops capability bounding/effective/permitted/inheritable/ambient sets and sets
no_new_privs before its third exact stop. Last capability is read before fork,
bounded to 0..63; no capabilities or descriptors remain as an escape fallback.

At the rooted stop, parent checks exact kernel root/cwd objects, namespace/map
identity, all four real/effective/saved/fs UID/GID slots, zero capability sets,
NoNewPrivs and exact selected descriptor set. It continues only for fixed
read-only path probes: interpreter/libc/libgcc/signer identities, absent host
`/etc/passwd`, absent `/proc/self/root` and absent `/etc/ld.so.cache`/preload,
plus `..` remaining the selected root. A bounded scalar record (not readiness or
authority) reports probes. Exact terminal exit follows; refusal/drop reaps only
the original child. Deadline five seconds, fixed events, no wait-any/PID adoption.

## Checks and falsification

Capture existing runtime ten-case baseline, local UID/kernel/namespace/cap bound
and fixed unshare feasibility (`--user --map-current-user /usr/bin/true`) first.
If unprivileged namespaces refuse, retain that outcome and stop this mechanism
without privileged fallback. Preregister before running that active child probe.
Add missing-owner red, then tree/hash/ELF/substitution/mode/extra/missing tests;
owned kernel positive, map/cap/status decoder faults; wrong root/mapping/sequence
and SIGKILL/drop/refusal/replay tests; seed 0x07403b29, eight probes and four
independent owners. Three focused repeats, prior runtime/joined/standalone/file/
producer regressions, strict current check/lint, MSRV1.88, Windows portability,
normal-library symbols, root contracts/docs/registry/local checks and expected
require-ship refusal. Retain development failures, source/frozen/raw hashes,
article/evidence, separate source/documentation commits and nonforce branch push.

This root observation/probe is not proof of actual GNU loading, complete sandbox,
continuous immutable input protection or production namespace/account custody.
Joining root setup with original descriptor exec, checking actual loader/mappings
and installation, and prohibiting later root/namespace escape remain next work.
Dummy handoff, actual tracer loss, real job/keys, worker P2/P3, all-writer cessation,
later cells, server and qualification remain closed.
