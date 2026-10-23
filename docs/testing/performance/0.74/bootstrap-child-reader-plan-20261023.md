# Bootstrap executable and bounded entry reader

Baseline c9654c84 has parent-owned HCBT0741 material but no bootstrap executable.
Add the binary named by the already selected build contract, without enabling
original start. This is a reader/refusal boundary, not an authentication gate.

The explicit --systemd-bootstrap-entry-v1 mode accepts only the three exact
activation assertions for this process and twelve named descriptors. Proposed
child semantic slots remain 0..14, preserving launcher slots 0..10 and appending
host/launcher/bootstrap-image/entry at 11..14. The reader inspects only entry at
14; names/count alone cannot authenticate the other roles. No parent mapping,
transport, environment cleanup, original owner join or executable handoff is
added. Old launcher and all original live predicates/origins/windows stay exact.

Use bounded positional reads of exactly 1024 bytes, regular/readonly/non-O_PATH,
four seals, initial offset zero and unchanged metadata/flags/offset around reads.
Inherited descriptor flags must be zero rather than parent CLOEXEC: delivery
would otherwise close the descriptor at exec. Do not clear/rewrite flags in the
reader or repair malformed input. Decode to assertions only and compare a second
fresh observation. Even valid input exits 2 with authentication explicitly closed.

Test ingress shape, actual private memfd refusal/retention, and fresh synthetic
processes that reach reader refusal or the still-closed authentication boundary.
Forged canonical input must never execute anything. Run Windows, GNU 1.94 and
MSRV 1.88 focused tests/check/strict lint, formatting and local contracts. Retain
the first absent-binary failure and every later failure separately. No original
live retries, measurements, services, clock changes, server or product workload.

Still required: authenticate every inherited role and original manager/root/
guardian/credential context, bind this to consumed parent ownership, independently
enroll the actual built bootstrap, and prove one-shot execveat plus all-writer
cessation before SIGKILL/lost-ACK, completion or server admission.
