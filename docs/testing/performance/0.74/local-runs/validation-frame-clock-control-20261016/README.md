# Approved WSL clock control: preflight refused

The separately preregistered control in commit 32b5f2ba did **not** reach any
runtime test. The operator authorized stopping/restoring only WSL Ubuntu's
systemd-timesyncd. The exact unchanged release executables passed their observed
SHA256 checks. A foreground root wrapper observed the service active, stopped
it and observed it inactive. It did not change clocks, configuration or Windows.

The ninety-second probe refused at index seven: CLOCK_REALTIME decreased by
3,151,529,293 ns while CLOCK_BOOTTIME increased by 1,000,220,159 ns. The fixed
preflight guard refused; its threshold was not changed. The exit trap restored
timesyncd and verified it active. Independent follow-up observes active/enabled,
NRestarts zero and the same boot id. The wrapper reports result 93; the outer
WSL tool reports exit 1. Both are retained without normalizing them into success.

No matrix or budget command began. Rust 1.94/1.88 live-fault coverage is therefore
still incomplete. The earlier twelve budget chains and five completed fault
cases remain in the [original packet](../validation-frame-20261016/README.md).
Stopping timesyncd alone is insufficient in this observed environment; this
does not uniquely identify the source of the clock steps. No identical retry
was made, and neither host clocks nor another clocksource were selected.

The run script, capture and independent restoration are retained here with a
read-only verifier. The runtime candidate remains the same uncommitted sources
and archived patch, not a newly admitted implementation or server build. No new
Starting/SIGKILL/workload/qualification/server operation occurred. Further host
clock or environment changes require separate operator agreement; otherwise
finish the gate in a separately approved stable Linux environment.

The verifier passes from outside the repository; its deliberately promoted
preflight manifest outside Git refuses. Both observations are retained in
`verification-results.json`. Verification integrity is not a passed preflight.
