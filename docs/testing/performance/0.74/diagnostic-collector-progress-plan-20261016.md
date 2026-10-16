# Authenticated collector progress before Starting

Baseline b9fb32e1da407d16b7073fbc94820667f3bfdfab. Initial collector
Reserved is retained, but its repeated observations are not controller progress.
Add one consuming, non-executing preflight for a newly received signed Heartbeat.
Do not publish Starting, change the initial state or renew admission freshness.

Consume the original collector reservation and a kernel-owned packet. Retain both
on success and failure, including the original request, policy/epoch journal and
common fence. Require the same receiver origin by private object identity, not
matching caller-supplied host fields. The packet must have been received strictly
after the owner's last observation. Authenticate its actual sender and fresh
kernel time with the original configured controller policy. Require Heartbeat,
the same controller/identity/source/preset and revision 1, and request id and nonce
different from the original Reserve. The new owner has no second-progress method,
extraction, cloning, decoding, start or durable-publication route.

Each revalidation brackets the progress check with the unchanged reservation
checks. Keep the original 60-second packet hold and original request/collector/
OIDC windows. Check the new packet's own hold and signed window as well. Freshness
uses the original receive BOOTTIME, exclusive ten-second controller-loss deadline,
ordered boot/monotonic/realtime samples and the unchanged 300-second total bound.
Never replace a receive point or count revalidation as another heartbeat. This
is a preflight under still-valid admission, not a runtime lease or durable replay
ledger. After owner loss the retained context continues to refuse fresh admission.

Keep old method bodies exact; use explicit child-module registrations for private
kernel/worker extensions and the public owned preflight. No new wire or durable
schema is needed: existing signed DiagnosticRequest v1 is only inspected. It
cannot substitute for a future signed Starting projection or commit/replay protocol.

Capture an existing targeted baseline, then an API compiler red before runtime
implementation. Test real same-receiver messages, immutable files/state and held
fence, validly signed operation/revision/identity/controller/id/nonce substitutions,
wrong key, different receiver, pre-received/stale packets, source drift and sticky
cross-refusal. Exercise exact deadline and clock-order boundaries privately without
waiting or changing production budgets; distinguish those modeled cases from real
kernel observations. Check consuming/no-start/no-replacement API constraints.

Run current/MSRV focused tests, original-chain and projection regressions,
check/strict Clippy for affected packages, format, static/doc/local contract gates
and expected-closed ship admission. Retain source-pinned commands and results;
amend historical source guards only for exact registrations. No server, workflow,
product workload, qualification, push, root output creation or process launch.

Next is a separately preregistered consuming Starting publication with its own
durable replay/uncertain-outcome handling and real SIGKILL/lost-ack boundaries.
Runtime authority beyond the original admission hold, authenticated original start,
autonomous watchdog and all-writer cessation remain open.
