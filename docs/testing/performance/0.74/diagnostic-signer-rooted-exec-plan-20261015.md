# Owned original exec and GNU mapping observation inside the selected root

Baseline: `2072b30ef4084cb910712450dc87b0529b3f80fa`. Test one composition:
child-only selected root, original descriptor exec, selected kernel-reported
file mapping identities and exact existing filter installation in one child.
Keep the fixed Rust1.94/GNU helper and 34-instruction policy unchanged. Later
root/namespace escape restriction is a separate slice, not silently part of this
hypothesis. No credentials, server job, product workload or qualification.

## Source and owner scope

Add private child module `diagnostic_signer_rooted_exec_fixture.rs`, bounded maps
parser and tests under the existing Linux x86_64 root libtest cfg. The root module
gains only its child-module declaration. Its old path-probe owner/child, root
creation, retained fields and cleanup stay unchanged. The new owner creates and
owns its own Root internally; it never accepts an asserted guard, caller root,
PID, descriptor, command or acceptance bit. Private parent helpers supply bounded
text/map/status/root checks only. No public visibility or guard/FD export.
No dependency, normal-library, CLI/server, durable/wire schema or reviewed pin
changes. Old helper, joined/standalone policy and qualification remain exact.

Prepare fixed enum arguments and empty environment, with three original FDs:
/dev/null stdio, retained selected root and selected signer. Close all other
descriptors before TRACEME/SIGSTOP. Own pidfd/thread-bound trace precedes child
CLONE_NEWUSER. At the next exact stop, require a new namespace and empty maps,
deny setgroups and write only the same nonzero UID/GID single maps. Child enters
the original root/cwd, closes its root FD, drops all capability sets and enables
no_new_privs, then stops. Verify root/cwd/namespace/maps/status/task and exact
stdio+selected-executable FD set before original execveat/AT_EMPTY_PATH.

At PTRACE_EVENT_EXEC require original signer object, root/cwd/namespace/credentials,
empty environment and only stdio. All checks bracket retained root/input checks.
At that stop only selected signer/loader file identities may appear. GNU startup
is observed through existing native syscall decoder, with at most 256 pairs and
0..1023 native numbers; extra exec/task/control, root/namespace/mount changes,
unknown ABI/signals/sequences refuse before dispatch. At the exact seccomp entry
require TSYNC and fixed 272 bytes under original stack/header/program addresses
captured once. Require exact zero exit, memory recheck, original executable/root,
one task and selected four-image map identities. Observe exact PR_SET_DUMPABLE/0
entry/zero exit, then existing self/different exec denial and zero terminal exit.
No ready string or old observational acceptance manufactures authority.

## Bounded mapping observation

Parse at most 65536 bytes and 128 sorted, nonoverlapping mappings. Require bounded
hex/decimal fields, checked ranges, 4096 alignment and private supported permissions
with no WX. File-backed regions must match original retained device major/minor
and inode, use offsets/ranges inside rounded file size, have expected fixed root-
relative or actual temporary-root pathname, and include executable regions for
each required image. First exec allows exactly signer/loader; installation entry
and zero exit require all four. Unknown/missing/foreign/equal-byte inode, deleted
or ambiguous path, unexpected file/anonymous executable map and unknown pseudo
mapping refuse. Permit only fixed observed Linux pseudo forms: non-X anonymous
private regions, [heap], [stack], [vvar], [vvar_vclock], one RX [vdso] and the
fixed --xp vsyscall page at ffffffffff600000..ffffffffff601000. Require one RW
non-X stack and one vdso. A parser mutation is not mapped-memory attestation.

This uses kernel-reported VMA file identities, not privileged map_files reopening,
relocation-byte validation, loaded-memory attestation or complete/continuous runtime
closure. Root/input custody remains fixture assertion. No future dlopen, TLS,
network/config or arbitrary helper-code guarantee; no post-install root/namespace
filter change, multithreaded signer, handoff or actual tracer-loss claim.

## Falsification and checks

Capture seven root and nine joined baselines, then missing-owner compilation red.
Add first-exec/installation positives; unsupported map/header/field/range/permission/
path/identity/pseudo/missing-image mutations; initial/rooted/first-exec/install
SIGKILL/drop; phase/replay/namespace/env/tree/hash/refusal; old installer flags/
bytes/length/heap-pointer and pre-install exec faults. Seed 0x07403b2a: eight
fresh rooted execs and four independent owners. Keep descriptor exec probes
deferred because the unchanged helper's fd mode opens /proc/self/exe, absent
from the mount-free selected root. No /proc mount, helper edit or target fallback.

Three focused repeats, old root/runtime/joined/standalone/file/producer regressions,
Windows portability, strict current checks/Clippy, parent-crate MSRV1.88,
normal-library symbol exclusion, root contracts, format/docs/169-gate registry,
local non-promotable check and expected require-ship refusal. Retain failures and
raw/source/frozen hashes; update article/docs; commit source separately from evidence
and nonforce push feature. Change the old root gate filter to its exact old tests
module so adding a nested fixture does not mislabel its seven-case count.

Only after this result evaluate late namespace/root mutation denial, then dummy
handoff and actual owner-loss boundaries. Independent production custody, worker
P2/P3, all writers, later cells, server and expensive qualification stay closed.
