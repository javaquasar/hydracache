# Strict held activation ingress checks

The runner reuses the unchanged exclusive logger from held-complete-census-074,
with its own fixed source list, commands and output directory. It selects only
parser/core unit tests and the new synthetic ingress process control, not the
ignored launcher fault matrix. No systemd unit, original owner, worker, server,
clock/service mutation or performance workload is invoked.

```powershell
python -B tools/held-activation-ingress-074/check.py host
python -B tools/held-activation-ingress-074/check.py 194
python -B tools/held-activation-ingress-074/check.py 188
python -B tools/held-activation-ingress-074/check.py metadata
```

Existing target/held-activation-ingress-20261022 lanes refuse before execution.
Failures/timeouts are retained by the shared logger, not overwritten or resumed.
retain.py checks exact commands/source/log hashes, scope, component selection,
qualification and committed source before exclusively creating the evidence
packet. Compiler/test elapsed times are operational metadata, not performance.

The positive ingress control intentionally reaches the unchanged core's missing
descriptor refusal. It does not execute a worker or demonstrate original launch,
manager delivery, executable addressing, all-writer cessation or server readiness.
