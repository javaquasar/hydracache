# Original placement preparation time evidence

Baseline ddbfe9edee97624fb6ab6a8bcb25db93045ea58d retains the previous
[partial live result](../owned-placement-20261019/README.md). Selected test-only
source is 8e976d769e00f0e818711ab4fc041a1b15b61a03. The
[plan](../../placement-preparation-plan-20261020.md) and
[design](../../placement-preparation-design.md) preserve the entire old fixture,
all live observations and original 300/60/10 origins/bounds.

The new wrapper copies already-used kernel-time and raw-age arguments across
preparation, catches unwind inside both captures, emits after original cleanup
and resumes the same panic. It adds no admission clock or retry. Nested join
facts keep their old scope. Raw-age buckets contain no sampled boot identity;
only kernel-time facts can report their actual boot IDs. Neither a new healthy
call nor a later failure recovers the unobserved historical tuple.

## Baseline construction and ordinary checks

baseline-images.log pins the prior GNU/MSRV images; baseline194.log and
baseline188.log each pass the eight old placement model tests. The old packet's
twenty mutation controls pass without changing it. red-missing-trace.log records
the missing helper before implementation; green-check.log compiles the new
helper. All 430 prior source bodies, COMPAT, the registry and external harness
are reconstructed by three exact new guards; no old source body is ignored.

build.ps1 requires clean source and stable HEAD. It pins compiler, recipe and
ordinary unprofiled image, runs strict all-target/all-feature lint, five new
trace controls, 61 unchanged selected units and all 247 Linux doc tests. The
trace tests cover one call, unchanged return/error/panic identity, first refusal,
nested/concurrent restoration and an actual already-received kernel point.
These 66 selected units are not the entire 1,306-entry Linux library suite.
Windows has 60 ordinary units and twenty doc tests. Full source-contract output,
portable lint, registry and local contract logs are separate from live results.

For reproduction, copy the recipes from this evidence revision into a separate
checkout at the selected source, preserving their packet-relative location but
not output directories. Never overwrite the retained packet or share one
compiler target directory between simultaneous builds.

```powershell
./build.ps1 -Compiler 1.94.0 -Label gnu194
./build.ps1 -Compiler 1.88.0 -Label gnu188
```

The first Windows cargo fmt --all invocation exceeds the OS command-length
limit (206). The scoped cargo fmt -p xtask -p hydracache-long-run-supervisor-074
and its -- --check invocation avoid that tooling limit without dropping either
changed package. fmt-check.log records the successful check.

## Local live method

After builds and all heavy checks finish, live.ps1 runs the unchanged seven-case
fixture serially on each selected image. It temporarily stops/restores only the
previously approved local WSL systemd-timesyncd, requires 65 stable seconds and
never sets a clock. Its pre-run identity is retained even if a native test fails.
After each compiler, the wrapper checks empty temporary cgroup roots and fixed
source/image identity before permitting the other independent compiler cohort.
It does not retry any failed attempt.

Each successful compiler must complete healthy, revalidate, unarmed, wrong-key,
different-consent, foreign-root and trust-drift. Only the first two may accept.
All old live assertions remain: original offsets and origins, timer still live,
Busy while custody is owned, EOF and dead custodians before raw-lock availability
in the never-Armed case, typed replay refusal, and post-Drop Busy for Armed cases.
Time/Expired cannot substitute for an intended negative. The causal and raw-age
preparation rows show actual copied observations; they do not decide admission.

```powershell
./live.ps1 -Source 8e976d769e00f0e818711ab4fc041a1b15b61a03 -Label live-1
```

compiler-outcomes.json records the actual WSL process exit codes, not a guessed
native libtest exit status. outcome.json has a separate boolean full live gate;
logs from one compiler cannot fill a missing case on the other. The fixture
uses synthetic issuers, actual retained manager/root custody and temporary root
names, not approved artifact objects or proven expected systemd slice ancestry.
No original worker, held systemd unit, product workload or server is started.

## Actual result and next cost attribution

All 315 source contracts pass. Each compiler passes the 66 selected Linux units,
247 doc tests and strict lint; portable units/docs and local checks also pass.
The actual GNU healthy attempt refuses in prepare_owned_root_guardians before
placement binding. Both existing observers copy the same expired comparison:
issued 884474515163, now 894476360325 and age 10001845162 ns, with identical boot
c56f74ec-65b5-4a21-964d-cd04ce280385. The raw caller is
diagnostic_collector_causal.rs line 109. This new reproduction proves expiry,
not backward BOOTTIME or boot mismatch. It is not the missing historical tuple.

The first custody-final observation is already at age 8736206870 ns; the last
healthy final-clock age is 9901326075 ns before the failing window comparison.
These locate elapsed original budget but do not attribute it to CPU, filesystem,
hashing, signature work or scheduler delay. No isolated optimization is chosen
from those durations alone.

The independent MSRV cohort passes all seven cases once. Its last raw custody
ages span 7392394267–9803213256 ns; intended negatives do not substitute expiry
for a binding/trust refusal. GNU remains red and the full two-compiler gate is
false. The WSL exit codes are 101 and zero respectively. Clock restoration and
empty roots are verified after both, and all nine signed fixture hashes match.

Next, use existing test-only cost spans and fixed stage markers to attribute
where the pre-custody budget is spent before proposing a single isolated change.
Keep actual cost probes separate from a future unprofiled complete functional
matrix; no blind rerun, altered origin or relaxed check follows from this result.

## Verification

verify.ps1 checks raw file hashes, recipes/images/source, tests, intended live
outcomes, clock/root cleanup, unchanged signed fixture inputs and frozen previous
packet/qualification digests. Self-tests reject twenty metadata/inventory
mutations, including fabricated full-gate promotion or historical tuple recovery.
Evidence integrity and a successful ordinary suite cannot replace a live gate.

```powershell
./verify.ps1 -SelfTest
```

There is no allocation, preparation-speed or product-throughput claim. Actual
fixed slice placement, artifact admission, held projection, authenticated original
delivery, durable completion/cells 2–4 and reconnect remain separate work. No
installation, supervisor operation, qualification or product workload is opened.
