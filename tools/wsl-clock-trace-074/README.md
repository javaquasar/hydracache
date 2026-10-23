# Bounded local clock tracing

One explicitly authorized WSL-only diagnostic, not an original-owner retry.
Read the frozen [method](../../docs/testing/performance/0.74/wsl-clock-trace-plan-20261023.md).

From the exact Windows worktree, after committing the reviewed sources:

```powershell
python -B tools/wsl-clock-trace-074/run.py collect
python -B tools/wsl-clock-trace-074/run.py retain
python -B tools/wsl-clock-trace-074/run.py verify
```

The capture output lane is exclusive and must never be deleted to retry. A failed
capture can still be retained. Unit tests are not live tracing:

```powershell
wsl.exe -d Ubuntu -u artur --exec python3 -B -m unittest discover -s tools/wsl-clock-trace-074 -v
```

The guardian independently cleans only the owned instance/probes, including on
owner SIGKILL or its hard RAW deadline. Tests of its process-lifetime behavior
use only owned test processes and mocked tracefs cleanup. Snapshot equality is
checked after normal cleanup, never imposed by overwriting global controls.
On timeout, inspect the saved guardian/cleanup receipts and tracefs read-only;
do not rerun the capture. Unconfirmed cleanup requires explicit attention.

Analysis is offline, pairs returns, checks loss, distinguishes filtered query
returns and preserves unknown historical writers. Kernel PIDs are not treated
as authenticated distro process identities. It makes no performance or admission
claim and changes no runtime predicates or deadlines.

The 2026-10-23 lane is already consumed: configuration failed with EINVAL and
cleanup succeeded. See the [first-result report](../../docs/testing/performance/0.74/wsl-clock-trace-20261023.md).
Do not invoke collect again or delete its output. The later path/traceback
diagnostic correction has only offline/mock validation, not a second trace.

The separate configuration-only localization lane is also consumed. It
identified an open-time EINVAL on kprobe_events in append mode; no command was
written and no trace buffer activated. See the
[configuration report](../../docs/testing/performance/0.74/wsl-clock-einval-20261023.md).
Verify its exact archive with `python -B tools/wsl-clock-trace-074/einval.py verify`.
Do not invoke its collect/session modes again or delete either output lane.
