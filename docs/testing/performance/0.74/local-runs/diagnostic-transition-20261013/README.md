# Local diagnostic state transition evidence

The first P1 slice adds adjacent-state checks to the local model publisher.
Implementation is `4085e4e9b587fb5d99e0ed271e7ec2ad251bf0f5`, tree
`0f9fd3cf6a70556b5c428cebc7985d6454414a0e`; preregistration is
`73d767a87949748f397b5844b6347b9045043dd1`. The starting source is
`cd01ddbee8f0ba405869a639c78bc16eb5852d37`.
See the [transition design](../../diagnostic-transition-local-design.md).

## Scope

The validator checks all five stages and four cells, exact next revision,
immutable identity/reservation origin, monotonic assertions, controller refresh,
original start/inode, sticky stop reason and one-cell completion. Invalid or
stale candidates refuse before pending/state publication. Terminal cannot start
a new transition. Validation has no I/O, spawn or authority conversion.

The model publisher uses its existing transaction fence and existing schema-1
state bytes. The fixed publisher/consumer and their original bindings are
unchanged. This does not implement cells two through four in the fixed path,
authenticated time/peer, early durable nonce admission, production execution,
host enrollment, cgroup cleanup or qualification.

## Retained negative observations

`02-red-missing-validator.log` is the expected missing-function compile failure.
`03-first-focused.log` retains a faulty graph fixture: Reserved cancellation
incorrectly contained a process start/inode. It was corrected without relaxing
the validator. `11-root-regression.log` retains an invocation failure from the
nonexistent target `gated_proof_registry`; `12-root-corrected.log` uses the
actual `gated_tests` target. These are not successful proof runs.

## Verification

The manifest binds raw logs, source bytes and the exact implementation tree.
The read-only verifier checks repeated named tests, generated-history totals,
the W11 registry, closed permissions and unchanged frozen qualification inputs:

```text
C:/Python313/python.exe docs/testing/performance/0.74/local-runs/diagnostic-transition-20261013/verify-packet.py
```

Three exact-source Linux repeats use seed `0x074e`. Each exercises 25 stage
pairs, 64 legal four-cell histories, 1280 legal transitions and 2560 invalid
mutations, alongside publisher refusal tests. These counts describe functional
coverage, not throughput, latency, allocations or production readiness.

Final scoped Linux regression passes 607 tests with four existing manual cases
ignored; native Windows passes 204 and root contracts/registry/governance pass
162. Linux compile-fail doctests pass 38. Rust 1.94 and MSRV 1.88 all-target
checks, warnings-denied Clippy, format, Rustdoc and documentation checks pass.
The qualification dry run and ship contract still refuse as expected. This is
not a full workspace release gate, CI receipt or installed-host proof.

The old [P0 packet](../diagnostic-process-crash-20261012/README.md) remains
historical process-loss proof. This slice introduces no new write boundary or
durable format, and does not reinterpret surviving history as permission to
execute. Early attempt admission and an owned fixed progression protocol are next.
