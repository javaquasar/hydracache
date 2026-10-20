# Held artifact build authority and executable topology

Baseline is 148bb3fe25cb94d329e6a35468ecd6838a88fed7. The private fixed-slice
kernel gate is complete; it does not supply trusted held artifacts. Before
implementing their original-owner join, approve the executable topology and
purpose of the new build authority below. No option is implemented or enrolled
by this audit. Inherits docs/RULES.md and the approved
[attempt-root sequence](cgroup-root-enrollment-plan-20261019.md).

## Existing producers and their scope

| Object | Current implementation | Authority gap |
| --- | --- | --- |
| Frozen worker build | BuildStatement v1, exact timing-controls-074 recipe and source 62114be0f5da3218706e30d7424acfb5d0579d07 | It cannot attest a changed target or a supervisor/helper build |
| Builder policy | Externally pinned builder/controller keys and fixed hydracache-linux-observer-074-v1 purpose | Reusing its key or policy does not silently extend that purpose |
| LaunchArtifacts | Three signed expected SHA256 values in v2 launch claims | No builder receipt, open executable object or actual process-role proof |
| Root watchdog | Private clone3 continuation in the supervisor image | Not a separately built watchdog executable; original role/image join remains |
| Barred collector | Private clone3 continuation retaining original I/O custody | Still quarantined; not an active independent collector executable |
| Held launcher | Private packet/barrier mechanics | No production launcher target or pinned-worker exec path yet |

The code anchors are the [worker build verifier](../../../../tools/long-run-supervisor-074/src/diagnostic_artifacts.rs),
[builder policy and signer](../../../../tools/long-run-supervisor-074/src/diagnostic_builder.rs),
[launch claims](../../../../tools/long-run-supervisor-074/src/diagnostic_two_phase_launch.rs),
[root guardian creation](../../../../tools/long-run-supervisor-074/src/diagnostic_root_guardian.rs)
and [barred collector continuation](../../../../tools/long-run-supervisor-074/src/diagnostic_original_barred_collector_kernel.rs).
The fixed nine-file install reader contains the worker, its receipt/log/locks
and four workload configs, not a held-helper bundle.

There are five additional binaries under src/bin: builder, controller and three
signer probes. None is the required held launcher. The supervisor's default
executable is separate from those five; counting Cargo binaries is not evidence
that any supports the required held-start semantics.

The diagnostic_controller binary is explicitly a protected signing-job producer,
not the original execution host. The new fixed-placement join is not called by
that CLI or main.rs. The future execution driver must name and attest the actual
hosting executable; linking the library into a signing tool is not role proof.

## Why this is a design decision

Three logical artifact roles do not require three different executable files.
The current v2 grammar does not require distinct hashes. Watchdog and collector
are created without exec and therefore retain the supervisor executable identity.
Giving their names to new binaries would change their startup, FD inheritance,
post-clone safety and failure boundaries, rather than merely verifying hashes.
Conversely, equal hashes alone do not prove either role or original custody.
Those still come from original owned constructors and actual kernel observations.

The [reusable library boundary](test-run-control-library-boundaries.md) explicitly
defers crate extraction until P2/P3 and local crash gates are complete. Do not
create a generic runtime crate as an incidental artifact-verification change.
The [held placement plan](held-placement-plan-20261019.md) also forbids inventing
a launcher install path or replacing the old ExecStart. Both restrictions remain.

The old BuildStatement rejects even re-signed target/source/toolchain drift.
Weakening that verifier, relabeling a worker receipt or selecting a caller-supplied
key would remove a trust boundary. A separately reviewed purpose-bound policy,
build recipe and signed binding to the original attempt/epoch are needed. The
existing controller endorsement of artifact hashes is not that build provenance.

## Recommended minimal contract for approval

Keep root guardians and the barred collector inside the existing supervisor
image, preserving their exact clone/retirement machinery. Add only a dedicated
held launcher. In the future role manifest, watchdog and collector may name the
same attested supervisor image, but their role-specific original ownership,
pidfd/FD/cgroup checks and full validation frames must remain independent.
Keep the existing barred I/O quarantine unchanged. An active collector still
needs a separately admitted activation/handoff and fault gate; sharing its image
does not make the quarantine loop an active drain/finalization implementation.

