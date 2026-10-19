# Original pending watchdog custody evidence

This package connects Pending guardian preparation to the original borrowed
message owner and its existing fence/timer. It does not enroll a writer, deliver
the message, arm a production watchdog or permit completion. The
[plan](../../original-pending-watchdog-plan-20261019.md) starts at `f94b4b56`;
plan commit `279bcc47` precedes implementation
`4b946bbeee1341088c2a1af9504ae3ae4f57da60`. The
[local contract](../../original-pending-watchdog-contract.toml) keeps production,
all-writer proof and server qualification closed.

## Kernel ownership and original validation

The kernel adapter takes borrowed fence/timer descriptors, never an asserted
original identity or deadline. Two independently cloned guardians inherit the
same fence OFD and aliases of the same already-armed timer OFD. It performs no
flock, timer creation, timer read or rearm. Before a successful Pending notice,
each child closes everything except its fence, timer, creator pidfd and private
channel. Existing credentialed
Pending notices and the prearmed child loop remain byte-identical; there is no
new wire format or public enrollment method. No target is handed to this adapter.

Atomic clone3 pidfds own cancellation before restoring the parent's signal mask.
Cleanup signals and waits through those pidfds, not a reusable PID. A partial
second-child failure cancels the first while the original borrow and original
collector/custodian remain live. Only the two new Pending children are cancelled;
this neither unlocks the original fence nor proves completion.
Drop is last-resort best-effort cancellation, not an acknowledged cleanup
receipt. A cleanup error cannot authorize completion or fence release.

The original Pending owner consumes the exclusive OriginalWatchdogContext. It
derives the borrowed FDs from the still-retained original storage/alarm objects
and checks their original roles. Full original rounds enclose kernel preparation:
68 authentication calls, 68 signature calls and four channel checks on the
healthy fixture. Revalidation retains one unchanged full round, with 34 of each
and two channel checks, bracketed by Pending observations. Failure poisons the
same original chain; setup failure retains that refused borrow in its failure
owner. The distinct test-channel probe cannot construct either production owner.

The new parent handles do retain timer aliases and child/channel resources.
This is not a zero-FD, zero-allocation or performance result. Pending never means
Armed, responsiveness or cessation: SIGSTOP can leave a nonterminal pidfd, and
the unchanged targetless child parks with its fence on creator loss or expiry.
Complete cgroup/external-writer revocation, v2 launch artifacts and actual
original delivery remain unimplemented by this adapter.

## Controls and retained failures

Four ordinary kernel tests check every invalid/duplicate inheritance role,
actual shared timer OFD status and unrelated writable-FD exclusion, partial
clone/restoration cleanup with the exact original signal mask, and refusal
after moving the mechanics owner to another thread. The OFD status mutation
uses only a newly created owned unit fixture, not any original frame's timer.
The constructor control checks the Rust consuming signature; it does not open
an installed system-bus channel.

Ten explicit full-frame cases cover healthy Pending/revalidation; first/second
clone failure; first/second restoration error after real restoration; actual
SIGKILL/reaping of each new guardian before closing admission; retained/closing
origin drift; and actual original custody loss. Healthy setup has 68/4 auth/
channel observations; retained drift has 0/0, ordinary setup/guardian/closing
drift refusal 34/2 and inside-custody loss 34/3. Sticky retry checks zero channels.
Each fixture independently checks unchanged records, original origins,
300/60/10-second offsets and retained original fence. These are not the complete
future send/release/durable-write SIGKILL or lost-acknowledgment matrix.

The previous 41-file closed context packet is the baseline. The fresh old-image
rerun in `baseline-frame.log` refuses after three cases during old fixture setup.
Its first-refusal trace identifies clock-order: BOOTTIME grows from 39856045060
to 40473337723 ns while CLOCK_REALTIME falls from 1792370255968957841 to
1792370250568576142 ns. The old live predicate correctly refuses the approximately
5.4-second backward step. This records an observed clock fault, not its external
cause; it does not retroactively explain an earlier packet's untraced refusal.
Local WSL timesyncd was already stopped during this rerun and restored afterward.

