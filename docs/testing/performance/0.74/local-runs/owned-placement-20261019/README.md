# Same owner placement constraints local evidence

Baseline `f4d7f44c2dc39ef6aa19cfc26611bde32e97bdcc`; selected implementation
`67c2f41c1b9288297c95507986859883f5645949`. See the
[plan](../../owned-placement-plan-20261019.md) and
[ownership design](../../owned-placement-design.md).

The private new result consumes the same original manager/root holder. It
selects the retained controller-policy key, compares every original consent
field and exact Armed enrollment, and repeats the original full live round.
There is no caller-selected issuer, supplied root/clock, descriptor getter,
retry owner, held-unit projection, send, completion or fence release.

## Baseline and construction sequence

baseline194.log and baseline188.log use retained ordinary GNU 1.94/MSRV 1.88
images from the preceding implementation; their hashes are in baseline-images.log.
The baseline revision changes only docs/evidence after that implementation.
baseline-live runs one old privileged writer-screen per compiler with actual
fixed manager, original frame, kernel guardians, three dummy writers, drained
EOF and a busy fence. It is not a new performance or creator-SIGKILL experiment.

red-missing-module.log records the new tests failing before implementation.
first-check.log and fixture-check.log retain the preliminary successful compile
checks. green194.log is the first eight-model-test result from the work in
progress, not the selected-source compiler receipt. clippy-initial188.log
rejects a large error variant. The final implementation boxes that error,
matching the old holder pattern; no lint is suppressed or bound changed.

gnu194/gnu188 retain the initial 08253cca build cohorts. gnu194-final/gnu188-final
are the selected-source cohorts after the test-only custody oracle correction.
build.ps1 checks clean
runtime source and fixed HEAD, pins compiler/image/recipe and runs strict
all-target/all-feature lint, eight new ordinary controls, old launch/enrollment
and original-owner/kernel/root selections, and the complete Linux doc suite.
The private result owns the full prior holder; no raw pointer/FD work is added.

Copy build.ps1 from this evidence revision into the same packet-relative path
of a separate checkout at the selected source, without copying output labels:

```powershell
./build.ps1 -Compiler 1.94.0 -Label gnu194-final
./build.ps1 -Compiler 1.88.0 -Label gnu188-final
```

Never overwrite this retained packet or use two builds for one compiler at once.
The recipe expects 247 Linux doc tests; Windows has twenty. The source-only
suite uses four threads, with all previous bodies and process-global mutation
restrictions unchanged. Its three new exact guards reconstruct all 425 old
sources, COMPAT, the registry and external creator harness. Only three module
hooks and one explicit manual gate are removed. The generic gate scanner's
184 discovered gates do not enumerate out-of-line library unit tests; the new
manual fixture is explicitly registered and requires an actual one-test receipt.

Models exercise all 36 consent leaves, root/context/deadline substitutions,
every opening/closing/authentication failure, unarmed/uncertain/refused controls,
sticky refusal and 128 re-signed constraint swaps with seed 0x074e6a1d. Their
healthy trace contains exactly the one old full round, two new signature checks
and four kernel observations; no enrollment or descriptor extraction is called.

The preliminary model log labels its 128 iterations identities. These controls
actually reuse the same diagnostic identity and vary the re-signed artifact
constraint; this packet reports 128 constraint swaps, not 128 distinct identities.

## Local live scope

After ordinary builds and the source suite stop, live.ps1 runs serial local WSL
fixtures using the fixed manager and actual guardian/root objects, with test
issuers. It temporarily stops only the previously approved local
systemd-timesyncd, requires 65 stable seconds and restores/verifies it in finally.
No clock is set manually. Source/image identity, native success, exact case
count, empty fixture-root scan and restored active clock are separate checks.

