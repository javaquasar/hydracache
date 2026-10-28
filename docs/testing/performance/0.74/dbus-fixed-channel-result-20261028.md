# Fixed read-only channel: zbus 6 source compatibility

The selected read-only FixedManagerChannel source now compiles on the standalone
zbus backport with its original PathPins/KernelPins/Operation and sandbox read
bodies. **95/95 parent tests pass on GNU Rust 1.94 and MSRV 1.88**, with check
and strict Clippy. This advances source compatibility and real kernel-leaf
coverage; it does **not** open a real manager connection or admit an original
owner, host, product dependency graph or release/server run.

## Exact source boundary

Six complete supervisor source files are pinned. The channel source SHA-256 is
`1110a742d1806279a69a568a70399f9a0dfce9868ccd9da70550005c4d81c27d`;
the kernel source is
`2eefec1c52c06d42e88e4b7ddfdaa70593ecbedc83a6c8b95fc1037a8726164c`.
The projection includes open_fixed, inspect, revalidate, query/owner, timer-bound
work and the complete observe_sandbox read path. Namespace/wire/flag spellings
are adapted without changing their checks. inspect/revalidate and the whole
register_operation_alarm/Operation::bound bodies remain byte-identical. The
kernel body is exact apart from excluding its receiver child-module route.

ReceiverPin/original_join/held_delivery routes require the larger original-owner
frame and are explicitly outside this slice. No fake successful kernel/context/
owner object substitutes for them. Two original ignored manual host tests are
retained as source data and not registered in this no-bus test catalog; they are
not removed from the production tree or marked passed. No full AST/module closure
or live manager/kernel-frame acceptance follows from compiling this slice.

## API and fixture graph changes

Three connection API adaptations preserve the existing policy:

- The Tokio UnixStream is consumed through checked into_std at its original
  builder site, after the same kernel/path/boot observations. io maps conversion
  failure to the existing Io error; no descriptor is cloned for the transfer.
- Queue 16 becomes NonZeroUsize::new(16), preserving the bound exactly.
- AuthMechanism::External resolves through zbus::connection, retaining EXTERNAL.

The first bounded compile input retains two actual E0433 errors before any tests:
missing tokio::select in the fixture's disabled macros feature and the old root
AuthMechanism path. Their native streams/source/retirement remain. The separate
candidate fixes the namespace and enables the same Tokio macros feature already
present in product Cargo.toml. The biased select, early cap check, post-ready cap
check, timer readiness ordering and two-second absolute BOOTTIME origin are not
rewritten to avoid the feature.

The added fixture-only tokio-macros 2.7.0 archive has SHA-256
`385a6cb71ab9ab790c5fe8d67f1645e6c450a7ce006a33de03daa956cf70a496`,
equal to the root lock and the retained cached primary sparse record. Its manifest
declares Rust 1.71, build=false. Static review of the selected select generators
and entry module finds token/AST construction, with no filesystem/network/process/
environment access in those source bodies. This is a bounded source observation,
not a general procedural-macro safety theorem or a fresh advisory verdict.

The fixture has **63 package identities / 59 registry archives**, up from 62/58.
Only Tokio's macro edge and the new macro row change. The macro uses the already
selected compatible syn 2.0.119 node; product uses syn 2.0.118 for that package.
Thus the archive identity matches product, while the whole macro dependency row
does not. All other lock rows and canonical zbus source-package files remain
exact. Root manifests/lock, deny policy, supervisor and qualification identity
stay unchanged; no package is integrated/published by this step.

## Actual tests and compile-only entrypoints

All 78 predecessor cases remain. Seventeen additions comprise five original
channel tests, four kernel tests, four alarm tests, two original sandbox validators
and two adapter controls. Full actual catalogs, Cargo image/source/path/SHA,
selected/pass/ignored/filtered counts and postflight agree on each compiler.
No tests execute open_fixed, inspect/revalidate or observe_sandbox against a live
host: their new entrypoint control takes function items without calling them.
Those functions have compile coverage, not a live healthy-frame verdict.

The active original cases check bounded unique names and reply headers/flags/FDs,
expired cap refusing to poll ready work, a pending future's unchanged timer expiry,
and the original readable alarm remaining unconsumed. Real temporary directory/
socket fixtures reject UID, write permissions, symlink and replacement changes.
Kernel cases observe actual SO_PEERCRED/SO_PEERPIDFD, CLOEXEC and canonical procfs
boot data. A single owned/unreaped sleep child is SIGKILLed through its retained
pidfd; death is observed without signalling an observed/reusable numeric PID.
This is not the release's complete SIGKILL/lost-ACK lifecycle matrix.

The unchanged four alarm controls execute isolated one-test children to check
alias lifetime, cancellation/numeric-slot reuse, registration failure and ready/
unpolled paths, preserving their five-second case budget. They assert their own
one-test child summary; the existing helper does not retain separate successful
child streams. Parent results and the complete unchanged helper source remain.

The new transfer control uses an owned private socketpair: into_std keeps the
same descriptor number, CLOEXEC and status flags. Both directions carry exact
bytes; an original duplicate remains usable after the transferred owner drops,
and the peer sees EOF only after the last owner drops. This demonstrates the
ownership adapter, not execution admission or all-writer cessation in production.

## Evidence, limits and next boundary

Eight inert projection/graph/boundary controls and owned Rust formatting pass.
The pinned CI owner retains offline/locked GNU commands, disjoint targets,
1200-second outer/1100-second inner absolute origins, four-GiB floors and five-second
capacity/retirement. Original two-second Operation, 250-ms method and live 300/60/10,
package 780/workspace 4800 limits are exact. Root formatting debt and product gate
failures are not relabeled green by this scoped pass. No system/session bus,
service, clock, installation, server, product load or performance experiment runs.

The [primary packet](local-runs/dbus-fixed-channel-20261028/index.json) contains
1,412 logical members / 456 raw objects, both frozen inputs, all six original
files, manual host tests, first compile refusal, final native catalogs/receipts,
macro archive/source/metadata and controls. Index SHA-256:
`5f757928da182fd26ccb70f727d07ea0c57092f067bf8280ab9d11c649cedb13`.
Native binaries/build trees are excluded; image receipts remain. Source commit:
`2549ef0178cd44f3bc2fae7b4841d7c62b9fdb8c`.

Next resolve ReceiverPin and the complete original-owner/module closure, then
held/original delivery and actual context/source/effect coverage. Real trust/host
admission, registry shipping/consumer, remaining target/feature and final graph/
package/workspace gates remain. Chitchat's costly preview stays declined.
Production writer retirement, durable completion/cells 2–4, full SIGKILL/lost-ACK
and reconnect proof are still independent release blockers.