`red-module.log` retains the expected missing-module failure. `check-first.log`
and `check-second.log` retain new-wrapper visibility, storage conversion and
borrow-lifetime compilation errors. Those were corrected only in new code and
additive private registrations. `clippy-first.log` retains the new large-error
warning; boxing the cold failure error fixes it without a lint allowance.
`units-first.log` passes four initial kernel controls; the later exact-mask
assertion compares the whole before/after mask. `source-first.log` and
`source-before-commit.log` each pass three new source controls, the latter with
the exact registry block frozen. `frame-first.log` passes all ten initial frame
cases after a 65-second local clock observation with no backward sample; it
restores/validates the local service in finally. No original predicate or
admission window was changed to obtain that result.

Both `gnu194-source` and `gnu194-warm` subsequently refuse on the strict
clock-probe, before either frame gate starts: backward steps of 5969402419 and
5851547969 ns respectively. Their failed logs/builds and missing completion
receipts are retained. `gnu188-source` has build/unit/regression results only;
the serial driver never reached its frame stage after the first failure.
`run-before-settle.ps1` retains that runner version. No failed cohort is promoted.
The read-only environment log records WSL boots and service activity; it does
not establish the external cause of either step. The separate clock study
observes a stable 65-second interval without creating an admission.

The final recipe separates environment stabilization from the unchanged strict
probe. With only local WSL timesyncd temporarily stopped, it records every early
backward step and waits for 65 observed stable seconds, bounded by 180 seconds.
It then runs the exact previous strict 65-second probe, which still refuses
on any backward sample, before either original fixture is created. These are
workstation preparation observations, not authority or a renewed request lease.
Any later step remains subject to all original live ordering checks. Both final
cohorts bind this recipe's execution-byte hash; the verifier compares the strict
probe with the old recipe byte-for-byte after the declared text normalization.

## Selected source reproduction

Both final cohorts pass all declared gates: four kernel controls, one consuming
constructor type control, four old context unit controls, thirty exchange
controls, seventeen guardian regression cases, ten Pending frame cases and
fourteen old context frame cases each. Pending/context fixture wall times are
95.73/132.29 seconds on GNU 1.94 and 95.85/135.11 seconds on MSRV 1.88. These are
whole-fixture times, not product latency or a performance comparison.

The executed image hashes are respectively
`3ef2399bc3ede47e04db363b0789c942d36c7c8f300c939eac52a9e26a204db7`
and `d10ffb76f8218b1120a5ca3147b28c80a137cde0794faa8a8c9605cd4de57e08`.
Both receipts bind nonroot UID 1000, the same selected source, recipe hash
`18aba0bb7be9679d93e6205edf196784e073334bbadedf20195e9f8d23a0c9d8`,
unchanged images/nine inputs and active clock service before/after. Both final
settling observations report zero noticed backward steps; this does not erase
the earlier refused observations or establish the clock's external cause.

The full source suite passes 274 controls, preserving all 376 old supervisor
Rust bodies, old wire and imported harness bytes. Windows passes 49 unit and
17 compile-fail controls. Registry checking reports 184 discovered gates, all
registered; the new manual gates are executed explicitly, not inferred from
that registry count. Current/MSRV all-target checks, strict lint, formatting,
documentation and the local non-promotable contract pass.

With clean selected source on this Windows/nonroot WSL workstation:

```powershell
./run.ps1 -Compiler 1.94.0 -Label fresh-gnu194 -Stage build
./run.ps1 -Compiler 1.88.0 -Label fresh-gnu188 -Stage build
./run.ps1 -Compiler 1.94.0 -Label fresh-gnu194 -Stage frame
./run.ps1 -Compiler 1.88.0 -Label fresh-gnu188 -Stage frame
```

Use the script's full path or run it from this folder. Only build stages may run
in parallel. Each build stage performs all-target check, strict all-feature
Clippy, the four kernel controls, one constructor control, four retained context
unit controls, thirty exchange controls and the unchanged seventeen-case
guardian matrix. This last regression harness does spawn owned nonproduct
guardians/targets; its results do not supply original enrollment authority.

