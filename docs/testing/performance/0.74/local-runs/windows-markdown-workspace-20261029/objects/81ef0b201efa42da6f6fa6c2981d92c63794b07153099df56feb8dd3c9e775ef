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

The nonseeking registry writer is now implemented with mock/owned-regular-file
controls only. See the [writer report](../../docs/testing/performance/0.74/wsl-clock-writer-20261023.md).
Both consumed lanes remain consumed; neither is an entry point for validating
the corrected writer. A fresh live configuration requires separate scope.

The separately authorized fresh configuration lane has now passed and is also
consumed. Seven exact probes were registered and removed, with no buffer
activation or clock queries. See the
[real configuration report](../../docs/testing/performance/0.74/wsl-clock-registration-20261023.md).
Verify it with `python -B tools/wsl-clock-trace-074/registration.py verify`.
Do not rerun its collect/session modes. A clock capture remains a separate gate.

The separately authorized bounded setter capture has now completed; its lane
is consumed. See the [setter report](../../docs/testing/performance/0.74/wsl-clock-setter-20261023.md).
`python -B tools/wsl-clock-trace-074/observe.py verify` checks the raw archive.
`attribution.py` is offline only and its output lane is also exclusive; reproduce
arithmetic without writing via `attribution.summarize` on the pinned input, or run
`python -B -m unittest discover -s tools/wsl-clock-trace-074 -p test_attribution.py -v`.
Do not invoke any consumed collect/session entry point again.

`identity.py` only analyzes and seals already collected read-only namespace
metadata. It has no WSL dispatch or capture mode. Its current-process join
requires matching start ticks, namespace links, executable digest, credentials,
cgroup and the nested NSpid chain; it never authenticates a historical trace PID.
Use `python -B tools/wsl-clock-trace-074/identity.py verify` for the separate
writer-identity archive. Retention is exclusive, not a metadata recollection.
The eight `test_identity.py` controls use synthetic metadata and temporary files.

The [root-namespace handoff](../../docs/testing/performance/0.74/wsl-clock-root-namespace-plan-20261023.md)
is prepared, not live-validated. `root_namespace.py collect` requires an already
elevated Windows PowerShell and a clean committed worktree; it never self-elevates.
It makes one fixed read-only lookup and preserves the first output in a separate
exclusive lane. Do not use it to repeat the old refused command from this agent
context. `test_root_namespace.py` uses offline mocks only; no WSL is dispatched.

That original operator handoff now has an unconfirmed/lost-output outcome. Do
not execute it again. The separate [journal-first preparation](../../docs/testing/performance/0.74/wsl-clock-root-namespace-journal-plan-20261023.md)
opens outer logs before Python and all original preflights. Its entry was
`root_namespace_logged.ps1`, never either Python payload directly; that lane is
now consumed. `test_root_namespace_logged.py` uses mocked preflights
and temporary fake PowerShell/Python payloads only, not the real entry or WSL.

After later explicit authorization the new launcher was invoked once and
[refused at Administrator preflight](../../docs/testing/performance/0.74/wsl-clock-root-namespace-logged-attempt-20261023.md)
with exit 1 before WSL. That journal is now consumed: do not clear or reuse it,
invoke this launcher again, or infer a known result for the older operator
attempt. Further execution needs a distinct approved lane and an actually
elevated Windows context. No source correction or bypass accompanied the refusal.

The separately approved [UAC attempt](../../docs/testing/performance/0.74/wsl-clock-root-namespace-uac-plan-20261023.md)
uses `root_namespace_uac.ps1` and a distinct output lane. Its projection test
keeps the previous launcher identical apart from two placement tokens; the core
and collector remain unchanged. Only the controller may request standard UAC
once under that scope. Do not start either consumed historical entry.

This UAC lane is now [consumed as well](../../docs/testing/performance/0.74/wsl-clock-root-namespace-uac-attempt-20261023.md).
Administrator preflight passed, but the debug client timed out at the unchanged
30-second limit. Its raw file contains complete current root metadata, including
the `1157 → 193 → 176` timesyncd chain and root chronyd PID 210. Offline parsing
does not turn that timeout into success, authenticate historical writers or
prove guest-process retirement. Do not rerun either the controller or launcher,
clear any of the three output lanes, or change clocks/services to pass a gate.

The [offline transport audit](../../docs/testing/performance/0.74/wsl-debug-transport-audit-20261023.md)
identifies the console/process-completion contract mismatch. The following
command reads only the pinned saved archive; it never requests UAC or calls WSL:

```powershell
python -B tools/wsl-clock-trace-074/debug_transport.py
```

Its framing/client/guest outcomes are separate. The isolated preflight API
refuses process-completion use of the debug console and unreviewed alternatives;
it is not integrated into a new executor. Eight `test_debug_transport.py`
controls use only saved bytes and synthetic temporary files. All live entries
remain consumed; a finite process-tracked root exchange requires separate review.
