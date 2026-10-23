# Read only WSL root namespace lookup

Historical handoff only: the operator later reported completion but its console
output was lost and the expected directory remained absent. Execution is
[unconfirmed](root-namespace-first-unconfirmed-20261023.json), not a pass or a
known failure. Do not execute the old command below. A separate
[journal-first preparation](wsl-clock-root-namespace-journal-plan-20261023.md)
preserves this unknown result; its live launch is not yet authorized.

The next metadata lookup is prepared but has not run. The current Windows
process is not an Administrator. The [previous namespace audit](wsl-clock-writer-identity-20261023.md)
and its debug-shell access refusal remain unchanged. Nine offline runner controls
pass; these are not a successful live debug-shell exchange or writer identity.

## Fixed diagnostic scope

`tools/wsl-clock-trace-074/root_namespace.py` has one explicit `collect` mode.
It does not request elevation. It refuses before any WSL command unless the
calling Windows process is already elevated. Sources must be committed and the
worktree clean; frozen runtime, original outcomes, capture and writer-identity
archive pins must verify. The WSL executable uses the exact local System32 path.

The runner first reads the running-distro inventory. It requires exactly the
already observed Ubuntu and docker-desktop; it does not start a missing distro,
HydraCache074Native or a stopped VM. There is one debug-shell dispatch with an
outer 30-second timeout and no retry. Microsoft documents this shell's
[root namespace access](https://github.com/microsoft/WSL/blob/master/doc/docs/debugging.md#root-namespace-debugging).
Its actual stdin/output behavior remains a live gate, not something mock tests
can establish.

The supplied shell script requires UID 0, the original boot
`cf222c07-5476-4f70-9a32-455e99204d0a`, time namespace `4026531834`, and a PID
namespace different from the three previously accessible distro views. Observed
context values are printed before a mismatch refuses. The script reads only
fixed PIDs `1 210 1157 1126 1112 920`: before/after stat, status, cgroup,
executable/namespace links and executable SHA-256. Missing PIDs and individual
field errors are explicit metadata results, not a reason to redispatch. It does
not read command lines or environments, scan for replacement PIDs, install
packages, attach a debugger, call nsenter, mount anything or send process signals.

Raw stdin, stdout, stderr, intent, source hashes and exit results go to the
exclusive `target/wsl-clock-root-namespace-20261023` directory. A zero client exit
without ordered, exact script completion markers refuses. On lookup failure the
runner still attempts a read-only inventory/source postflight. Timeout or partial
output remains an incomplete first result; it does not prove that all helper
processes have exited, and must not trigger WSL termination or a retry.

## Operator handoff

From an already elevated Windows PowerShell, run once:

```powershell
Set-Location -LiteralPath 'C:\Workspace\prj\jq\cashe\hydracache-074'
python -B tools/wsl-clock-trace-074/root_namespace.py collect
```

Do not invoke this command from the current non-elevated agent context to retry
the old debug shell. Do not delete the output directory or rerun after failure.
If the pinned boot or distro inventory changed, preserve that refusal and agree
on a new diagnostic scope; do not restart anything to recreate the old context.
After this single command returns, preserve its output directory for offline
interpretation and exact-byte sealing. No trace, clock policy change or
original-owner run is bundled into this handoff.

## Offline validation

Nine runner controls cover non-admin and dirty-worktree refusals, inventory
decoding/drift, exact read-only targets, one-shot success/failure/timeout,
partial/reversed markers, exclusive output, postflight after failure and source
drift. Eight existing identity and three attribution controls also pass on
Windows. Git Bash `-n` accepts the exact stdin script without executing commands.
The initial six-control suite had two mock errors: the fake git command returned
bytes even when `text=True` requested a string. Those logs remain separate from
the corrected tests; production runner behavior was not weakened to fit them.

The actual non-admin entry additionally refused before WSL dispatch and did not
create the root-lookup lane. No full Linux suite or root metadata session ran in
this step. Product runtime and original 300/60/10-second admission windows are
unchanged. The 30-second diagnostic client timeout is not an admission window.

## Interpretation boundary

Completion means only that the supplied metadata script completed. It does not
authenticate a historical trace PID, prove the survival of earlier init tasks,
admit original delivery, or establish a production clock policy. Any surviving
process must still be joined using its boot, start ticks, namespace chain and
executable, with the post-capture observation boundary stated explicitly.

## Retained offline controls

Runner and method commit: `892aeddb`. The separate
[39-file controls archive](local-runs/wsl-clock-root-namespace-controls-20261023/index.json)
has index SHA-256
`b432abc379020f7a48cd38416d68246122502b61ac60db831339935f1a384501`.
It retains the first mock failure, corrected/expanded controls, actual non-admin
refusal, stdin syntax-check receipt, local gates and exact offline sources.
The actual non-admin entry preceded the absolute WSL-path hardening; that guard
remained unchanged, while mocks cover the final argv. No real debug-shell
invocation occurred in either candidate. Root lookup output is still absent.