The final cohorts are `gnu194-settled` and `gnu188-settled`. Frame stages MUST be
sequential because they share one local clock-service control owner. They
require active systemd-timesyncd only inside WSL Ubuntu,
temporarily stop that service using the previously approved local permission,
and restore/verify it in finally. Environment settling and the unchanged strict
65-second workstation observation occur before fixtures and authenticated
receive/admission origins are created. They change no clock and waive no live
predicate. A passing observation does not
guarantee future clock stability; original frame checks still enforce ordering.
Windows time, WSL configuration and the dedicated server are not changed.

Each frame stage executes ten Pending and fourteen retained context cases on
the same selected image, hashes nine fixed bundle inputs before/after, and
checks source/image stability. The existing timing-controls artifact is hashed
and copied by fixtures, not executed as a product workload. Receipts bind source,
compiler, nonroot UID, image, fixed qualification digest and service restoration.
An existing/partial cohort is retained, never overwritten. Reproductions create
new cohorts rather than replacing the original evidence.

The final reproduction controls refuse an existing build cohort, escaping label,
unsupported compiler and replay of a completed frame cohort before clock-service
actions. `reproduction-preflight-audit.log` separately checks script parsing,
byte-equivalent strict preflight and the recipe hash in both builds.
`previous-inventory.log` verifies the preceding 41-file packet with eight
in-memory negative controls under its historical verifier, not the corrected
complete-byte canonicalization below. Raw environment output, including the native WSL
version command's NUL-containing text, is retained without repairing its log.

The first inventory verification refuses at the recipe hash. On this workstation,
culture-sensitive StartsWith(BOM) returns true even for ASCII `param`; both the
initial inventory builder and verifier silently stripped its first character.
`inventory-first.log`, `manifest-before-bom-fix.json` and the BOM diagnostic retain
that failure. The corrected verifier and inventory builder use ordinal BOM
comparison. Six in-memory canonicalization controls preserve ASCII/non-BOM
Unicode prefixes, strip only an actual BOM, normalize CRLF, detect a first-byte
mutation and reject invalid UTF-8. The eight inventory mutation controls remain.
Historical packets are not rewritten or silently presented as complete-byte
bindings. `historical-prefix-audit.ps1 -Verify` separately checks four affected
packets against full Git baseline `f94b4b56`, exact file sets and corrected
ordinal canonicalization. Its append-only JSON binds all 182 unchanged files,
including their historical manifests/verifiers. This closes the committed
historical content's missing-prefix coverage, not the safety of those legacy
verifiers for new inventories. Original binary hashes, the unchanged
qualification digest and this package's actual test runs do not depend on the
text-normalization bug.

`verify.ps1 -SelfTest` validates the closed canonical UTF-8/LF/no-BOM inventory,
both cohort bindings, exact case sets/counts, old regressions, local gates and
eight in-memory negative controls. This is evidence integrity only, not runtime
trust, a policy enrollment receipt or server admission. Qualification stays
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.

## Changed files and subsequent gates

New mechanics/tests are `diagnostic_inherited_pending_watchdog.rs` and
`diagnostic_inherited_pending_watchdog_tests.rs`. The original join, distinct
test probe and full-frame matrix are `diagnostic_original_pending_watchdog.rs`,
`diagnostic_original_pending_watchdog_test_api.rs` and
`diagnostic_original_pending_watchdog_frame_tests.rs`. Six previous Rust files
receive exact private module/reexport registrations only. The new source guard,
previous guard registration, manual gate and local contract freeze that boundary.
Plan, design, article and this packet describe the scope.

Pending custody is not original start. The next join must authenticate the
reviewed v2 artifacts and actual executable/process/streams/complete cgroup,
retain original timer/fence ownership through target enrollment and independently
revoke every writer. Do not expose raw-pidfd enrollment or remove the old local
direct-child predicate to turn this Pending adapter into a production permit.
Original/system-bus delivery, durable uncertainty/completion,
later cells, complete new-boundary fault coverage and same-attempt reconnect
remain later gates. No release installation, supervisor operation, dedicated-server
action, product candidate, long qualification or numerical performance claim is
part of this package.
