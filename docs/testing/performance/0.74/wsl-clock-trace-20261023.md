# First bounded WSL clock trace: configuration refusal

## Result

The one authorized local trace attempt returned exit 1 during configuration:
`OSError(22, 'Invalid argument')`. It produced no probe-format snapshot, trace
text or clock-query samples. This is a failed diagnostic capture, not proof of
absent clock setters, healthy time, original delivery or server readiness. The
exact failing operation is unknown: the original exception receipt saved only
`repr(error)`, without the originating traceback or control-file path.

No live redispatch or original-owner retry was performed. The source used by
the first attempt is `e420f315`, retained byte-for-byte with the frozen
[method](wsl-clock-trace-plan-20261023.md). Its exclusive output lane remains
consumed. The first original-owner ordering refusal and the earlier read-only
series are unchanged; none has been replaced by this trace failure.

## Bounded effects and cleanup

Preflight checked the pinned kernel/BTF ABI, exact read-only query helper and
empty global event/probe/instance configuration. A detached guardian was armed
before configuration effects. The planned private instance used random group
`hc074clock_80784d1e284e49358f14cf2403b0eaf1`.

Both owner and guardian cleanup receipts report success: the owned instance is
absent and no owned probes remain. The guardian observed normal cleanup
completion, not expiration. This does **not** mean the capture completed.
Global controls compare equal before/after: nop tracer, original local clock,
original trace options, tracing_on=1, events disabled, empty set_event,
kprobe_events, dynamic_events and instance list. An additional read-only
readback confirms the empty registry/instance list and error_log.

Boot `cf222c07-5476-4f70-9a32-455e99204d0a`, PID1 start ticks and time namespace
are unchanged. No clock-setting request, service change, original-owner
execution, product workload or server action was made by this diagnostic.
There is no claim that configuration reached every planned probe; successful
cleanup alone does not supply the missing configuration-stage history.

## Evidence and checks

[Exact archive](local-runs/wsl-clock-trace-20261023/index.json): 25 logical files,
index SHA256 `89912e476413847377bed1f22d1e52c799d2ded988fd3f861ecfedc56960da03`.
It retains the explicit scope/intent, source bytes, command exits, 16 successful
pre-dispatch controls, first refusal, before/after context and both cleanup
receipts. Its dedicated diagnostic schema declares tracing-state effects;
it does not mislabel the attempt as read-only instrumentation.

```powershell
python -B tools/wsl-clock-trace-074/run.py verify
wsl.exe -d Ubuntu -u artur --exec python3 -B -m unittest discover -s tools/wsl-clock-trace-074 -v
.\target\debug\xtask.exe doc-check
.\target\debug\xtask.exe performance-contract-check --release 0.74
```

After sealing the first outcome, a separate source-only correction `ed0df8c7`
adds the exact control path to buffered write/close errors and saves the full
traceback. The new negative control raises EINVAL on both write and close and
checks that the path is retained. All 17 current controls pass; they do not
enable probes. The original 16-control run and its source stay in the archive.
Doc-check and the local non-promotable performance contract pass. No Rust
runtime source or tests, Cargo.lock, COMPAT or qualification manifest changed.

## What remains

The existing rate explanation is unchanged. Neither the exact backward setter
nor the historical tick setter was identified by this failed capture. The
collected kernel error_log is empty, so it cannot fill the missing operation
traceback. Do not infer which configuration write failed from EINVAL alone.

The next diagnostic step would require a separately approved, separately named
bounded candidate that records the exact failed operation, retaining this first
result. The current lane cannot be replayed into a pass. No change to runtime
checks, observation windows or clocks is proposed. Healthy original delivery,
all-writer retirement, full crash/lost-ACK matrix and server rehearsal remain
their own unresolved gates; this diagnostic does not admit any of them.
