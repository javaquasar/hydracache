# Real WSL registry configuration gate

## Confirmed result

The one freshly authorized configuration-only session completed successfully,
exit 0, using the nonseeking command writer. All seven planned probes were
registered and their displayed definitions matched the intent exactly. The
private adjustment filter and event enables were configured, all seven event
format reads succeeded, and all 28 journaled setup operations completed without
a failed operation. No private trace buffer was activated or clock query run.

This closes the real registration/deletion check for this pinned kernel and
candidate. It does not prove event delivery, field values at an actual setter,
stack parsing, loss accounting or clock-setter attribution. Those require a
separately scoped observation rather than a configuration receipt.

## Exact path and retirement

The source was `a7751388`, using the existing writer correction `e95bf69c`.
The entry point selects a fresh exclusive lane but calls the unchanged
`einval.session`: same preflight, setup order, detached guardian and deadlines.
No changes were made to `capture.py` or `einval.py` in this gate.

The private nonce was `hc074clock_e06c6293733e4dcba9479b8d126b387c`.
The retained pre-cleanup registry shows all seven exact definitions; both the
root and private error logs are empty. Owner cleanup removed the exact owned
definitions and instance. The guardian independently confirmed no remnants and
reported normal completion. The global probe/dynamic-event registries and
instance list are empty again.

Root controls compare equal to preflight, including nop tracer, original local
trace clock/options, tracing_on=1, disabled root events and empty set_event.
Boot, PID1 start ticks and time namespace stayed fixed. Cleanup was verified by
comparison, not by overwriting root controls. The original 120 RAW-second
guardian and 180-second outer timeout were retained; product 300/60/10 windows
and all runtime checks remain unchanged.

## Evidence and checks

The [exact archive](local-runs/wsl-clock-registration-20261023/index.json)
contains 43 logical files and 37 objects. Index SHA256:
`39a658169264764197f24cf849309373d7002c2c9d9520e6eedc3b8ef767c501`.
It retains the authorization/intent, source bytes, 33 passing pre-dispatch
controls, complete operation journal, exact registered definitions, configuration
receipt, command exit, both cleanup receipts, context comparisons and local
gate logs. The old failed trace, EINVAL localization and writer-control archives
remain pinned and unchanged; none was retried or replaced.

```powershell
python -B tools/wsl-clock-trace-074/registration.py verify
python -B tools/wsl-clock-trace-074/run.py verify
python -B tools/wsl-clock-trace-074/einval.py verify
.\target\debug\xtask.exe doc-check
.\target\debug\xtask.exe performance-contract-check --release 0.74
```

Doc-check, the local non-promotable performance contract, frozen source/input
guards and both earlier live-archive verifiers pass. No Rust runtime source or
test, Cargo.lock, COMPAT or qualification manifest changed. No clocks, services
or packages were changed, and no original-owner, product workload or server
execution occurred.

## Next boundary

The consumed lane cannot be rerun. A new bounded clock observation would need
separate authorization: private-buffer activation, at most 90 RAW seconds of
events and the existing read-only helper, with the original 120 RAW-second
guardian and loss/cleanup checks. Preserve the first result even if no setter
occurs or the decoder refuses. Historical tick ownership and the exact setter
of an older failure cannot be inferred from a future event alone. Original
delivery, all-writer retirement and server admission remain separate gates.
