# Initial fixed intent consumption: local evidence

The local library now retains cell-one intent under the runtime-acknowledged
Reserved publisher's original common fence. No server operation, product workload,
worker execution, qualification or ship admission was performed or enabled.

Baseline: `175d2dae1509d1b664efe0852074665cc99b82b7`.
Preregistration: `84cb01d6`.
Implementation: `9e6a781cffba39ec788c3f43343dea1cfe13c445`.
Implementation tree: `2946b3f46356ebe9884addea886be2d4af66548e`.

## Observations

Baseline passes all 15 existing reservation-publication cases. The absent-entry
integration test fails with E0432 before implementation. The final 12 new cases
pass in each of three repeats with four test threads and checkpoint seed
`0x074c`. They cover all 14 logical publication checkpoints, canonical sealed
record/digest bytes, unchanged Reserved state, original lock retention, late
targets and legacy/unknown context, metadata/link/substitution drift, signed
intent expiry/peer/signature/worker refusal, original authority safety, sticky
no-I/O refusal, same-guard replay and independent concurrent chains.

Scoped Linux regression passes **596** cases with **one existing ignored real
systemd-bus test**. Linux compile-fail doctests pass **38** cases. Native Windows
passes **196** portable cases; Linux-gated targets report zero there and are
not Windows coverage. Root contracts/registry/governance pass **160** cases.
Current Rust 1.94.0 and MSRV 1.88.0 all-target/all-feature checks, warnings-denied
Clippy, scoped Rust formatting, warnings-denied native Rustdoc, doc-check and
mdbook pass. Functional test durations are not performance measurements.

The expanded red log retains an overly narrow legacy-context error expectation:
both context paths refuse, but through different fence error classes. The test
now checks their correct rejection semantics. The red Clippy log retains a
test-only manual range-pattern warning, corrected without suppression. Captures
03–07 are provisional observations, not substitutes for the final repeats.

## Scope and reproducibility

The [manifest](manifest.json) binds raw capture bytes and commands, final source
and tree, preregistration ancestry, contract pins, named tests and the untouched
qualification inputs. Git source hashes use canonical repository bytes; raw
logs are never newline-normalized. Run the read-only verifier from this checkout:

```powershell
C:/Python313/python.exe docs/testing/performance/0.74/local-runs/diagnostic-initial-consumption-20261012/verify-packet.py
```

The existing record encoder, layout and digest marker are unchanged except for
crate-private visibility needed for reuse. Existing state and journal records
are never adopted, overwritten, repaired or removed. Files from an interrupted
attempt remain admission-blocking context after the owned lock drops. A surviving
valid pair is not a recreated runtime acknowledgement.

Only initial cell one is implemented. The original state is not advanced.
Logical checkpoint exceptions are not actual SIGKILL, power-loss or positive
root-host durability evidence. Next test actual owned-process interruptions,
then preregister later cells/state transitions. Authenticated peer/time, reviewed
trusted ownership/binary rollout and original output/start/watchdog/owned cleanup
still precede activation. See the [contract](../../diagnostic-initial-consumption-local-design.md).

The frozen qualification manifest remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
All eight inputs remain unchanged. Dry-run still refuses the inherited plan
digest mismatch, and ship admission stays closed. No threshold was adjusted.
