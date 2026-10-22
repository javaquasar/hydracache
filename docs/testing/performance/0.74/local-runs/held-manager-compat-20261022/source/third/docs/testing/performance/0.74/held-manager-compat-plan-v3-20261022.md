# Corrected fixed property interface before any unit submission

Status: preregistered read-only tooling correction. V1 and V2 sources and their
first raw failures remain immutable. Neither invocation reached StartTransientUnit.
V2 proves the pinned unique owner reports PID 1/UID 0, then its Version query
uses the bus name where the property interface must be org.freedesktop.systemd1.Manager.
The original runtime channel is unaffected by this new test-tool typo.

Add an exact-argv regression for that fixed property request and retain its red
baseline before changing the interface. Correct that single argument and run
all eleven offline controls on both platforms, then freeze a fresh V3 method.
No peer, version, name-absence, one-attempt, outcome or closing check is removed.
V1/V2 test budgets and 250 ms command deadlines remain; original runtime
300/60/10 origins/windows and qualification remain untouched.

One fresh V3 attempt may reach the single mutation that neither prior tool
invocation submitted. After its first StartTransientUnit response/error/timeout,
stop this lane with no repeat, fallback, deadline extension or original admission.
All three tooling prefixes remain separate; no failed mutation is retried.
