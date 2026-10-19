# Original watchdog context preparation evidence

This package binds nonexecuting watchdog preparation to the same ready original
message owner. It creates no guardian, timer or retained FD owner and grants no delivery,
release or completion authority. The [plan](../../original-watchdog-context-plan-20261019.md)
starts at `38f8ecb2`; commits `23a5e971` and `25246007` precede implementation.
The [contract](../../original-watchdog-context-contract.toml) keeps actual
watchdog enrollment, v2 launch artifacts, cgroup/all-writer proof, production
and server qualification closed.

## Original ownership and full validation

OriginalWatchdogContext exclusively borrows the ready OriginalManagerMessage;
its original frame, collector/custodian, receiver and fixed channel remain owned
in place. No decoder, Clone, arbitrary File/time constructor or descriptor
getter exists. It derives Starting state/identity, reservation/receive/challenge/
cell origins, three absolute bounds and selected existing timer deadline,
original ten descriptor roles, two child-pidfd roles, message serial/names and
addressed writer aliases from that owner. Actual current PID/TID and real/
effective credentials are captured from the kernel, not caller claims.

Opening and closing observations enclose exactly one call to the unchanged
addressed message round. Its complete predicate set, 34 healthy authentication/
signature calls and two channel checks remain. Context comparison is not a
cached authorization result. Retained drift refuses before a full round;
closing drift refuses after it. Any error poisons the original chain and payload,
so a retry cannot enter another round. Dropping the view returns only the same
borrow, not a detached or durable authority token.
The contract's new_fd_owners=false concerns retained aliases. Required original
observations can still open temporary kernel documents; no zero-allocation or
zero-FD-syscall result is claimed.

## Controls and retained baseline refusal

Four ordinary controls cover source/descriptor/origin/bound/identity mutations,
actual thread identity, addressed alias drift and the exclusive original-owner
constructor signature. Distinct test-channel metadata probes cannot construct
the production borrowed view.

The fourteen full-frame cases are healthy preparation/revalidation; retained
owner, identity, origin, bounds, deadline, fence, alarm and addressed-alias drift;
closing fence/deadline/origin drift; closing channel-name refusal; and actual
original custody loss. Every fixture checks the same original origins,
300/60/10-second offsets, unchanged records and retained fence. Healthy and
closing-complete cases retain 34 authentication/signature calls and two channel
checks. Retained-context refusal has zero such calls/checks; live custody loss
has zero authentication calls and one opening channel check. Retry checks zero
channels after sticky refusal. These are not product throughput or allocation
measurements, actual system-bus delivery or new-boundary process-crash proof.

`baseline-frame.log` fails on an unchanged old test image after two scenarios,
with Kernel(Invalid) during fixture preparation. WSL reported repeated initial
time synchronization, but no first-refusal trace was captured in that old test,
so the exact original site is not established. Do not attribute it conclusively
to the new candidate or to NTP. Under the previously approved temporary stop of
only local WSL systemd-timesyncd, `baseline-frame-isolated.log` passes all eleven
old scenarios. The local service is restored and verified active afterward.
No clock value, source predicate or admission deadline is changed.

`red-module.log` retains the expected missing-module failure. `units-first.log`
passes all four new ordinary controls with one ignored frame gate.
`source-first.log` passes the three new source guards. `frame-first.log` passes
all fourteen initial full-frame cases; its external wrapper restores local
time synchronization in finally. These preliminary checks are separate from
the selected-source cohorts.

## Selected source and reproduction

Selected implementation is `c9b0651bbd0042c5c67217731ad4f96de73e690f`.
The GNU 1.94/MSRV 1.88 build stages run all-target checking, strict all-feature
Clippy, four ordinary context controls, thirty old exchange controls and the
unchanged seventeen-case guardian matrix. Frame stages bind the same committed
source and executed image to fourteen new controls and nine fixed bundle input
hashes before/after. Frame stages MUST run sequentially; their shared WSL clock
service cannot have concurrent owners restoring it midway through another gate.
The receipt's watchdog_spawn=false describes the new original-context path,
not the separate guardian regression harness: that harness does create owned
nonproduct guardian/target children for its existing seventeen fault cases.

