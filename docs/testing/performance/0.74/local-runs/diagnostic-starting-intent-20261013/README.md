# Owned initial Starting intent local evidence

This P1 packet proves one owned durable departure intent, not worker execution.
Baseline: `444ab967da19907261ade8c2fbd1d33e821a3869`.
Preregistration: `a4a6d6e76b5346238d1eee89d0919eb64e5d6657`.
Runtime implementation: `07008eb43eb6606c7726d7d27799e995d2cddf6b`;
tree: `5565559900294a98e4ed20f19f7dfb0cbbf3f909`.
Final proof source: `099bb14da0d17069913287a528ca2efc5a05caee`;
tree: `0d736e15f86ac2b443f96493ce4dc0b9f295dbe3`.
The latter changes only two existing transition test fixtures to hold one
original fence; runtime and workload/proof constants are unchanged.

The [design](../../diagnostic-starting-intent-local-design.md) and
[contract](../../diagnostic-starting-intent-local-contract.toml) require the
acknowledged original attempt, initial state and cell-one record/commit.
A consuming API exclusively retains a canonical revision-two Starting record,
then seals/syncs and revalidates original objects/authority before acknowledgement.
The original active state remains immutable. Hash links are not new controller
signatures, current executing state or permission to start/complete a cell.

## Evidence sequence

The initial filters passed 26 publication/consumption tests with one manual
case ignored; the literal transition filter matched no graph test. Earlier
transition evidence remains separate. The new missing-entry test failed with
E0432 before implementation. Development compiler negatives include E0277
generic I/O conversion and E0433 missing test-only authority-stage alias.
The first root contract guard failed on formatter whitespace around the sync
call; its correction ignores whitespace while preserving the required call.
All original logs are retained, not replaced by corrected runs.

Eight initial functional cases passed, then fourteen expanded cases and sixteen
final cases. The final-source three repeats each pass all sixteen named cases.
Seed `0x074b` checks 128 legal departures and 128 rejected graph mutations per
repeat. These are correctness observations, not performance measurements.

The initial owned-process proof passed nine SIGKILL cases and one control.
An overloaded concurrent repeat batch retained two 60-second readiness failures;
those attempts are not complete crash matrices. A separate repeat completed.
The full initial seven-phase regression also completed. Final process repeats
use single-flight scheduling with unchanged source, seed and readiness timeout.
Every successful case requires original lock release, exact helper termination,
unchanged retained bytes/metadata and two fresh read-only processes. BeforeCreate
already has attempt/state/cell-one context and fresh reservation must refuse.

The initial Linux functional regression retained a transient `Busy` when an old
transition fixture released and immediately reacquired flock. The fixture
correction uses `reserve_fenced` under the same original fence throughout;
invalid/stale-candidate assertions and product lock behavior are unchanged.
Its eight graph tests pass on Linux and Windows, with check/lint/format green.

## Final proof results

At `099bb14d`, each of three final-source functional repeats passes 16 tests;
each process repeat passes nine SIGKILL cases and one uninterrupted control,
with twenty fresh read-only inspections. The final seven-phase process
regression passes 66 SIGKILL cases and six controls, with 144 fresh inspections.
Scoped Linux functional regression passes 660 with eight manual/host tests
ignored; those seven process proofs are run separately. Full Windows regression
passes 204, root release contracts 166, and Linux compile-fail doctests 48.
Current/MSRV all-target/all-feature checks, warnings-denied lint/Rustdoc,
formatting and docs/book checks pass. These are local correctness/refusal
results, not numerical product measurements, power-loss or installed-host proof.

## Frozen qualification and closed boundaries

Qualification dry-run and ship admission refuse with exit 1 and no requested
qualification output is created. Raw qualification manifest hash remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`;
all eight inputs remain unchanged. The inherited plan mismatch is retained:
actual LF hash
`cff351817e9d83c3ec72b587e4ad7580bb8b882c8fab154f88c31b63aebb9828`,
declared
`6b443b0c766de5dcd131bdf7dcf46e6e8d256063f1fab1685034ae86427aa44d`.
No digest refresh, threshold change, product candidate or host workload occurs.

Run the read-only verifier from the repository root:

```powershell
C:/Python313/python.exe docs/testing/performance/0.74/local-runs/diagnostic-starting-intent-20261013/verify_packet.py
```

It checks exact Git source/trees and source hashes, byte-preserved command logs,
named tests/seeds, crash/control inventories, raw historical preservation,
registered manual gate and unchanged qualification inputs. Unexpected failures
retain their intended exit 0 and observed exit 101, separately from green gates.

No Starting-to-Running conversion, real worker launch, authentic cell completion/
cgroup cleanup, next-cell intent, recovered acknowledgement, production peer/
clock/issuer enrollment, host installation, physical power loss or reboot is
proved. P1 remains open; P2 through P5, qualification and ship remain closed.