The seven named cases are healthy, revalidate, unarmed, wrong-key,
different-consent, foreign-root and trust-drift. Only the first two may accept.
Each uses a fresh original attempt with its unchanged exclusive ten-second
window; a time refusal cannot substitute for an intended negative. The fixture
copies only already existing time observations, not a new admission clock.
It checks EOF and original offsets after destruction. While either success or
failure still owns the consumed holder, the common fence must be busy; intended
refusals also require the retained original timer to remain live. Six Armed
cases must keep the fence busy after Drop. The never-Armed case may release
the raw flock only after EOF and dead custodians; a fresh typed admission must
still refuse its retained incomplete context. It creates
no held systemd unit and starts no original executable or product workload.

```powershell
./live.ps1 -Stage candidate -Source 67c2f41c1b9288297c95507986859883f5645949 -Label candidate-live-final
```

The temporary root's actual device/inode and custody are checked. Its name is
not required to be the expected systemd slice: actual slice ancestry and approved
launcher/watchdog/collector objects remain separate gates before projection.
This result neither enrolls production trust nor authenticates a new incoming
peer. It does not extend the historical creator SIGKILL cohort or prove new
all-writer cessation, durable completion, reconnect or server admission.

The initial candidate-live GNU cohort passes healthy and revalidation, then
fails its incorrect post-Drop Busy oracle for the never-Armed pair. MSRV is not
executed in that cohort and no complete receipt exists. Its raw logs, exact
source/images and restored clock are retained in initial-live-outcome.json.
The preregistered correction changes only fixture assertions and adds test-only
non-consuming observation of the same original timer. No runtime validation,
keeper, bound or threshold changes. Fresh final cohorts must stand on their own.

## Selected source results and open live gate

Selected source 67c2f41c passes all 312 source contracts, strict Windows/GNU/MSRV
lint, formatting, registry, documentation and local non-promotable performance
contracts. Windows passes 60 units and twenty doc tests. Each Linux compiler
passes 61 selected ordinary units and all 247 Linux doc tests; this is not the
complete 1,301-entry library test suite.

The fresh candidate-live-final GNU cohort completes healthy, revalidate and
unarmed, including the corrected retained-custody and typed replay checks.
The next fresh wrong-key attempt fails the unchanged original time guard in
prepare_owned_root_guardians, at live fixture line 107, before signed placement
binding. Its raw Causal(Time) error does not establish the wrong-key negative.
MSRV and the four remaining intended negatives are unexecuted in this cohort.
No successful final live receipt exists; local_live_gate_passed stays false.

The last observed original ages in the completed positive calls are
9,096,508,779 ns and 8,235,511,560 ns. They are not the missing failing tuple,
preparation cost samples or a speedup comparison. The time capture currently
starts at binding, so it cannot recover issued/now/boot for the earlier failure.
The clock preparation records one backward step before its required 65 stable
seconds. This does not identify the later time-guard cause. The clock is restored
active, no temporary cgroup roots remain, and all nine signed fixture inputs
retain their original hashes. Final-live-outcome.json keeps failure and scope
distinct from the first oracle failure in initial-live-outcome.json.

Before a fresh full cohort, extend test-only copies of existing causal and raw
age observations to encompass preparation, with unwind-safe output. Do not add
a clock, refresh a challenge, alter a live round or change a bound. Attribute
the actual failure before proposing an isolated cost change; do not blindly
rerun until green. Actual slice/artifact admission and held projection remain
later work, not a way to bypass this incomplete local gate.

## Verification

verify.ps1 checks exact file hashes, source/image/recipe identities, selected
test receipts, live case outcomes, original offsets, clock restoration and the
unchanged previous packet/qualification digests. It refuses resealing and tests
metadata/inventory mutations. Packet attributes preserve raw Git bytes.

```powershell
./verify.ps1 -SelfTest
```

There is no throughput, allocation or preparation-wall improvement claim.
The next boundary is actual fixed slice placement, artifact admission and a
complete held-unit projection, not sending the old direct-worker message.
The immediate prerequisite is the preparation attribution and fresh complete
live cohort above. A successful evidence verifier confirms the retained partial
result and rejects false full-gate promotion; it does not turn the live gate green.