Both selected cohorts pass all of those checks: fourteen context scenarios,
four ordinary controls, thirty old exchange controls and seventeen guardian
cases each. The frame test takes 148.04 seconds on GNU 1.94 and 134.70 seconds
on MSRV 1.88; these are whole-fixture wall times, not product latency or an
optimization comparison. Both receipts bind nonroot UID 1000, unchanged image
and all nine bundle hashes, plus active clock service before and after the gate.
The executed GNU image hashes are respectively
`a99a5ecc6974f6cb85f1b94a38f71310400900cd7d54b94d2171ee816eaf4fc6`
and `f04b18c41f1d38c7ea0951804a78fc296dc5ddf79a7773273ccff36c1e7cc3df`.

On this Windows/nonroot WSL workstation, with clean selected source:

```powershell
./run.ps1 -Compiler 1.94.0 -Label fresh-gnu194 -Stage build
./run.ps1 -Compiler 1.88.0 -Label fresh-gnu188 -Stage build
./run.ps1 -Compiler 1.94.0 -Label fresh-gnu194 -Stage frame
./run.ps1 -Compiler 1.88.0 -Label fresh-gnu188 -Stage frame
```

Use the script's full path or run it from this folder. Only build stages may run
in parallel. Frame stages require initially active systemd-timesyncd inside WSL,
temporarily stop that exact service through WSL root, restore/verify it in finally,
and record active before/after. This uses the explicit local testing permission;
it is not a production recommendation or permission to alter Windows/server time.
They execute only nonroot libtest fixtures and read the fixed existing bundle;
the timing-controls artifact is hashed/copied by the fixture, not executed.

The runner refuses dirty selected source, existing build cohorts, overwritten
logs, completed frame cohorts, changed source/image/bundle and frozen qualification
bytes. Reproductions create new directories; those extend the closed packet and
must not be presented as the original inventory. `verify.ps1 -SelfTest` checks
canonical UTF-8/LF/no-BOM inventory, source/image/bundle bindings, restored clock
service, exact case sets and eight in-memory negative controls. It is local
evidence integrity, not runtime authority.

`reproduction-final.log` records parser checks for both scripts and four refused
preflights: an existing build cohort, an escaping label, an unsupported compiler
and a completed frame cohort. The replay refusal precedes clock-service actions
in the runner. The local service remains active. `previous-inventory.log`
separately verifies the preceding 38-file guardian packet and eight in-memory
negative controls; that prior packet is not rewritten here.

Separate receipts cover Windows/API controls, formatting/strict affected lint,
full source contracts, registry, docs and the local non-promotable contract.
Windows has 49 passing ordinary controls and 17 compile-fail controls; the full
source suite has 271 passing controls. Registry checking reports 184 discovered
gates, all registered. The ordinary inventory does not automatically execute the
new manual gate: its two explicit fourteen-case receipts are retained separately.
The new source adapter restores all 372 prior supervisor Rust bodies, existing
wire compatibility and registry bytes exactly. No old live predicate, effect
bracket, protocol or admission origin is replaced. Qualification remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.

## Changed files and remaining integration

The implementation and ordinary controls are `diagnostic_original_watchdog_context.rs`
and `diagnostic_original_watchdog_context_tests.rs`. Test probes and full-frame
fixtures are `diagnostic_original_watchdog_context_test_api.rs` and
`diagnostic_original_watchdog_context_frame_tests.rs`. Three previous message/
test files receive exact additive child-module registrations only. The new
`performance_contract_074_original_watchdog_context.rs` guard is registered through
the previous redundant-watchdog guard. Registry, local contract, plan, design,
optimization article and this packet record the new boundary. No wire format is
added: this is a private in-process Rust view, not serialized data.

The original message remains v1. Its consent cannot authorize separately signed
v2 held-launcher roles or infer a new process/cgroup identity. The view is not an
armed watchdog or a way to feed asserted inputs into the local prototype. Actual
original enrollment still needs retained kernel/timer/fence ownership, reviewed
v2 artifacts, live executable/process/stream/cgroup binding, original delivery
and independent all-writer revocation. Durable history/completion, later cells,
complete crash/lost-ACK coverage and reconnect remain subsequent gates. No
release installation, supervisor operation, original/system-bus send, product
workload, dedicated-server operation or expensive qualification occurred here.
