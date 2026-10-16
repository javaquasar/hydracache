# Original collector Reserved publication evidence

Baseline is `13dac632d91bb029821b9f3a2678e13694222130`; the
[preregistration](../../diagnostic-collector-reserved-plan-20261016.md) is
`c39882b4`. Implementation/final source is
`6c49e3f9b4ec0241114f08e023affc29c5917bb2`, tree
`acf87ed5e4792ae3ab4d2af554306676eadf47f6`.
See the [design](../../diagnostic-collector-reserved-design.md) and
[contract](../../diagnostic-collector-reserved-contract.toml).

## Confirmed publication boundary

The observed initial state and signed collector requirements now have a separate
durable record under the original request, kernel/worker/authority/controller
and fence. The consuming publisher retains the original request descriptor,
bytes/stamp and same held lock; it never opens a second authority journal or
reconstructs an old guard to inspect two records. Legacy storage and publication
transfer bodies stay byte-identical after excluding only the exact new child
registration. The new COMPAT record does not reinterpret old regular-file intents.

Its strict canonical record binds the complete original controller-attempt hash,
frozen state and signed collector envelope. Actual runtime authentication remains
under the original live owner. The paired byte inspector checks consistency only;
a validly encoded zero signature can pass it without becoming an acknowledged
guard. Twenty targeted record mutations refuse. New schema/extra fields,
noncanonical/trailing bytes, malformed/over-budget pairs, request/state/window,
epoch and unit substitutions are not repaired or adopted.

The exact inventory changes from one original request to two owned documents.
Nine named checkpoints bracket exclusive creation, initial directory sync,
partial/full writes, sealing, file/directory sync and acknowledgement with live
original signature/authority/clock checks. Logical seed 0x0740c09a exercises every
failure boundary. Refusal retains the original lock and partial bytes until drop;
later refused observations perform no I/O. Returned success acknowledges initial
retention, not Starting, controller heartbeat, execution lease or output release.

## Development captures

The unchanged observed-reservation baseline passes six tests. The new API probe
fails with E0599 for missing publish_reserved; its unused test import warning is
also retained. The first API-shape green has a temporarily unused private test
hook/import, not final strict-lint evidence. After the actual tests are added,
the clean build-probe passes. The first positive test reaches all nine stages,
checks the returned guard, frozen state, unchanged request and retained fence.
Its elapsed time is functional test reporting, not a product performance result.

The focused group passes ten tests, with the separately gated process matrix
ignored there. It covers all nine injected failures, twenty byte mutations,
occupied names, changed original/new files, directory ancestry, worker/OIDC
source drift, pre-refusal/missing records and independently owned attempts.
Independent threads use distinct private modeled roots; they prove local refusal
isolation, not simultaneous publication on one host. Same-host exclusion is
checked through actual common-lock contention at the publication checkpoints
and before each process-loss case.
Current/MSRV signed-projection regressions and fifteen compile-fail examples
pass, as do eight Linux/Windows build/strict-Clippy lanes and 173 static tests.

Automatic registry discovery remains at 176: that scanner enumerates integration
test targets, not nested lib-unit manual tests. This lib SIGKILL proof is an
explicit external_tool/manual registry entry, with its exact command/source
checked by the new static gate. The only historical-registry exclusion is that
exact appended entry, not arbitrary registry changes. The separate preregistered
1800-second manual-job harness limit changes no existing packet/request,
controller-loss or qualification deadline.

## Scope of the local process proof

All new checkpoints already follow durable request retention. BeforeCreate must
therefore refuse fresh admission too; it is not the empty-before-original-write
case. Fresh inspectors cannot turn partial or complete surviving bytes into a
runtime acknowledgement. The process harness owns every temporary root and
unreaped child pidfd, signals only that child, checks actual SIGKILL status,
original lock release and unchanged origin/authority snapshots, then performs
two fresh read-only inspections. The uninterrupted control separately checks
normal context cleanup. No writer is restarted or adopted after death.

The manual Rust 1.94 proof passes all three preregistered repetitions. Each has
ten actual SIGKILL cases (nine stages plus AfterReturn), one uninterrupted
control and twenty-two fresh inspections. Total: 30 kills, 3 controls and
66 inspections, including BeforeCreate refusal in every repeat. Original
origin/authority snapshots stay unchanged; lock release after death and
fresh-process no-adoption are confirmed. The same final source and unchanged
packet/request windows are used for all repetitions.

Final source passes the complete original-input group on both Rust 1.94 and
Rust 1.88: 92 tests each, with the old and new manual matrices ignored there.
Each compiler separately passes 30 signed-projection tests, 26 kernel tests
(one existing manual case ignored) and fifteen compile-fail examples. The
negative API examples fail on the intended move/trait/method/argument boundaries,
not missing imports or private-module errors. Neither original-input log has
an unexpected first-refusal trace. Kernel logs include the intentional unwind
classifier canary. All 173 static tests and eight affected build/lint lanes pass.
Formatting, documentation, automatic gate registry and local non-promotable
performance contracts pass; ship admission is expected-red. The packet retains
31 command captures and 33 hashed artifacts, including development reds.

Keys and OIDC statements are synthetic; fixed storage paths are modeled under
nonroot temporary roots. Sender/message/time observations are actual local
Linux kernel facts. There is
no positive installed-root enrollment. The process test does not prove power-loss,
reboot, whole-host rollback or a global nonce authority. In-memory refusal is
not represented as durable epoch revocation. Historical Kernel(Invalid) remains
unexplained; expected classifier panic output is a canary, not a reproduction.

Starting, actual authenticated controller progress, root output construction,
original executable/streams/process/manager/cgroup start, watchdog/all-writer
cessation, completion, later cells and host rehearsal remain pending. Revalidating
the record cannot renew controller progress. Product and frozen qualification
inputs, thresholds and release status remain unchanged; ship admission stays red.
No server, workflow, numerical product workload, full release verification or
push is included.

The manifest hashes normalized UTF-8/LF logs, source objects and unchanged inputs;
empty output is represented by one final LF. Its verifier is unsigned consistency
checking, not binary reproducibility, a clean-checkout build or signed execution
attestation. Dependencies and compiler caches are reused.