Introduce a separate purpose-bound build policy and bundle receipt for the held
launcher and supervisor image, with explicit role mapping. Prefer a dedicated
held-runtime builder key, distinct from the existing worker builder and controller
keys. Approval of implementation does not itself enroll that key or its digest.
Do not modify the frozen worker source, recipe, locks, policy or receipt.

The new policy must pin exact reviewed source/tree/lock/toolchain/target/feature
inputs. Its receipt must bind role mapping and complete build observations to
actual file bytes. A new versioned signed launch binding must connect that
policy/receipt digest to the same admitted original attempt and epoch; it must
not upgrade v1 consent or give the attempt another replay slot. Register actual
new wire/durable formats in COMPAT while preserving every old signed byte.

Before release, bind the actual held-launcher process and its retained worker
executable FD, streams and cgroup. After the one-shot pinned-FD exec, observe the
same process generation and actual worker executable before publishing Running
or completion. These are distinct observations: proc/exe cannot be both launcher
and worker before exec. No pathname fallback, arbitrary command, resend or renewed
lease is permitted. This distinction must be explicit in the new signed contract.

Keep all old live predicates, 46 effect brackets and original 300/60/10-second
origins/bounds. Any new complete frame adds checks; it does not substitute a
snapshot or supplied assertion for the existing original frame. Armed still
precedes delivery, and actual EOF/finalization still precedes fence release.

The alternative is separate watchdog and collector executables with a new
authenticated exec/enrollment topology. That requires its own owned FD/process
handoff and full guardian/collector fault regression, so it is not the minimal
next step for this release. It is not selected here.

## Work after approval

1. Freeze the approved logical-role mapping, builder purpose and pre/post-exec
   semantics as a local implementation plan. Choose no production install path.
2. Build the local held-launcher target and role-specific receipt/object verifier;
   retain Windows/current GNU/MSRV tests and baseline/red/fault evidence.
3. Join real role objects and receipts to original policy/epoch, fixed root,
   manager channel and full frames, then build the separate held-unit projection.
4. Complete joined SIGKILL/lost-ACK, all-writer cessation, durable lifecycle and
   same-attempt reconnect before separately authorized server rehearsal.

No production key generation, new policy enrollment, install, system-bus/original
send, supervisor operation, server access or product workload is authorized by
this proposal. The old direct-worker message remains unsent. Existing regressions
can check the old boundaries but cannot accept the proposed held artifacts.

## Local audit verification

The [separate evidence packet](local-runs/held-artifact-boundary-20261020/README.md)
retains seventeen ordinary Windows artifact tests and all thirty-one artifact
tests on each GNU 1.94/MSRV 1.88 compiler. Each retained release image also passes
eleven existing two-phase unit controls with its original source reference and
unchanged before/after hash. Windows strict lint, scoped fmt, docs and the local
non-promotable contract pass. The previous namespace packet verifies unchanged.

All 437 supervisor sources, tests, Cargo.lock, xtask, COMPAT, registry and the
qualification digest remain unchanged. These are old regression checks and a
recorded architecture boundary, not a new held-artifact implementation or live
fault cohort. Stop for topology/build-authority approval rather than introducing
another hash wrapper which labels that unresolved authority as complete.

## Approved local continuation

The operator subsequently approved the proposed minimal contract by continuation.
The [implementation plan](held-build-authority-plan-20261020.md) records its exact
scope: one future held launcher, existing keepers in the original host image,
separate build purpose, and distinct pre/post-exec identity. This is local design
approval, not production enrollment or installation. The sealed audit packet
above remains historical and unchanged, including its false approval field.

The first implementation at 48854c1e adds the separate static build-policy,
constrained signer and receipt/content verifier. It does not yet implement the
launcher target, retain original role files or join them to the live frame.
The [new packet](local-runs/held-build-authority-20261020/README.md) separates
synthetic format tests from actual role builds and later execution gates.
