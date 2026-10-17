# Fixed origin grouped collector preparation

This packet checks a new nonexecuting production owner, not a service launch.
[Preregistration](../../diagnostic-fixed-grouped-preparation-plan-20261017.md)
starts at 8193e64202a5cff9fe769c0238534ff201151625. Implementation is
7ddcd6f3bd6507e945048147971a5cc4bca15be6. The original source and binary cohort
supplies all healthy/timing and persistent-drift results below. A separate
two-line origin-harness correction has its own capsule and binaries; it does not
replace or reattribute those measurements.
The corrected origin source is 1d0c9e4eacb54e6803ccc24e57ef1ae9daaf16ff,
preregistered after the negative in 1efa9f71. Four original/corrected binary
identities are separate in manifest.json.

## What changed

OriginalCollectorSinks::prepare_original_grouped consumes the acknowledged
original sinks and takes no replacement inputs. Its only production loader is
inspect_fixed_install(original identity, original checked builder policy).
Fixture path/key selection and internal callbacks are cfg(test) only. No fixed
refusal falls back to a fixture. The type retains private bundle descriptors,
unsent two-FD message, absolute timer and the original sinks/fence; it exports
revalidation and refusal state, never send/start, extraction, rearm, cloning,
deserialization, completion or recovery.

Construction uses one existing material_round opening and one full original_round
closing, with original build/state/message/FD bindings and final timer inspection.
The ready flag is set only after closing succeeds. Failure latches the original
chain and retains the whole owner until explicit drop. Reserved/Starting/progress
kernel-receive origins remain 300/60/10 seconds. The timer is not an independent
watchdog. Persistent closing drift is rejected; restored transient drift between
endpoints is outside this agreed grouped observation guarantee.

The rejected separately bracketed alarm, old grouped candidate, all original
runtime validators and prior evidence stay unchanged. A static guard compares
the duplicated alarm primitive and closing round exactly with the old reference
after only the error-type rename. Negative controls change deadline equality
and progress origin. Narrow exact registration adapters preserve older source
comparisons; no behavioral diff is normalized away.

## Healthy and persistent refusal results

The first Rust 1.94 screen passes, followed by three fresh healthy chains on each
compiler. All seven constructions and subsequent complete revalidations pass,
visit five ordered internal checkpoints, authenticate/sign 34 IPC messages per
phase, retain original origins and unchanged durable records, hold the fence,
and refuse a fresh retained context after owner drop. phases.json derives all
14 spans directly from the raw eight-stage reports.

| Repeated inclusive span in milliseconds | Rust 1.94 | MSRV 1.88 |
| --- | --- | --- |
| Grouped construction | 361.724–364.171 | 361.386–361.684 |
| Complete revalidation | 334.937–339.191 | 334.596–335.250 |

These are synthetic correctness/deadline controls, not product throughput,
latency percentiles or a paired comparison with another candidate. The earlier
failed separate composition cannot supply a successful before/after pair.

Fourteen persistent-drift/expiry cases pass per compiler, 28 in total. They cover
trust mutation at all five checkpoints, bundle content/inode replacement, output
writes, projection/state/writer/message/timer mutation and actual original
progress expiry without clock adjustment. Each case checks the precise refusal
family, sticky refusal, unchanged retained bytes and fence retention through drop.
A late time error is not accepted as the requested earlier binding fault.

The corrected origin cohort passes on both compilers. Each exercises the actual
production constructor against the absent fixed install, then independently
constructs the fixture owner and requests production revalidation. Both must
refuse without fallback, latch the entire original chain, retain unchanged
records and the fence until drop. There are four successful expected refusals,
not four production acceptances. Origin controls use only the corrected binaries;
all healthy/timing/refusal-matrix results above retain their original identities.

## Retained negative results and cohort separation

build-initial-194.log and build-initial-188.log retain an initial incorrect Rust
field access in the fixed loader; it is corrected to the original checked policy.
static-initial.log retains the first exact-reference failure caused by rustfmt
wrapping a longer error type. A same-length error name preserves the original
source comparison without whitespace normalization or changed predicates.

checks-overlap-194.log and checks-overlap-188.log retain exit-127 shell failures
after a completion marker: a script was edited while sh was reading it. Clean
checks-baseline files rerun the immutable script to exit zero. CompilerChecks in
the verifier rejects both trailing shell errors and compiler errors even when
a completion marker exists. The original binary cohort uses final-build logs;
the corrected origin cohort is built from the final checks logs.

origin-initial-194.log and host-origin-initial-194.log retain the actual first
origin failure. The missing fixed install is correctly refused and all inner
reason/sticky/record/fence checks pass. Then with_original_at correctly observes
the refused original chain, contradicting the test's mistaken refused=false
argument. Execution stops before the next cohort. The correction changes only
the two origin-test wrapper arguments to true: all original helper assertions,
inner checks and production bodies stay exact. origin-harness-fix.patch records
that delta from 7ddcd6f3, separately from implementation.patch at the old baseline.
The negative is not reclassified as a successful test.

## Reproduction and boundaries

checks.sh selects Rust 1.94 or 1.88, runs four new release/debug alarm tests,
four unchanged reference units, six trace units, 43 focused integration tests,
eight new and five old compile-fail boundaries, all-target check and strict
all-feature Clippy. Windows supervisor/xtask checks and Clippy, all 198 static
contracts, the 184-discovered-gate registry, format, doc-check and local
non-promotable performance contract pass. This is not the full milestone verify
or require-ship gate.

Run verify.ps1 -SelfTest for the log oracle and verify.ps1 for exact packet
inventory, Git source/capsule provenance, raw/derived rows, binary/script identities,
refusal order, source-cohort separation and guards. verify-prior.ps1 runs the
unchanged previous process packet in its exact archived source view with an
owned index copy and verifies that the active index did not change.
The verifier accepts four real control families and rejects 13 altered or retained
negative inputs, including the actual completion-marker-before-shell-error log.

All hosted fixtures use only uid/gid 1000 in
/tmp/hc074-fixed-grouped-20261017.v5u5LH. Guard scripts pin boot, active supervisor
PID 8839, zero restarts, installed binary/receipt hashes and absence of campaign
or lifecycle contexts. The nine signed fixture files are nonexecuted data.
Independent postflight checks executable objects, not process command strings.
The qualification manifest remains unchanged at raw SHA256
11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.

No release install, supervisor stop/restart, root mutation, product workload,
pilot, expensive CI or qualification is performed. This new owner's own real
SIGKILL/return-ACK matrix, autonomous guardian, authenticated original send/start,
P2/P3 all-writer cessation, completion and later cells remain separate obligations.
The old grouped process-loss packet does not certify this new production type.
