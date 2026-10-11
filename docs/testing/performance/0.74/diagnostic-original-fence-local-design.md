# Existing campaign fence and original Reserved state

Bind the original fixed Reserved-state reader to the existing campaign lock for
a short supervisor-owned observation. Baseline is
`7370c51a0ca23f4f9de1b6fad99dadecc201d16a`. Do not create another lock,
change the installer, transfer ownership, publish a state or authorize execution.

The retained performance-parent descriptor opens the fixed campaigns component.
Require the original intent leaf's trusted owner and private 0700 campaigns
directory; retain its descriptor and identity. Open the existing
`.host-execution.lock` with no-follow, nonblocking descriptor-relative flags,
require regular empty 0600, one link, same device and exact owner, and acquire its
nonblocking exclusive flock. Retain that original descriptor, metadata and named
identity. No public caller-selected path, owner, FD or fixture promotion exists.
Private tests substitute modeled nonroot ancestry, not host enrollment.

The deployed worker-owned 0750 campaigns directory is deliberately refused.
Changing it is not automatic cleanup or a prerequisite that may be silently
repaired. Any future enrollment must coordinate all campaign users while
quiescent and establish supervisor ownership without replacing a live lock.

After acquisition and at each revalidation, bracket the exact original Reserved
reader with fence identity and context checks. Refuse any legacy active campaign,
legacy diagnostic state, pending state/IPC or unhandled IPC ledger in campaigns,
and lifecycle/controller-loss fixture contexts in the performance parent,
including dangling symlinks. The existing Reserved reader also checks its own
fixed leaf. Any unsafe initial inspection or subsequent bound failure latches
the original storage; restoration and wrapper drop cannot refresh it. An initial
Busy result is ordinary contention and does not revoke otherwise healthy storage.
No I/O follows an already latched refusal. Drop releases only the retained lock;
no marker, state or output is removed. The guard is noncloneable and original
thread only and provides revalidation, not cached state, mutation or start.

Tests must demonstrate interoperation with the real HostExecutionClaim and
lock_host_root flock, both lock acquisition orders, named and original descriptor
replacement, unsafe/missing/symlink/FIFO/hardlink locks, directory identity and
mode drift, all conflict contexts, sticky failure and healthy release. Also
demonstrate the limitation: once this short guard drops, legacy campaign admission
does not inspect the fixed Reserved state and can acquire a claim. Therefore this
package is not durable mutual exclusion, trusted publication or complete shared
supervisor protocol. A future publisher must first join campaign admission to
the same fixed state under the same lock; retaining a lock during workload or
using an externally owned lock to stall maintenance is forbidden.

Retain baseline, missing-API red, repeatable targeted tests, legacy fence
regressions, portable checks, Linux lint/MSRV and documentation checks. No frozen
qualification inputs, manifest digest or release thresholds change. Existing
qualification and ship refusals remain real blockers.
