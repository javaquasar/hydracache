# Consuming original placement and retained held roles

Status: implementation candidate. The complete local Linux join gate has NOT
passed. Inventory verification is evidence integrity, never execution acceptance.

Implementation d9d37a8a; final selected source
f07dcca31e2e69796644d70fef695404b22d1519. Inherits docs/RULES.md and the
[consuming join plan](../../held-consuming-join-plan-20261020.md).

The private production adapter consumes FixedPlacement, PinnedRoles and fixed
HeldTrustInput. It retains them together on success and in a failed holder, with
no original/FD/source/retry extractor or start/send/unlock method. Explicit
retirement precedes field Drop, and placement/original custody drops before role
descriptors and held trust. No role or trust replacement enters revalidation.

Two constant-depth rounds each invoke the unchanged complete FixedPlacement
frame before and after fresh held trust/provenance and full role-content checks.
That is four full frames, not a reduction of the previous predicates. The final
original frame/clocks follow hashing. Original 46 brackets and 300/60/10 origins
and deadlines remain unchanged. This extra work has not been cost-qualified.

The private trust adapter reuses the original SignedControllerInput guard,
matches the exact retained signed v3 bytes and freshly verifies v4/receipt under
independent fixed configuration. Actual request/authority revision, epoch,
policy and trust snapshot must agree across rounds. The private role adapter
matches the entire consumed build statement, then runs unchanged positional
content/stamp/seal checks. Identical images with another valid log/receipt fail.

Seventeen new controls comprise ten orchestration/signature/drop-model controls,
six real local kernel/authority/sealed-memfd bridge controls and one wrapper of
an unchanged preceding assertion under the existing kernel refusal trace. The modeled
four frames cover 46 refusal cuts each (184), eight stage cuts, retained failure
and drop ordering, unchanged modeled bounds, budgets and 128 seeded substitutions.
Those 184 cuts are NOT a real full-frame fault matrix. Bridge tests use synthetic
keys, owned temp configurations and signed synthetic ELF documents. They execute
no represented images and do not construct a privileged FixedPlacement holder.

The production consuming type/projection compiles against the actual fixed
placement/frame owner. A new privileged end-to-end consuming-holder run is still
a separate gate. local-live-prerequisites.log is read-only WSL preflight: cgroup2,
root and system.slice are available and timesyncd is active. It is not a healthy
composition result or an environment blocker. Subsequent clock-control attempts
and their exact local service restoration are recorded separately below.

checks.ps1 freezes source inputs before/after, records compiler and recipe hash,
and binds actual integration/lib test image hashes before two invocations.
New controls, preceding trust/fixed controls, existing integration controls and
capability docs have separate selections. Exact source/adapter guards preserve
all 443 old runtime files after three hooks, plus old test assertions, formats,
pins, registry and qualification. Earlier sealed packets are not edited.

baseline-fixed.log records five previous fixed-placement controls before hooks;
missing-module-red.log records the expected missing role adapter. The first
compile and first 16-test cohort are development checks, not frozen acceptance.
No code candidate was accepted by changing a deadline or threshold.

The first frozen GNU cohort at 26d88288 was NOT accepted: its second preceding
trust selection failed 25/26 after both new 16-test rounds passed. The unexpected
error value was not logged, and its cause remains unclassified. Eight fresh
diagnostic samples of the unchanged assertion passed with no kernel refusal;
this does not attribute the first failure. [The retained refusal](preceding-refusal.md)
keeps that uncertainty explicit. first-checks.ps1 and all first logs/receipts are
unchanged; confirmed/ belongs to a different source/recipe, not an overwritten run.

The second cohort also refuses. Its MSRV trace records same boot, boottime
increasing 141803453 ns and realtime decreasing 6642254088 ns at clock-order.
observed-clock-refusal.json copies those values. This proves that later clock
refusal, not the cause of the unlogged first error or timesyncd causation.

The third stable-clock/ cohort uses the previously approved exact local WSL
timesyncd stop/restore. It still records a clock-order rollback. The service was
inactive during that sample and restored active in finally; the before/after
monotonic journal proves the sequence. That experiment is not accepted, and no
Windows clock setting is changed. Its complete=false receipt is retained.

held-session/ uses one continuously observed WSL process, fixed preparation
before fresh test origins, and no clock-service changes. The observer remains
alive through GNU/MSRV execution and records any test-phase rollback; such a
rollback refuses the cohort even if tests independently happen to pass. This is
a local fixture lifecycle experiment, not proof of production time accuracy or
the historical cause of either earlier failure. All first three recipes/logs
remain intact, and every runtime clock/expiry predicate remains unchanged.

That fourth cohort also refuses at the unchanged clock-order predicate. Its
100 ms observer records no backwards samples while the finer original guard
records an out-of-order realtime sample. A coarse observer therefore cannot
replace the original live checks or establish a globally stable clock. Two
200000-sample libc/direct-syscall/Python probes and a 20000-sample forced CPU
rotation probe do not reproduce that discrepancy; they do not prove its cause.

affinity-smoke/ is explicitly limited to CPU 0 for Linux runtime test images.
An initial invocation of the exact same 17-test image passes under taskset -c 0;
the source-frozen recipe records that affinity separately. It does not relax a
guard, modify a clock, fix the host or qualify unrestricted CPU execution.
unrestricted_cpu_gate_passed remains false in every receipt. Parallel-across-CPU,
privileged full-holder, clock attribution and release gates are still open.

The CPU-0 hypothesis also fails on repetition: GNU's second 17-test round fails
16/17 with Input(Kernel(Invalid)) before the metadata mutation; its exact site is
not recorded. The previously traced clock-order failures do not automatically
attribute this separate sample. MSRV's first preceding trust selection fails
25/26. No complete Linux execution receipt is issued for either compiler.
One successful 17-test round and two successful 38-test rounds per compiler do
not turn these failed cohorts into acceptance.

compile-model/ separately passes two rounds of ten new orchestration models and
five preceding fixed models on each compiler, eleven capability examples and
strict lint. These are explicitly model/compile-only receipts, not the live
join gate. The first concurrent GNU wrapper startup refused an existing output
directory before executing checks; starting that mode after directory creation
does not alter the MSRV receipt or any tested source. Windows passes two rounds
of 28 controls/four docs; eighteen source guards, three predecessor checks,
strict lint, fmt, registry (186), docs and local contract pass independently.

No server access/installation, supervisor operation, original message delivery,
production enrollment/build, product workload, joined SIGKILL/lost-ACK or release
qualification occurs. No latency, CPU, allocation or throughput claim is made.
After sealing, verify inventory, receipts, selections and in-memory metadata
refusals without changing this packet:

```powershell
./verify.ps1 -Source f07dcca31e2e69796644d70fef695404b22d1519
```
