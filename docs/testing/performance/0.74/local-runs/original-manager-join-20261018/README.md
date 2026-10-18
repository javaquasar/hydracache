# Original receiver / live manager validation join

Private nonexecuting source experiment; no server admission or product claim.
The [preregistered plan](../../original-manager-join-plan-20261018.md) fixes its
scope. `manifest.json` binds the source-only commit, 18 changed-source capsules,
340 complete package/build inputs, exact compiler-artifact images and the closed
raw evidence inventory. `verify.ps1 -SelfTest` is offline and read-only.
Packet-local attributes preserve raw evidence bytes. Archived launcher scripts
retain their original trailing blank line; only blank-at-EOF lint is excluded
for these scripts, not any Rust formatting or runtime/source contract check.

Accepted source slice: `698f808bd088ba7841336aa82ad3f2acbcf53cfe`, immediately
after plan-only `3e7a70a83c478da6869701d5ce78adda8a09ab16`. The fresh physical-host
gate completes on both compilers: six healthy controls, twenty live refusals
and two real nonroot fixed-channel refusals in total. Actual local read-only
channel/context gates pass separately on both compilers. Installed supervisor,
boot, binary and provisioning receipt remain unchanged; no helper remains.

Final GNU test images (test executables only, not installed product candidates):

| Compiler | SHA-256 |
| --- | --- |
| 1.94.0 | `d158588159072818ca9f3f80ccecabd5c169402ec9fef002e9a699754d4d9a15` |
| 1.88.0 | `567bd06dc50b37174875d60cb5e4412edf77a6748fcb1464fd078630c39b3ee5` |

## What changed

`OriginalEnrollmentFrame::bind_fixed_manager` consumes the actual original frame.
Its private owner retains that frame, its children, writers and custody, one fixed
live channel, a retained original receiver binding and a caller mount pin. No
channel/path/factory/context argument, serialized constructor, owner getter,
message/FD/name export, send, start, completion or release capability exists.

The context binding retains the actual Origin Arc from the original progress
packet, after original sender and receive-hold checks. It is context-only, not
authority after packet destruction. Typed nsfs user/mount/PID/time identities and
boot must match the channel view; candidates are compared, never adopted. Live
checks surround one unchanged complete frame, still 34 authentication and 34
signature calls. Original predicates, child inventory and cancellation order,
alarm objects, origins and 300/60/10-second windows are unchanged. A failed join
owns and poisons the same original frame; no retry/reconstruction is exposed.

## Distinct evidence scopes

- Ordinary GNU current/MSRV: consuming-constructor compile contract, three
  receiver-binding units, nine previous manager units and nineteen exchange units.
- Nonroot physical-host frame composition: three healthy controls, ten selected
  live refusals and the real consuming fixed-channel access refusal, per compiler.
  Healthy composition uses a **distinct test channel**, never a production join
  factory. Full frame checks, record/origin/bound facts and sticky refusal remain.
- Separate local WSL-root read-only component: actual fixed manager connection and
  actual original receiver context, then closure and permanent refusal. No root
  child, full enrollment, trust writes or service operation.
- Static: 238 source contracts, including three new order/binding/mutation guards;
  exact additive normalization restores old bodies, not weakened predicates.
- Portable: 44 Windows controls; affected all-target checks and strict Clippy,
  format, registry (312 entries; 184 automatically discovered gates, plus explicit
  external-tool registrations), docs and local non-promotable performance contract.

These separate scopes do **not** prove a positive production-enrolled full join,
unique-owner original message/FD delivery, active start, active watchdog,
all-writer cessation, completion, later cells or disconnect/reattach.

## Retained negatives and attribution

1. Baseline old manager/exchange units pass on both compilers. The first new
   constructor compile test fails as expected with E0599 before implementation.
   The initial red-run wrapper incorrectly searched stderr alone; its raw JSON
   diagnostics and corrected expected-red evidence are both retained.
2. `fixtures-194.log`: three controls and seven refusals pass; the next fixture
   stops in the unchanged constructor at `Original(Challenge(Time))`, **before
   entering the new join**. No MSRV fixture cohort or root case completes as part
   of this stopped cohort. `attribution-old-frame.log` is a later, separately
   labelled passing old-frame screen, not an explanation or replacement. Local
   timing causation and complete local robustness remain open.
3. `first-contracts.log`: 236 pass / two fail because older output/Starting guards
   compare new kernel registrations raw. Only their marked restoration adapters
   are extended; old assertions/bodies remain exact. Final static suite is separate.
4. `first-host/`: the old trust reader refuses before frame construction. The new
   harness omitted private umask: independent owned mode probe records 0002 and
   0775. Fix is process-local umask 077, not a relaxed directory predicate.
5. `second-host/`: corrected harness and first images pass all three controls and
   ten refusals on current Rust. The real nonroot Channel refusal works, but the
   new test's enclosing fixture expects false instead of original poisoned=true.
   Its exact source is retained. Change only that new expectation and the Clippy
   format spelling; runtime source and deadlines are identical. Fresh images and
   a fresh host cohort are recorded separately, not a retroactive pass.
6. `first-gnu-188.log`: strict MSRV lint catches the new test's format spelling.
   `first-check-target.log`: evidence wrapper used an incorrect Cargo target name;
   the corrected runner uses the actual integration target. Neither is a runtime
   failure or an altered acceptance threshold.

`first-images.json` binds the earlier images. Their package source equals the
final source except for `second-host/frame-tests.source` (the two test-only edits
above); the verifier checks that exact relationship. No binary is checked in.

## Host constraints

`host-guard.sh` reads stable supervisor PID 8839, active/running, NRestarts=0,
boot, installed executable and provisioning receipt hashes, plus absence of
campaign/fixture markers. Tests run as UID/GID 1000 in a new owned 0700 `/tmp`
root using only uploaded test images and nonexecuted signed bundle data. The
final postflight checks no fixture executable remains. No installation,
supervisor stop/restart/configuration, clock change, product load or qualification.
Temporary artifact locations remain in the three host receipts for later scoped
cleanup; no broad deletion occurs in this experiment.

## Reproduction

`run.ps1 -Stage Baseline`, `-Stage Red` and `-Stage Candidate` document the phases;
Red requires the preimplementation source and is expected to fail compilation.
Cargo JSON selects each exact executable, then hashes bind its execution.
`run-fixtures.ps1` records the **stopped local route**; do not rerun unchanged to
replace that outcome. `run-host.ps1` creates a new guarded nonroot fixture gate
under existing server authorization; it is not an offline verifier. Its exact
boot/PID/receipt guards must not be edited just to obtain a pass. `run-readonly.ps1`
is a separate local privileged read-only component gate. `check-local.ps1` runs
the affected step gates only, not full release qualification.
Its `-Finalize` mode requires the recorded heavy checks and reruns format,
registry/docs/local contract/diff after documentation edits; it cannot replace
the heavy gate or make a failed GNU log complete.

Next: construct the uniquely addressed original message under the same owner and
full frame, prove actual negotiated FD delivery, then authenticated original
start. Active watchdog/all-writer cessation and the new active SIGKILL boundaries
must precede durable completion or fence release. Qualification digest stays
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
