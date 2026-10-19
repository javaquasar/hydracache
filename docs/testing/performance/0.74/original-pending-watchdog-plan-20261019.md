# Original pending watchdog custody

Baseline is `f94b4b56`. The ready original context now retains its real message,
receiver, channel, origins and descriptor roles, but no guardian can be created
from it. Connect only the Pending stage to that original borrow. Do not enroll a
target, send the original message or reinterpret v1 consent as v2 launch authority.

Add a private kernel-mechanics adapter that borrows existing fence/timer FDs,
inherits those exact OFDs into two independently created guardians and reuses
the unchanged credentialed Pending protocol and child loop. No timer is created,
consumed or rearmed, no flock operation is performed, and no target PID/FD enters
this adapter. Retain its creating PID/TID/credentials, same timer aliases, owned
atomic clone3 pidfds and channels. Parent-side cancellation uses those pidfds,
not a PID-only signal, and owns cleanup before signal restoration can fail.
This low-level adapter is not an original-admission constructor. It exposes no
enrollment, send, release, raw-FD getter or completion operation.

Consume the exclusive OriginalWatchdogContext into a private pending owner.
Derive the borrowed FDs from the still-held original objects, never caller claims.
Perform the unchanged complete original context round before and after kernel
preparation and on revalidation, with pending-state observations on both sides.
Keep the original collector/custodian, payload aliases and channel in place.
Failure poisons that same original chain; partial setup is cancelled while the
original borrow and original fence owners remain live. Preserve every predicate,
all 46 old effect brackets and the original 300/60/10-second origins.

Retain the previous closed packet as baseline and rerun its original fourteen
frame controls before implementation. Keep a missing-module red receipt first.
Add current/MSRV ordinary and explicit nonroot full-frame controls for healthy
Pending/revalidation, actual guardian death, partial clone/restoration failure,
context drift and unchanged records/timer/fence/descriptor exclusion. Test probes
must remain distinct from the real fixed-channel original owner. Freeze all 376
previous supervisor Rust bodies and old wire/helper bytes with exact additive
registrations. Run strict lint, formatting, Windows/API, full source contracts,
registry/docs and the local non-promotable contract. Record committed-source,
executed-image and fixed-input receipts, then update design and the article.

Pending is not Armed or proof of responsiveness: a stopped guardian may retain a
nonterminal pidfd. The existing child loop parks with its inherited fence on
owner loss/expiry without any target to revoke. There is no common-mode failure
guarantee, durable exclusion, all-writer cessation or production enrollment.
Target/artifact/process/cgroup/stream binding, original delivery, completion,
later cells, full new-boundary SIGKILL/lost-ACK coverage and reconnect remain
subsequent gates. No installation, supervisor operation, dedicated-server action,
product workload or expensive qualification is authorized by this package.

## Local result

Source `4b946bbeee1341088c2a1af9504ae3ae4f57da60` implements the consuming
original Pending owner and private borrowed-FD mechanics. GNU 1.94 and MSRV 1.88
each pass four kernel controls, one constructor type control, ten new full-frame
cases, fourteen retained context cases, four retained context unit controls,
thirty exchange controls and seventeen guardian regression cases. Healthy
preparation retains 68 authentication calls, 68 signature calls and four channel
checks; revalidation retains 34 of each and two channel checks. Original records,
origins, absolute timer, 300/60/10-second offsets and fence custody remain checked.

The full source suite passes 274 controls, preserving all 376 prior Rust bodies,
old wire formats and imported harnesses through exact additive registrations.
Windows passes 49 unit and 17 compile-fail controls. Both compilers pass all-target
checking and strict all-feature Clippy. Formatting, registry/docs and the local
non-promotable contract also pass. The [closed evidence packet](local-runs/original-pending-watchdog-20261019/README.md)
binds source, executed image, runner and unchanged fixed inputs for each cohort.

The old-image baseline and two later clock-preflight refusals are retained.
They observe real backward CLOCK_REALTIME steps; no original admission starts
in the refused preflights. A separate bounded workstation stabilization phase
precedes the byte-equivalent original strict clock probe and all fixture creation.
It neither changes clocks nor renews a request. Every actual frame still runs
the unchanged live predicates and original bounds. Local WSL timesyncd is restored
and verified after each gate; Windows/server time settings are untouched.

This closes Pending custody binding only, not Armed or production enrollment.
The adapter has no target/send/release/completion operation. Best-effort Drop
cannot supply a cessation receipt. Trusted v2 artifacts, actual target/stream/
cgroup binding and all-writer revocation remain before original delivery,
durable completion, later cells, full new-boundary crash/lost-ACK coverage,
reconnect or server admission. No numerical performance improvement is claimed.

Final evidence sealing also fixes a local culture-sensitive BOM-prefix defect.
The new packet uses ordinal comparison and six prefix/encoding controls while
retaining the failed inventory. An append-only audit separately binds all 182
unchanged files in four affected historical packets to their full Git baseline;
their original files are not modified and their legacy verifiers must not create
new inventories. This integrity repair changes no product signature, binary
digest, original frame predicate or qualification artifact.
