# Actual tracer loss at original handoff boundaries

Baseline `ad5c4c8681eabde410d4195cdadb9bad33afa323`. Test actual
SIGKILL of the original launcher/tracer, not a simulated observation failure.
Keep existing GNU helpers, policy bytes, root/exec owner, handoff protocol and
all production authority unchanged. No key/token, host service/mount/account/
cgroup change, server contact, product workload or qualification.

## Separate original observer and private report

Add private `diagnostic_signer_tracer_loss_fixture.rs`, `_channel.rs` and
`_tests.rs` beneath the handoff fixture, with only a module declaration added to
the existing handoff source. Re-exec only the current libtest executable with
one exact test name and closed monitor/tracer roles, stage and fault selectors.
Clear environment, preserving only local HOME for the fixed helper compiler and
an internally created temporary parent. No caller command, root, PID or FD API.
Do not allocate Rust state after raw fork: use Command plus scalar-only pre_exec.

The outer test creates a new process group containing its fresh monitor, tracer
and child. Only the isolated monitor sets PR_SET_CHILD_SUBREAPER; never change
the ordinary test process or host. It creates the report socketpair and original
tracer subprocess/pidfd. Tracer keeps the existing original RootedExec owner and
child/pidfd, advances to one of initial/rooted/first-exec/installed/ready/delivered,
then sends one fixed report and waits. No original child is resumed after report
until a closed healthy/refusal response, or actual tracer death.

The report is 32 bytes: magic `HC074-LOSS-v001!`, stage byte, dummy-send count
(one only for delivered), two zero reserved bytes, native little-endian tracer
TID u32, and eight zero reserved bytes. SCM_RIGHTS carries exactly five original
readonly/CLOEXEC descriptors: child pidfd and four original root directory FDs.
SO_PASSCRED supplies one kernel SCM_CREDENTIALS matching the originally spawned
tracer PID/UID/GID, not the socketpair's pre-fork SO_PEERCRED. Use an aligned
128-byte control buffer, MSG_CMSG_CLOEXEC|MSG_DONTWAIT|MSG_TRUNC and exactly32
data bytes; scan/own all kernel-delivered FDs before refusal so they close even
for truncated/wrong/duplicate control. This is a test-observer report protocol,
not a change to the production ancillary receiver, which still rejects RIGHTS.

Require original pidfd type/fdinfo, actual child PPid=original tracer TGID,
TracerPid=reported original thread, and that thread's Tgid=original tracer.
No PID-only liveness or caller-PID adoption. Directory descriptors must bind
the single original temporary root below this monitor's original temporary
parent and the exact four directory names/inodes, readonly/CLOEXEC/0555.
All report/identity checks precede fault injection.

## Death proof and cleanup

The positive path sends SIGKILL only through the original tracer pidfd. Require
original tracer SIGKILL, child pidfd terminal readiness, then exact P_PIDFD waitid
CLD_KILLED/SIGKILL from the isolated subreaper, consume that original terminal
record and require ECHILD on repetition. Only afterwards chmod the four retained
original directories for fixture cleanup; no pathname substitute or host root is
mutated. No helper replacement or automatic retry is permitted.

Use5-second logical-stall refusal for report/death waits,30 seconds for the outer
subprocess bound and8192 bytes for captured nonsecret libtest output. A separate
emergency guard may kill only the fresh unreaped outer process group or original
tracer/validated child pidfds on failure. Such cleanup is failed evidence, never
a successful EXITKILL observation. Preserve failures and any cleanup limitations.

Add an exact same-child negative control that removes only EXITKILL at installed,
ready or delivered. Predict natural exits73/74/76 respectively when tracer death
closes the private handoff endpoint; these are not SIGKILL or a demonstrated live
orphan. Preserve an incorrect prediction instead of changing the helper. Healthy
delivered control resumes exact ACK/close/zero exit under the original owner and
must leave the monitor's temporary parent empty. Wrong-stage, short report,
missing-FD, self-pidfd and wrong-directory reports must be refused before kill
admission; a private reject byte asks the still-trusted tracer to refuse/reap its
own child. No unvalidated received FD is ever used to signal a process.

Test actual six-stage loss, three controls, healthy completion, report refusal,
bounded frame mutations with seed0x07404a12, eight fresh cases and four independent
monitors. Keep each original single-thread signer; no production multithreading,
mapped-memory attestation, independent custody or whole-writer revocation claim.

## Gates and retained evidence

Capture handoff13 baseline and missing implementation red first. Run three new
focused repeats, old112 regressions, strict current package/xtask checks/lint,
MSRV1.88, Windows portable9/check/lint, scoped format, source contracts, unchanged
gate registry, docs, normal-library exclusion, local contract and expected-red
require-ship. Register this private test report in COMPAT only. Source and raw
evidence/article commits stay separate; new packet begins with no-text attributes
and verifies source/frozen hashes, mutation canaries and exact committed Git bytes
before nonforce feature push. Older packets and qualification digest stay exact.

Actual test-only EXITKILL does not enroll a signer job/runtime/issuer, read a key
or token, integrate the authenticated producer/receiver/replay/fence, implement
worker P2/P3 original-start or all-writer cessation, publish completion/cells2-4,
or open dedicated-host/release/performance admission. The pre-options constructor
window is outside the positive proof; outer owned-group emergency cleanup covers
test failure there, not production custody or admission.
