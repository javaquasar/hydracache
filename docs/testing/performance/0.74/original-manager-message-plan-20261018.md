# Original uniquely addressed message preparation

Baseline is `e673bf4c`. Consume the private OriginalManagerJoin into a new
nonexecuting owner. Derive manager/client names only from its retained fixed
connection, never from caller arguments. Keep the old well-known-addressed
message, original ten-descriptor inventory, all children, alarms and custody
unchanged and unsent. Add exactly two explicitly owned message writer aliases;
they must close before the original owner can drop its final custody alias.

Build a new METHOD_CALL to the retained unique manager with the exact old
canonical body and original stdout/stderr pipe objects. Require the old signed
property validator and byte-identical canonical body, correct envelope/signature,
two typed CLOEXEC write-only descriptors in their original order, and stable
actual alias FD numbers. Retain the original message's serial in the new owner
and reject replacement/reconstruction. There is no connection/message/name/FD
export, callback, deserialization, Clone, send/start/completion or retry API.

Use the unchanged live manager/full-frame round before and after preparation
and every later message validation. This adds two full rounds; it does not
remove any check or refresh any origin. Keep Reserved 300 s, Starting 60 s,
progress/challenge 10 s, original alarm and operation limits unchanged. Any
failure poisons the same original owner and retains the entire failed candidate.

Test canonical body/envelope and FD mutations with pure local units. A distinct
test-channel fixture exercises the shared round with the real original frame;
it cannot construct a production join. A test-only adapter may reuse the old
owned SCM_RIGHTS receiver to prove actual transfer of this exact message and its
original writers. That is same-process socket delivery, not negotiated system-bus
delivery or authenticated systemd start. Do not send StartTransientUnit to a
live manager, enroll trust, install a release or operate the stable supervisor.

Record baseline and expected missing-constructor compile failure, then fresh
current/MSRV images, targeted units, source equivalence/mutation guards,
all-target checks, strict Clippy and portable controls. Use the already approved
isolated nonroot physical-host fixture route for full frames; retain exact
boot/PID/install/receipt and no-campaign pre/post guards. Do not retry or promote
the previous stopped local Time cohort. Stop a fresh cohort on any unexpected
failure, attribute it separately and preserve raw evidence before a correction.

No new durable write boundary or active writer/process is introduced. Active
SIGKILL/lost-start-ack, authenticated process/cgroup identity, active watchdog,
whole-writer cessation and completion remain later gates. Qualification inputs,
release status and product admission remain unchanged.
