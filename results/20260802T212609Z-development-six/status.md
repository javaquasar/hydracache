# Six-experiment run status: initial diagnostic

This is a preserved diagnostic attempt, not valid qualification or bootstrap
evidence. Bundle SHA-256: `cfd32c24b42daa1ad936178797c5568577e22811fd4328989286a651bef95594`.

The workload artifacts are retained for audit. The attempt exposed harness
issues later corrected in the canonical run: host CPU metadata had an awk
quoting error and the reference-evidence tmpfs was not prepared before the
suite. The profile was also subject to the host `perf_event_paranoid=4`
restriction. Do not compare or rank results from this attempt.

