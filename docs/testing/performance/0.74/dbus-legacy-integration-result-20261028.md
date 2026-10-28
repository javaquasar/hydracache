# Whole remaining production callers joined to the zbus 6 graph

The approved security continuation now joins diagnostic_manager.rs,
diagnostic_loaded.rs and systemd_unit.rs to the genuine zbus 6 Message graph in
an isolated source candidate. Root source/lock/dependencies remain unchanged.
The [plan and first failures](dbus-legacy-integration-plan-20261028.md) are retained;
this is compatibility work, not a new optimization or performance claim.

## Source and ownership boundary

Relative to the preceding 1,891-file fixture, the final tree adds one private
blocking bridge, changes three entire original caller files, the library module
route, package dependency category and two vendor-path configurations. There are
no removed files. All other old source hashes, complete Cargo.lock bytes,
upstream backport bodies and protected product/qualification inputs stay exact.
The new tree has 1,892 files; every original frame/kernel/receiver/live-check body
outside those three legacy helpers is unchanged. All old caller tests remain.

The original API projector validates whole files before editing. The two blocking
callers use the previously qualified multi-threaded bridge v2. Only unused private
from_async, close and connection getter methods are omitted; the cloned connection
is still stored in the original field order, now _connection, by both constructors.
Async proxy Drop still enters the original runtime. Constructor validation/error
priority, property caching, method timeouts, sender checks and operation order
remain. No lint allowance or dummy call is added to suppress a failure.

The old zbus 5 requirement moves from a normal Linux dependency to an explicit
Linux dev-dependency: the existing manual negotiated transport and credential
DTO still use it. Therefore **old zbus is not removed from the full test/lock
graph, and dependency policy is not declared green**. No test/target is excluded
to conceal that boundary. Root integration, transport migration, final source/
distribution/consumer review and full product gates remain separate.

## Actual GNU/MSRV gates

The final tests/native-a7 source passes all-feature library check and strict
Clippy on GNU Rust 1.94 and MSRV 1.88. Actual default library test images then
build with Cargo source/image path, SHA256 and size receipts. Both complete
catalogs agree at **1,593 tests**. Each executes exactly **64** original cases:

- fourteen diagnostic-manager cases, including owned helper IO/exit/timeout,
  snapshot binding, sender/error order and property shape;
- seven loaded-settings cases covering drift, sticky refusal, invocation/schema,
  canonical binding and exact wire types;
- two original attachable-unit shape cases;
- all forty earlier original held/addressed/pipe/dispatch/ReplyState wire cases
  and the retained credential DTO control.

Each image reports 64 passed, zero failed/ignored/measured and **1,529 filtered**.
No filtered or ignored real-bus/privileged case is counted as passed. Final strict
`--lib --tests --all-features -- -D warnings` passes on both compilers. The new
unit driver binds its whole input, exact actual image, catalog and all row/summary/
exit/filtered counts. Four projection/ownership controls and three independent
catalog/accounting controls pass. The unit command uses no system/private bus,
service, real original owner, clocks, server or product workload. Its subprocess
resource-limit controls affect only owned synthetic test children.

Every command retains the original Windows Job/Linux subreaper owner,
1200/1100-second origins, 4 GiB floors, five-second polling/retirement and all
original 300/60/10, method and Operation limits. Source/boot/vendor and owner
retirement postflight pass. No product performance or security-policy pass follows.

## First failures and capacity are not erased

The original owner-bound check fails two wire imports (E0432). The api-bound
library check passes but strict Clippy refuses unused private bridge APIs. The
first narrowing misses one constructor field (E0560); a separate longer Windows
path then refuses during inert copy (WinError 3), without a native command. The
short-path fixed candidate passes paired library gates but its first full test
build refuses two cfg-test glob ambiguities (E0659). The final candidate qualifies
only those two test-local imports as zbus6 before the actual paired unit gates.
No unchanged source is rerun to pass. Failed builds execute zero tests even where
the phase-level intent says tests_executed=true; raw compiler streams decide it.

Old a2/a3/a4/a6 build caches are removed only after owned retirement and no observed
consumer, retaining all source/first logs/receipts and the current a7 images. This
frees reusable Linux space, without initially returning Windows physical space.
As the running MSRV unit build approaches the unchanged floor, 589 old Windows
.rlib cache files dated before 2026-10-27 are removed: 3,437,817,150 logical bytes,
after checks for Windows compilers, tracked inputs and reparse paths. Source,
evidence, executables and current GNU images stay intact; Windows free space
recovers to about 7.3 GiB. No floor change, VHD compaction, service/clock setting
or foreign-process stop occurs. The original unit gate completes without a
resource refusal or replacement attempt.

## Evidence and remaining release blockers

The [282-member packet](local-runs/dbus-legacy-integration-20261028/index.json)
contains 140 raw objects, original source projections, full locks/manifests,
all first attempts, committed producer bodies retrieved by actual dispatch HEAD,
image/catalog/row accounting and source/retirement postflight. Index SHA256:
`4fa2e7769efd5ce54b471528d091a4c9ee20fb5648853a273ea41c3dc7fd89a7`.
Native binaries/build trees are excluded; exact image/source receipts remain.
The longer path's partial inert-copy tree is retained locally, not promoted to
a completed preparation or compiler result. Historical CI envelopes and cleanup
are not production all-writer retirement proof.

Next port the real manual transport off zbus 5 without changing its peer/FD/error
semantics, then finish source/distribution/root security adoption and full
package/workspace checks. Chitchat's declined preview remains declined. Real
original delivery, cessation before manual cleanup, completion/cells 2–4,
SIGKILL/lost-ACK, reconnect, trust/enrollment and compatible-host qualification
are still release blockers. The first server expiry remains separate evidence;
this unit pass cannot replace it or authorize a new server workload.
