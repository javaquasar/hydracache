# Separate UAC root namespace attempt

The user agreed to prepare and execute one distinct read-only attempt through
standard Windows UAC confirmation, after the [previous logged attempt](wsl-clock-root-namespace-logged-attempt-20261023.md)
refused Administrator preflight. This changes the Windows execution context,
not the collector's predicates. The old unknown outcome and known refusal stay
immutable. It authorizes no clock/service change, server use, original-owner or
product workload, repeated prompt, fallback or retry-to-pass.

## Fixed placement and unchanged execution

The new journal is `target/wsl-clock-root-namespace-uac-20261023`; the outer
request journal is `target/wsl-clock-root-namespace-uac-request-20261023`.
Neither may be cleared or reused. `root_namespace_uac.ps1` differs from the
pinned previous launcher only in the fixed output-directory and payload-name
tokens. All startup journaling, pipe handling, exclusive writes and errors are
otherwise byte-identical. Its Python adapter verifies the pinned unchanged
core, temporarily selects the new journal and delegates once to that core.
It restores the placement even when the core raises.

The original collector and journal core are unchanged, including Administrator,
worktree/source/pin, running-distro, boot, namespace, fixed-PID and completion
checks. The 300/60/10-second windows, 15-second inventory and 30-second debug
client limits are unchanged. No credential, executable or scope predicate is
replaced with an assertion that UAC was requested; the real Administrator check
must still pass in the child.

After committing reviewed sources, the controller reserves its request journal
and records exact command/source/previous-result pins. It invokes the absolute
Windows PowerShell path once via `Start-Process -Verb RunAs -PassThru
-WindowStyle Hidden`, the [standard documented administrator launch](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.management/start-process?view=powershell-5.1).
The helper is hidden; Windows owns the interactive consent prompt. No redirect
parameters are combined with RunAs: the elevated launcher owns its early logs.
The controller retains process/error receipts and does not add a new deadline
or terminate Windows, WSL or any service to obtain completion.

Cancellation, inability to display consent, start/access errors, nonzero exit
and partial journals are first outcomes, not reasons to request UAC or launch
again. A returned PID proves only process creation, not elevation or successful
root metadata. Child receipts and original checks determine the reached stage.
If process status becomes unreadable, inspect the same journal read-only rather
than resubmitting. No clocks, services, mount, policy enrollment or server
configuration may be altered to pass a gate.

## Offline controls before the request

Four new controls cover exact two-token launcher projection, unchanged pinned
core, single delegation/restoration and a temporary fake child with exclusive
new output placement. They never request UAC or invoke WSL. The first native
fixture control refused on a lexical short/long TEMP-path comparison; its log
and exact initial test source are retained. A test-only correction compares
filesystem directory identity instead, without changing the launcher or core.
All four new and 32 existing Windows offline controls pass.

Only one live consent request follows these controls. The subsequent result is
to be retained separately from both the old unknown and the known non-admin
refusal. No result is claimed by this preparation document.
