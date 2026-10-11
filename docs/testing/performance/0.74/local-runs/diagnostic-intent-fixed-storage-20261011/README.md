# Fixed intent storage local evidence

This packet verifies read-only original-directory inspection, not a fixed intent
publisher, host fence, authority join or enrollment. Baseline is
`794d164d26f9c0cbe8bd050eb68588f5b3caffae`.
[manifest.json](manifest.json) binds the implementation source/tree and closed
raw captures; [verify-packet.py](verify-packet.py) checks them read-only with
Python 3.11 or newer. Details: [design](../../diagnostic-intent-fixed-storage-local-design.md).

Seven new unit cases pass three serial repetitions. Selected Linux tests pass
479 with one existing ignored. Root checks pass 108 contract, 13 evidence, six
gated-target and 23 governance cases. Linux/Windows strict lint, all-target checks,
MSRV 1.88 and local documentation/contracts are checked separately. Counts are
local harness rows, not unique release coverage or an exact-candidate ship receipt.

`01` retains the unchanged 19-test journal baseline; `02` is absent-API red.
`03` is the expected formatting refusal before rustfmt, not a behavioral failure.
`04` is the first green seven-test run; `06`/`07`/`08` are repeats. Tests use
owned nonroot temporary trees with a private ownership seam and modeled root
metadata; no root production directory is created or modified. They cover all
five original/named directory levels, unsafe/missing/symlink components, owner
policy, sticky restored-state refusal, benign growth and non-Send/Sync API shape.

Existing tmpfiles ownership is retained: worker-owned campaigns cannot serve as
trusted root intent authority. The old authority backend's root:root parent rule
also differs from root:hydracache-perf provisioning; neither is altered here.
The guard accepts the nonwritable performance-parent group and retains it exactly,
but requires root:root 0700 at the prospective separate fixed leaf. Sequential
inspection is not continuous confinement, initial namespace or live host proof.
There is no public path/owner/FD selector, fixture conversion, write or start route.

Qualification dry-run and ship admission remain expected-red; all eight frozen
inputs and the manifest are unchanged. The inherited plan-digest mismatch remains
unreconciled. Earlier evidence is verified at its own Git source, not relabeled
as proof of unchanged current code. No server, workflow, product workload,
0.73 frozen artifact, numerical claim or qualification operation is changed.

The next prerequisite is a preregistered original-authority/fence/state composition
and fixed publisher with bounded lease retention. Provisioning a new storage leaf,
real issuer/host enrollment, runtime/material/output join and Starting/watchdog/
owned cleanup remain later boundaries. A successful inspector is never permission
to consume an intent or start a worker.
