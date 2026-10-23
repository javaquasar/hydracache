# Debug console cannot prove guest process completion

The [retained UAC timeout](wsl-clock-root-namespace-uac-attempt-20261023.md)
exposed a transport-contract error, not evidence that the metadata script needed
more than 30 seconds. The saved stdout contains the complete framing and
`logout`, while the Windows client timed out. The safe correction is to reject
this console as a guest-process completion interface, not to raise the deadline,
treat the marker as an exit receipt, or repeat the lookup.

## Source review and causal limit

The already sealed version output reports WSL `2.5.10.0`. Its release tag resolves
to commit `642331364dda7a3d88bf64acb87e7056918a5fc9`. In that release,
[RunDebugShell](https://github.com/microsoft/WSL/blob/642331364dda7a3d88bf64acb87e7056918a5fc9/src/windows/common/WslClient.cpp#L1367-L1413)
opens a named pipe and relays its contents to stdout; it has no command-specific
guest exit receipt. A [maintainer explanation](https://github.com/microsoft/WSL/issues/9535#issuecomment-1404148600)
also distinguishes this debug channel from process-tracked shells and calls
the non-exit behavior intentional.

This establishes the interface mismatch and explains why logout cannot satisfy
the collector's process-wait assumption. It does not identify the exact native
instruction at which this installed client waited: no native stack was captured,
and the reviewed source is not an attestation of the installed executable.
Neither the stdout nor that explanation proves guest-process retirement.

The source-review receipt pins the release ref, complete source digest, reviewed
function digest and maintainer-comment URL/digest. It retains findings rather
than copying the whole upstream file. A local wrapper then refused a receipt
filename collision after the review had written its own result. The result,
streams and wrapper error remain; the fetch was not repeated and no child exit
receipt is invented for that wrapper failure.

## Isolated offline correction

`tools/wsl-clock-trace-074/debug_transport.py` has no subprocess, UAC, WSL or
network operation. Its fixed CLI reads the pinned earlier archive and prints an
offline assessment. The isolated code/tests are committed as `3c90aa48`. It separates:

- standalone framing, without interpreting echoed commands as markers;
- the unchanged client timeout/exit receipt;
- the still unknown guest exit and retirement status.

Framing is not metadata/identity validation. Even a zero Windows client exit
does not fill the guest-status fields. The original metadata checks are not
replaced by this assessment. `require_process_completion` refuses the debug
console and every unreviewed alternative before any possible dispatch, with no
fallback. This is an offline preflight API candidate, not a wired replacement
executor. There is no new live entry point, and the consumed collector, logging
core and launchers remain unchanged.

The eight new tests exercise the actual saved bytes and synthetic negative
cases: echoed, missing, reversed, duplicate or decorated markers; logout/EOF;
client exit zero; invalid timeout receipts; widened limits/windows; changed
input objects and original collector; and unreviewed transport fallback.
All pass. The 25 existing original/journal/UAC orchestration controls also pass
on Windows with mocks or temporary fake children, never WSL.

## Evidence and unchanged scope

The [47-file archive](local-runs/wsl-debug-transport-audit-20261023/index.json)
has 36 content-addressed objects and index SHA-256
`6e3f61836b0940c39b39d277b29188871c961a21442fac384ad318badf567a4f`.
It retains baseline/focused test results, the offline audit, local gates, source
review, wrapper error, source snapshots and pinned prior version/UAC indexes.
The assessment reproduces offline. The first UAC timeout and older unknown and
non-admin outcomes retain their independent hashes.

Doc-check, the local non-promotable performance contract and the earlier
identity/capture verifiers pass. The frozen runtime/tests/Cargo.lock/COMPAT
guard passes; the qualification manifest retains SHA-256
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
No live predicate, original 300/60/10-second window, 15-second inventory or
30-second debug-client limit changed. No new WSL command, service/clock change,
original-owner, product or server run occurred.

## Next boundary

The root metadata is already retained; another debug-shell invocation is not
needed to recover it. Do not kill a client on the end marker and relabel the
result as successful guest retirement. Do not automatically switch to another
distro view: the retained namespace evidence shows why a PID lookup there is
not equivalent to the root lookup.

Any future finite root-process experiment needs an explicitly reviewed
process-tracked channel plus the same boot/namespace/identity and retirement
proofs. It is a separate design/admission decision, not an implicit fallback.
For the current release work, the remaining independent decision is a safe WSL
clock policy accounting for the shared time namespace and observed writers.
This transport audit neither stabilizes those clocks nor authorizes changing
them. Original delivery and the full SIGKILL/lost-ACK matrix remain gated.
