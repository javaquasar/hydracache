# Ordinary dummy transfer creator loss

Baseline is source 89f81f454bfc5fcdb2d87619337d141a1dd95452. This local W11
experiment composes the ordinary selected-root dummy protocol with the existing
original-process monitor. It introduces no credential or production start API.

## Fixed experiment

Six cuts are installed, READY received, original recv syscall entry before the
sole send attempt, original recv return with dummy bytes, ACK received and
observed close return before exit. Each has creator-loss and healthy continuation
controls. Installed and delivered additionally have matched group-stop controls
with EXITKILL removed. These two losses must use the same proven group-stop;
their surviving negatives require adopted/untraced State T before explicit kill.
The other four losses are stopped ptrace cuts, not running-child evidence.

The monitor acquires the original Root and creates the exact socket pair before
the outer fixture-only creator fork. Its original endpoint objects bind the
transferred parent endpoint and the child's initial O_PATH FD3 metadata; unlike
a pipe, opposite socket endpoints have different inodes. The ordinary Owner uses
that fixed private transport through its existing raw child path. No caller can
supply an endpoint, PID, path, payload or arbitrary cut. The monitor drops its
child-end copy before the G gate and holds the parent peer through creator reap.

The existing credential-checked setup/cut frames, two transferred FDs, G/H gates,
four-packet budget and five-second waits remain. Passing losses signal only the
original creator pidfd and require exact creator and adopted worker SIGKILL,
WNOWAIT/reap/ECHILD, socket EOF, unrelated original child survival and original
root cleanup in that order. Healthy H resumes the original fixed protocol to
ACK/close/zero exit and ordinary reap. Negative cleanup is never automatic-death
proof. Unknown quiescence preserves the original root.

## Verification and exclusions

Run the existing 61-test targeted baseline, then a red integration test against
the unchanged executable. Add closed selector/frame/binding/cut-state units,
all 14 real-process controls, environment/extra-argument refusal and MSRV's
attributed extra-libm refusals. Repeat current-runtime targeted checks; run old
root/isolation regressions separately, current strict lint, MSRV/portable checks,
formatting, document/static registry and closed performance-contract checks.
Retain failures and final logs in a new local packet, with source/frozen-input
digests and a consistency verifier. Do not overwrite historical packets.

Raw worker fork, root graph, policy/installer, startup 256 pairs, protocol 16
events, image 8 MiB, proc 65536 bytes/maps 128 lines, 48-byte drain and existing
five-second outer guard stay fixed. No timeout or admission threshold waiver.
Pre-arm, running/thread-only loss, fresh creator CLI exec, hostile same-UID
custody, trusted credentials, authenticated worker original-start, P2/P3 writer
cessation, completion/later cells, server/qualification and performance claims
remain excluded. Earlier overlapping-load timeouts remain unresolved.

## Fixture scheduling correction

The initial separate new process test passed all 14 modes alone. In the full
default-parallel group, the old ordinary handoff test then hit its unchanged
five-second outer guard while the new third root/policy matrix was also active.
There is no scheduler trace establishing the cause; retain this failure rather
than call it fixed by an isolated success. A development compile also rejected
comparison of FixedProbe (no PartialEq); use pattern matching without changing
the shared enum.

Before the next candidate, place the new 14-mode helper after the existing 25
handoff cases in the same ordinary process test. Reuse that original Image and
keep the baseline number of concurrently active process-test fixtures, default
parallel harness and exact output guard. No extra target, ignored test, serial
harness flag, lock spanning process lifetime or timeout increase is introduced.
This controls the experiment's added fixture load; it is not a product change
or a proof of overlapping-load robustness. Final targeted count becomes 63:
43 root units, five child-entry, seven policy, one CLI and seven process tests.
