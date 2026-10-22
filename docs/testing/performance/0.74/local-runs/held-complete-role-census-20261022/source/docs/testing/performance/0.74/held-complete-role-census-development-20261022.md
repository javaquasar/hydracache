# Complete role census: first component failure

The first post-implementation GNU 1.94 release component command selected
`held_complete_census` (all features, library only, one test thread). It returned
101: one model test passed and seven component tests failed at fixture census
construction (`diagnostic_held_role_census_tests.rs:81`). This was not an original
attempt, measurement, service start, or qualification retry. The result is retained
as a failed development candidate, not replaced by subsequent corrected checks.

The added constructor had incorrectly required distinct `(dev, ino)` for the two
guardian pidfds. A separate local, owned-child diagnostic in Ubuntu observed:

```text
pid 312 fd 3 dev 15 ino 39; fdinfo Pid: 312
pid 313 fd 4 dev 15 ino 39; fdinfo Pid: 313
```

Thus inode inequality is not a valid task-identity requirement on this kernel.
The correction removes only that new unsupported assumption. Distinct ordered
PIDs, original expected metadata, distinct retained FD numbers, fresh procfs
`fdinfo` PID, nonterminal poll, unchanged stamps, consumed/Armed state and all
pre-existing validation frames and deadlines remain required. The diagnostic
killed and waited only its two synthetic `/bin/sleep` children.

Two source-contract development assertions also needed correction: one assumed
single-line Rust formatting, and one duplicate-hook mutation did not duplicate
the whole exact hook. Neither changed a runtime predicate or qualification.
No complete original-owner, delivery or all-writer cessation gate has passed here.

The first documentation driver also used an unregistered xtask spelling,
`gated-test-registry-check`. Doc-check passed; registry execution returned 1
before contract/diff execution. Its raw commands, streams and failed lane remain
in `checks/docs`. The versioned `docs-v2` driver selects the registered
`gated-test-check`, protected by a new offline regression. This is a tooling
correction, not a repeated live or measured attempt; the failed lane is not
overwritten. The original six synthetic tool controls and final seven-control
suite remain separate receipts.
