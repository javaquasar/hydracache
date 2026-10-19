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
