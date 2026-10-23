# Logged root namespace attempt stops at Administrator preflight

The single authorized invocation of the new launcher at `99f49fc3` exited 1.
Python started, then the unchanged collector refused because its Windows
process was not elevated. The traceback and structured failure record were
retained before the console closed. No collector git preflight or WSL command
was reached, and the launcher was not repeated or elevated.

## Recorded first result

The new journal is `target/wsl-clock-root-namespace-logged-20261023`.
launcher-started.json records Python PID `36272`; launcher-result.json records
child exit 1 with retry false. payload-failure.json records phase
`original-preflight-and-lookup` and the exact error:

```text
Requires an already elevated Windows PowerShell; no WSL command was run
```

stdout.log is empty, stderr.log contains 581 bytes including the complete
traceback, and launcher-errors.log is empty. This is a payload preflight refusal,
not failure to start Python. No subprocess-* records or lookup directory exist:
the pinned original collector raises at its first Administrator check, before
git status or any WSL invocation. The separate outer coordinator performed
read-only Git provenance/evidence checks; those are not collector preflight
calls. Raw outer PowerShell output, including its VoidTaskResult lines, is
retained without filtering or a code correction.

The [earlier operator outcome](root-namespace-first-unconfirmed-20261023.json)
remains a different, unconfirmed attempt. Its execution, root dispatch, exit
code and cause stay null, and its original output directory remains absent.
Its SHA-256 is still
`70a405fcf3a65394b83d3e6a500b3a4ad454ce649436c80af2a098e25ba3006c`.
The known refusal of this new invocation does not explain the earlier lost
console output or retroactively give it an exit code.

## Exact evidence and unchanged guards

The [29-file exact archive](local-runs/wsl-clock-root-namespace-logged-attempt-20261023/index.json)
has index SHA-256
`9169b0e30dc42349293137b0609ab77af653acc349f05e71dcfe41768a224c86`.
It retains one outer invocation intent/result, both outer streams, all journal
files, structured offline analysis, local gate output and exact source/input
snapshots. All four pre-launch source pins compare unchanged after exit.
Object bytes, sizes and the pinned index verify offline.

The original collector remains SHA-256
`99e140b530e2200d475e7b97c31ece6681563d1ea89bf3c49e589717a62687e5`.
No code, checks, thresholds or original 300/60/10-second windows changed. The
15-second inventory and 30-second debug-client limits remain configured but
were not exercised. No self-elevation, clock/service mutation, tracing,
original-owner run, product workload or server operation was performed by this
invocation. Existing background clock writers were not observed or controlled.

Doc-check, the local non-promotable performance contract and both earlier
archive verifiers pass. The frozen runtime/tests, Cargo.lock and COMPAT guard
against `8fb0fb5a` passes. The qualification manifest retains SHA-256
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
No unit or fake-launch suite was rerun in this step; the earlier 32-control
preparation result remains separate from this real refusal.

## Remaining access boundary

This result verifies preservation of an actual early Administrator refusal,
not root-namespace access or debug-shell stdin compatibility. Kernel writer
identity and clock policy remain unresolved. The one approved launcher
invocation is consumed; do not clear, reuse or relaunch its journal. Any further
diagnostic needs an actually elevated Windows execution context, a separately
approved distinct attempt and a new exclusive output lane. No permission to
bypass checks, change services/clocks or run on the server follows from this
refusal.
