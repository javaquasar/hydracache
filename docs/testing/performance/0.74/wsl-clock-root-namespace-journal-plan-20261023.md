# Journal first root namespace diagnostic preparation

Preparation only. The user explicitly withheld a new live launch. The agent did
not invoke either real entry while preparing this candidate. Validation uses
temporary fake payloads, mocked original preflights and a read-only Windows
interpreter-resolution lookup. No WSL command, service, clock, original-owner or
server workload was invoked.

## Preserve the first unknown result

The operator reported completion of the original handoff, then confirmed that
the console windows were closed. The expected output directory is absent and
the console output was not retained. The
[first outcome record](root-namespace-first-unconfirmed-20261023.json) therefore
keeps execution, root dispatch, exit code and cause as null. It is not a failed
process receipt, a successful lookup or proof that nothing executed. The record
has SHA-256 `70a405fcf3a65394b83d3e6a500b3a4ad454ce649436c80af2a098e25ba3006c`.
Do not reconstruct it from later controls or repeat the old command.

The original collector remains byte-identical with SHA-256
`99e140b530e2200d475e7b97c31ece6681563d1ea89bf3c49e589717a62687e5`.
Its [method and earlier controls](wsl-clock-root-namespace-plan-20261023.md)
remain historical evidence. This new preparation changes logging and output
placement, not its live checks or input script.

## Capture before preflight

`root_namespace_logged.ps1` is the future operator entry. After reserving its
exclusive `target/wsl-clock-root-namespace-logged-20261023` directory it opens
stdout.log, stderr.log and launcher-errors.log and durably writes the launcher
intent. Only then does it resolve or start Python. It does not check or request
Administrator privileges itself. A missing interpreter or Process.Start failure
is retained by this outer layer, and Python syntax/import failures reach the
already redirected stderr stream.

The launcher starts one unbuffered Python process without a visible helper
window or self-elevation. Both pipes are drained concurrently into write-through
files, avoiding the redirected-pipe dependency described in Microsoft's
[Process documentation](https://learn.microsoft.com/en-us/dotnet/api/system.diagnostics.process.standardoutput?view=netframework-4.8.1).
On normal child exit it waits for both drains, flushes both files and writes an
exclusive exit receipt. A child exit is not a successful metadata gate.

`root_namespace_logged.py` requires that journal, verifies the exact original
collector and first outcome pins, then imports the unchanged collector. If old
attempt artifacts appeared meanwhile, it refuses and leaves them untouched.
The collector's output goes only to the new journal's lookup subdirectory.
The original Administrator, clean-worktree, frozen-source, identity, inventory,
boot, namespace, fixed-PID and completion-marker checks run unchanged. The
original 300/60/10-second windows, 15-second inventory limits and 30-second
debug-client timeout are unchanged; no new retry or outer timeout is added.

The adapter transparently records each original subprocess invocation and its
captured stdout/stderr, including successful git status output that would
otherwise disappear inside check_output. It passes the same arguments and
timeouts once, returns the same result or rethrows the same exception. Hidden
output in chained CalledProcessError/TimeoutExpired exceptions is retained as
separate byte files. Tracebacks and the current failure phase reach both the
outer stderr log and an exclusive structured failure record where possible.
Byte-valued captures stay unchanged; text already decoded by the original
subprocess call is saved as UTF-8. Module globals are restored after the attempt.

## Limits and operator boundary

Journaling setup necessarily precedes journaling: a script that never loads, a
script blocked by external execution policy, or a filesystem unable to reserve
or write the directory cannot guarantee a local receipt. Refusal to reserve the
directory or open the initial logs occurs before Python starts and leaves
existing attempt bytes intact. Logging failures after Python starts cannot prove
the absence of root dispatch; the launcher records known/unknown child-start
state and never turns a partial journal into success. Partial
journals or missing completion receipts remain indeterminate, never retryable.
Write-through and fsync boundaries are not a guarantee against hardware failure.

No command is authorized by this document. After a separate launch approval,
the operator must use the new PowerShell entry, not invoke either Python payload
directly. A process-scoped PowerShell execution policy can apply only to that
session; it must not change persistent policy or override Group Policy, as
explained in [Microsoft's execution policy documentation](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_execution_policies?view=powershell-5.1).
The actual invocation and evidence retention will be agreed before launching.
Do not create, clear or reuse either real output directory during preparation.

## Offline controls and first instrumentation failure

Twelve new controls pass: preflight refusal without dispatch; exact unknown/source
pins; old-artifact refusal; argument/output placement; raw captured bytes and
exception chains; unchanged subprocess arguments/timeouts; timeout without retry;
wrong journal refusal; and native Windows PowerShell fixtures for missing Python,
Python import failure, concurrent 200,000-byte stdout/stderr, UTF-8 and exclusive
output reuse refusal. Fixture payloads cannot invoke WSL or the original lookup.

The first eight-control suite had two failures before fake Python started.
A separate read-only command-resolution check showed that Windows PowerShell
returned two Application matches for python.exe. The initial launcher combined
them into an invalid Process.Start filename. The isolated correction selects
exactly the first PATH result, without fallback or a second start. The first
logs and exact initial sources remain separate from corrected controls. This
explains the new fixture failure, not the cause of the lost operator output.

## Evidence and remaining gate

The first unknown outcome is committed as `9e904af1`; executable logging code and
tests as `dc47e31a`. The [47-file controls archive](local-runs/wsl-clock-root-namespace-journal-controls-20261023/index.json)
has index SHA-256
`2085ef152e98dfd486e946fa68a1d32178f84a6261bdb6a8d383272c8d26145b`.
Exact initial/corrected sources, method snapshots, first failed fixtures,
command-resolution evidence and local gate output are retained there. All 12
new and 20 unchanged offline controls pass, as do doc-check, the local
non-promotable performance contract and both earlier archive verifiers.

Both real output directories remain absent. No new real launch is authorized
or recorded. The next gate requires separate approval for a single elevated
read-only run through the new PowerShell launcher, followed by offline sealing;
it must not be inferred from these fake-payload controls. Kernel writer identity,
debug-shell stdin compatibility and the clock policy remain unresolved.
