# Native runner validation without measurement

This separate entry validates the existing balanced runner in local nonroot WSL.
It never calls execute, run_cohort, method freeze or a compiled libtest image.
The measurement runner and its checks/deadlines remain unchanged.

`python -B test_validate.py` is offline. After committing a clean source,
`python3 -B tools/authority-runtime-native-074/validate.py --attempt <new>` performs
one bounded validation, refusing overwrite and retaining its first failure.

First, the real subprocess adapter runs a synthetic Python two-process fixture
in a new owned group. A five-second synthetic timeout must retain partial output,
kill that group and expose both original pidfds as exited. A separate ten-second
exit-observation bound is not a changed production/measurement deadline. Only
owned fixture processes are affected. This is not all-writer/cgroup proof.

Dependency seed membership and bytes must match the sealed original CPU receipt,
before and after builds. The original byte/mode/no-link copy checks and supervisor
artifact exclusion remain in use. Missing or modified seeds refuse; no fallback.
Exact source overlays/includes and eight allowed role differences stay pinned.
Four fresh release libtests are built with offline/locked Rust 1.94/1.88, strict
lib/tests clippy and check, distinct compiler/role targets and fresh:false guards.
No image is executed. Clock/semantic controls are deferred, not waived; they
remain mandatory before any later separately authorized measurement.

Native validation has a separate 1,200-second aggregate budget, reducing command
timeouts to remaining validation time. Measurement 300/3600, freeze 600 and
production 300/60/10 windows are not edited. Source, boot, namespaces, UID/GID,
configuration and qualification postflight must match. There is no clock/service
change, remote host, installation, supervisor or product-workload action.
