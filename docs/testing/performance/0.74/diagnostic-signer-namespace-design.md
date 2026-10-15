# Post-install namespace root and mount policy in the original child

The private Linux x86_64 fixture extends the old 34-instruction policy to 64
instructions without changing its ABI prefix, exec/task/control restrictions or
prctl tail. Fifteen added pairs return EPERM for all arguments to unshare/setns,
chroot/chdir/fchdir, legacy mount/umount2/pivot_root and seven newer mount APIs.
Existing consumers and old helper/policy bytes are unchanged. A new fixed GNU
helper is a separate test image, not a product candidate or production signer.

The same original rooted-exec owner internally selects only fixed fixture cases,
creates its own root/child and retains its pidfd and thread-bound trace. Root
materialization is shared without duplicating the owner or ELF parser. Exact
original exec, readonly tree/files, four selected VMA identities, bounded startup,
TSYNC header/program bytes at entry and zero exit, and ND0 precede every probe.
The old path retains its 34/272 read ceiling; only the closed new profile uses
64/512. All fifteen operations are also refused during traced startup.

## Observations retained across hardening

ND0 prevents fresh environ and fd-directory opens on the supported local kernel.
The owner therefore opens both only at the exact original exec stop, retains their
original device/inode, procfs/type and readonly/CLOEXEC properties, and never exports
or refreshes them. Environment reads are positional one-byte EOF checks on the
original mm; they cannot authenticate a replacement after exec. The original
trace history and installed exec denial remain necessary. Linux's
[environ implementation](https://raw.githubusercontent.com/torvalds/linux/v6.6/fs/proc/base.c)
holds that mm reference; this is not generic memory attestation.

The original fd-directory is enumerated live through bounded getdents64, not a
saved list or reopened alias. A 512-byte buffer, three reads, eight complete aligned
records and EOF bound the decoder; only dot/dotdot and decimal FD names with fixed
types are accepted. The required inventory is exactly stdio0/1/2. Original task
iteration follows the [proc fd implementation](https://raw.githubusercontent.com/torvalds/linux/v6.6/fs/proc/fd.c).
Other live namespace/root/cwd/exe/maps/UID/GID/capability/task checks remain intact.
Missing/foreign/changed/nonproc/nonreadonly sources, weak flags and decoder faults
refuse; no ND1, privilege or reopen fallback is used.

## Attribution and the negative control

The new filter must produce EPERM for every added native call with unchanged
original context/mappings and exact zero exit. Pure bounded BPF evaluation checks
all native numbers through1023, all-argument denial, bad architecture/x32 kills,
preserved old behavior and exact bytes. Actual rooted x32/i386 calls terminate
with SIGSYS after ND0/core-limit-zero; no core collector or alternative ABI is used.
Kernel filter semantics, including rejection before dispatch, are described in
the [seccomp reference](https://www.kernel.org/doc/html/latest/userspace-api/seccomp_filter.html).

The old-policy controls distinguish chdir("/") zero, fchdir(devnull) ENOTDIR and
setns(-1,0) EBADF from candidate EPERM. The proposed nested user-namespace regain
counterexample is not reproduced: old BPF permits the number but the rooted kernel
call returns EPERM with namespace/capabilities unchanged. Linux documents this
chroot restriction in [unshare](https://man7.org/linux/man-pages/man2/unshare.2.html).
The control is refused/reaped before its failed-prediction helper branch continues.
It is not a successful escape, not readiness and not independent filter attribution.
No unshare(0), root removal, helper edit or relaxed candidate expectation hides it.

Thirteen tests cover policy/model/decoder faults, fifteen native denials, four
controls, actual ABI kills, retained source/flag/context faults, startup/installer/
environment refusal, replay, SIGKILL/drop at four boundaries, eight seeded fresh
probes and four independent single-thread owners. The source-specific
[preregistration](diagnostic-signer-namespace-plan-20261015.md),
[environment amendment](diagnostic-signer-namespace-environ-amendment-20261015.md),
[directory amendment](diagnostic-signer-namespace-fd-amendment-20261015.md) and
[negative control result](diagnostic-signer-namespace-control-result-20261015.md)
retain failed predictions. The [contract](diagnostic-signer-namespace-contract.toml)
keeps mapped-memory/continuous closure, independent custody, complete sandbox,
multithreaded signer, dummy handoff, actual tracer loss, issuer/job/credentials,
worker P2/P3/all writers, completion/later cells and server/qualification closed.
No normal-library API, dependency, signed/durable/wire schema or reviewed pin changes.
