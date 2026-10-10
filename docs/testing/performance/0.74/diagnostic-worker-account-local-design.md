# Bounded fixed worker account observation

## Scope and sequence

Baseline `0ba5811cdb3f5053dca524e88009607a17fdad99`. Add a Linux read-only
NSS helper for the fixed user and group `hydracache-perf`, using the existing
manager helper envelope. Baseline the supervisor library, artifact and manager
tests before test addition, capture the missing API refusal, implement and retain
repeated checks. This is an account observation, not trusted host enrollment,
production output preparation, signed start or qualification admission.

## Identity and helper boundary

Only the fixed canonical schema-1 request is accepted. Parent execution uses
`/proc/self/exe diagnostic-worker-account-worker`; no account selector, NSS root,
command or helper pathname is accepted. The existing envelope clears inherited
environment, sets close-on-exec for non-stdio descriptors, bounds combined
stdout/stderr to 65,536 bytes, applies a 2,000 ms operation deadline and retains
typed errors, partial output and pending cleanup. A cleanup uncertainty blocks
the next helper operation. Child address space, CPU and core limits are unchanged.

Use reentrant forward and reverse user/group lookup with fixed 16 KiB caller
buffers. Refuse not-found, NSS errors and ERANGE without buffer growth or retry.
Validate the returned record pointer and bound name inspection within its buffer;
export no password, home, shell, member names or raw NSS buffers. The user primary
GID must equal the fixed group's GID; reject root or sentinel UID/GID.

Capture at most 32 group memberships with one getgrouplist call per round.
Refuse exhaustion, invalid counts, sentinel or duplicate IDs; sort for canonical
comparison and require inclusion of the primary GID. This is the NSS membership
list, which includes the primary group, not a proof of a process's actual
supplementary groups. The [GNU libc group API](https://ftp.gnu.org/old-gnu/Manuals/glibc/html_node/Setting-Groups.html)
documents that distinction. A fixed caller array does not bound NSS-internal
allocation; the child resource limits and parent deadline are the outer bounds.

Two complete forward/reverse/membership rounds must agree. The snapshot decoder
rejects unknown, duplicate, malformed, noncanonical, future or over-budget input.
A runtime account guard retains the original snapshot; errors or any mapping
drift latch refusal without adopting later observations. No refresh/reset exists.

## Tests and remaining authority

Private deterministic lookup seams cover missing/invalid results, every UID/GID
and name mismatch, membership capacity/duplicates/order, both rounds, stable
sorting and mapping drift. Exercise wire rejection, first-error preservation,
sticky refusal and independent guards. Existing process-envelope tests continue
to cover deadlines, output overflow, environment clearing and pending cleanup.
Add ordinary integration checks of the real fixed helper request without creating
the account; missing account is an explicit refused observation, not success.

NSS executes in the helper's current host/user/mount namespace and uses its local
configuration. These observations do not authenticate a provider, freeze NSS
configuration, certify the host namespace, establish all-thread credentials or
enroll a production worker. Connection to the existing kernel credentials guard
requires a separately reviewed trusted host policy and actual group semantics.
No live server/IPC route, unit/account mutation, host install, product process,
workload or qualification is enabled. Matched parent/worker deployment is needed
for the new read-only helper mode; older binaries refuse it.

## Disk maintenance before baseline

Only this worktree's `target/debug/incremental` and
`target/canary-sweep/cargo-target` were cleaned with Cargo after checking exact
paths, no tracked descendants, no reparse points and no active compilers.
Cargo reported 111,829 files / 68.9 GiB and 12,533 files / 14.5 GiB respectively.
Free C: changed from 13,487,009,792 to 87,652,687,872 bytes at observation time.
This is not an exact physical deletion claim: NTFS compression, links and other
writes affect the result. Current Windows/WSL caches, the canary process capture,
all worktrees and the previous verified evidence packet were preserved. Deleted
caches are recoverable by rebuilding, not trash recovery.
