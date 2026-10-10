# Local original leader mount namespace evidence

This packet retains local consistency and regression checks, not signed mount
enrollment, all-thread proof, product performance or qualification.
See the [design](../../diagnostic-mount-namespace-local-design.md) and
[manifest](manifest.json). Raw logs are closed captures copied byte-for-byte,
including missing-API failures and expected-red ship admission.

The original process is borrowed; worker/observer mount objects and worker ns
directory are retained with read-only CLOEXEC FDs. The original observer TID is
required and the reader is neither Send nor Sync. Wrong type, changed objects/
directory/TID or original process exit refuses without refresh or mutable process
authority. Namespace identity is not mount-tree content immutability.

Linux passes 356 with one existing system-bus ignored; Windows passes 43.
Root contracts/evidence/governance pass 134. Sixteen new tests pass three extra
repetitions. Seed `0x7582026` supplies 256 modeled iterations of first-error and
identity drift in final suites. The first green preceded the extra seeded identity
cases and is retained as an earlier capture.

Actual owned local helpers test real mount descriptors and exit. Private TID,
directory-pin and distinct device/inode mutations are modeled failures, not a
foreign-namespace or setns/unshare experiment. No host mutation, workload, signed
composition, real issuer or authenticated original start is authorized. Frozen
qualification manifest bytes and actual contract inputs remain unchanged.

Run `python verify-packet.py` for read-only log inventory/hashes, repeated test
results, closed authority boundaries and frozen qualification digest checks.
