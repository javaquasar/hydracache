# Original pending custody after owner and notice loss

This packet tests the [pending-only loss contract](../../diagnostic-original-custody-loss-plan-20261018.md).
The ordinary-build pending implementation remains exact: the only new runtime
source lines are a cfg(test) seam that consumes the real first P notice. No
collector, output write, authenticated send, original start or completion is added.

## Matrix and observation ownership

Each GNU Rust 1.94 and MSRV 1.88 cohort first runs a healthy screen, including
34 authentication and signature calls at construction and later revalidation.
Three lost-notice cases then consume the actual P and require Expired at the
same selected alarm, unchanged four immutable origins and sticky retry refusal.
After refusal, the first remaining ordered notice must be exactly E while the
custodian is live. An unconsumed P would fail this observation, distinguishing
actual notice loss from expiry before the first read. The closing validation
callback must not be reached. The post-refusal observation's harness fail-safe
neither renews an authority window nor permits any write or release.

The separate Python driver executes only the opt-in libtest fixture. Its private
Unix datagram checks kernel sender credentials and transfers exactly the atomic
custodian pidfd, never the original fence or notice peer. The driver pins its
own unreaped fixture owner before waiting. Numerical-PID signalling is absent;
the production custodian retains its unchanged atomic clone3 pinning contract.

Three repetitions at each of two cuts kill that owner before its closing frame
or after its 34 authentication/signature checks return. The driver temporarily
acts as a child subreaper, checks the custodian's three inherited descriptors,
and independently opens the original flock. It owns no original descriptor
alias that could hide owner loss. After the owner dies, custody must survive
past the same original deadline. Only the subsequent custodian SIGKILL and exact
pidfd wait may make the kernel flock available; a fresh storage reader must
still refuse retained history. Origins, deadline, cut and owned process IDs are
logged per checkpoint. Origin records and authority metadata/bytes stay exact.

This establishes retention after pending-owner death, not an E-notice after
peer loss: the unchanged child may latch peer loss first and park with custody.
No active writer exists here, so the result cannot establish whole-cgroup writer
cessation, authenticated manager acknowledgement or safe production completion.

## Completed controls

The final source is a40860a7165dd8a9cc14d16e03cdf3f22b9d245e, following the
initial fault implementation 879ccd983cd04fb262e77a7e9ed238e333169849 and
preregistration cd0f1a7a. Both final compiler cohorts complete successfully.

| Selected result | GNU 1.94 | MSRV 1.88 |
| --- | --- | --- |
| Full-frame healthy screen | 1 | 1 |
| Lost P with first remaining E | 3 | 3 |
| Owner death before closing frame | 3 | 3 |
| Owner death after closing frame | 3 | 3 |
| Selected SIGKILL signals, owner plus custodian | 12 | 12 |
| Local Rust fast units | 32+5 | 32+5 |

Thus twelve selected owner scenarios use twenty-four selected SIGKILL signals.
Healthy cancellation and notice-refusal cleanup are separate, not extra selected
owner cases. All 216 static contracts, six Python unit cases, two compile-fail
doctests and affected checks/strict Clippy pass. Frozen qualification SHA256 stays
11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.
Postflight finds no fixture from any of the four retained scratch cohorts,
keeps supervisor PID 8839/NRestarts 0 and the installed executable/receipt hashes
exact. No original worker or product workload runs.

## Preserved negative fixtures

The first fixture selected the healthy postcondition although the fault correctly
poisoned the owner. It reached Expired and verified retained history, then failed
the outer assertion. Its complete ten-file source capsule, pre-change input
inventory, binary hashes and host/postflight logs are in negative-first-oracle.
The single archived fixture flag was restored after the working edit and its
entire raw file checked against the pre-change input digest; source-manifest.json
records that reconstruction explicitly. No owner-death cases or MSRV cohort ran.

The second fixture put authority inside its temporary tree. The unchanged strict
direct-/tmp authority rule refused initialization before pending custody existed;
the driver later hit its harness receive fail-safe. Three lost-notice refusals
had passed, but zero owner cuts completed and MSRV had not started. Its complete
source/input/log capsule is in negative-authority-origin. The replacement uses
a separate owned 0700 /tmp authority fixture, not a path-validation exception.
Both rejected cohorts have clean, read-only host postflight checks.

The matched source 879ccd98 untraced matrix also passed, but its supplemental
strace attempt refused upstream Time before pending custody. Its script, empty
of-P syscall trace, matched untraced control, source/input identity and postflight
are retained in negative-tracer. Zero traced cases completed; MSRV tracing did
not start. No timing window was extended to accommodate the observer. The final
test instead checks the ordered E notice after sticky refusal, outside the live
validation path; neither that proof nor the main matrix depends on tracing.

## Reproduction and independent verification

The source commit, complete ten-file patch, 316 raw libtest input digests with canonical
Git-source bindings, exact binary/script identities and all packet file hashes
are in manifest.json. Build and strict-Clippy logs cover Linux current/MSRV and
Windows; fast-final.log covers 32+5 Rust units per compiler and six Python units.
Two private-descriptor compile-fail doctests run on GNU 1.94. Full workspace
formatting, registry, documentation and local non-promotable performance-contract
checks are retained. This is not cargo xtask verify or release qualification.

For an already authorized nonroot fixture host, build the libtest images with
cargo test --release -p hydracache-long-run-supervisor-074 --lib --locked --no-run.
Review guard.sh, then use run.sh only in a fresh owned scratch directory with
the matching images and scripts and the signed bundle supplied as nonexecuted
fixture data. The owner-death driver command is python3
original_custody_owner_loss.py --binary ./tests-194 --repeats 3 (and tests-188).
Do not point it at an installed product executable.

Run ./verify.ps1 -SelfTest to check artifact/source/input identity, phase order,
compiler pairing, cut/repetition counts, original checkpoint shapes and nine
deliberately invalid result controls. The manifest is unsigned inventory, never
an authority receipt. capture.ps1 regenerates metadata from existing files and
a committed source identity; it cannot manufacture test outcomes. No installation,
supervisor mutation, clock mutation or product/qualification workload is performed.
