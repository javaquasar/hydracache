# Balanced local authority timing runner

Implements the [corrected v2 method](../../docs/testing/performance/0.74/authority-runtime-variance-method-v2-20261022.md)
and inherited first-draft conditions. Implementation and offline runner checks
do not authorize or substitute for a measured cohort. The runtime, historical
probe/readers and all previous evidence remain unchanged.

Safe local commands, with no image/compiler/WSL invocation:

```text
python -B tools/authority-runtime-balanced-074/runner.py --plan
python -B tools/authority-runtime-balanced-074/test_runner.py
```

The schedule is exactly 64 serial processes, 16 observations per role/compiler,
eight block ratios per compiler, 768 rows and 24,576 timed warm inspections.
SHA-256 is 1566ab8cbfa4cbdea2897737be07ddf533142ede5a6c6dbbc4f5bf36f6f638c1.
Blocks and compiler order are fixed; there is no adaptive iteration, warmup,
selection, resume, skip-validation or retry flag. Test backends are labelled
synthetic. Their cost values are fabricated, not new measurements or claims.

## Future explicit execution

Only after a separate authorization, `runner.py --execute --attempt <new>` in
the exact local nonroot Ubuntu WSL workspace builds four distinct fresh release
images, strictly lints/checks them and runs the old clock/semantic controls.
It archives exact source overlays/includes and method inputs, then publishes
ready.json. Within the unchanged 600-second freeze wait, exclusively create
method.json with source set to the full committed clean method SHA and
schedule_sha256 set to the fixed digest above. Inputs must match that commit.

The entire cohort has a fixed 3,600-second budget; each owned child gets at most
300 seconds, reduced to the remaining cohort budget. Metadata/parse/postflight
costs are charged to the cohort, and a returned over-budget step refuses. These
are experiment bounds, not changes to production 300/60/10 admission windows.
The runner checks boot, namespaces, kernel, source/clean method, UID/GID,
configuration, qualification and image hashes before and after each child.
Parent-side read-only context spans include missing optional fields explicitly;
they never filter data. Process PID/start/spawn/exit boundaries remain separate
from unchanged inner clock brackets. No frequency, affinity, IRQ, clock or
service tuning occurs; taskset pins only owned children.

Every attempted step and completed prefix is retained. On timeout, interruption,
semantic/source refusal or budget exhaustion the runner stops; existing attempt
directories refuse restart. Timeout kills only the owned process group created
for that child. No supervisor, manager, product workload or server path exists.
Hard external termination may leave an incomplete final journal; it is not
resumable or summarizable as a complete cohort.

## Offline review

`evaluate.py <attempt-folder> --output <new-file>` requires a complete native
attempt, 22 successful build/control receipts, all 64 raw profile receipts,
fixed membership/clock/context/provenance and a valid bounded timeline. Synthetic,
failed, incomplete, reordered or modified attempts refuse numeric admission.
It retains all phase/counter observations and eight geometric block ratios per
compiler. Zero counters retain values/differences without fabricated log ratios.
For strict boundary decisions, compare the exact squared geometric ratio of
the two observations per role with (95/100)^2 or (105/100)^2. This is algebraically
the same fixed criterion and prevents floating-point rounding from promoting a
boundary tie; displayed ratios still use the specified exp/mean/log estimator.
Primary intervals and the five-percent descriptive boundary are unchanged; no
pooling, release admission or production end-to-end claim follows.

The [limited native result](../../docs/testing/performance/0.74/authority-runtime-native-validation-20261022.md)
now verifies four Linux builds/checks/lints, unchanged historical dependency seeds
and timeout cleanup of an owned synthetic group. It also fixes cross-platform
inventory ordering without changing sealed bytes or any receipt/workload check.
Compiled-image clock/semantic controls and measured process timing remain later
separately authorized steps. Native group cleanup is not all-writer/cgroup proof.
