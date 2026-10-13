# Local signed history inspection evidence

This packet covers supplied-byte inspection only, not fixed lifecycle or
execution. Baseline: `8549010a649a4a7394071a3dbaff67c794fc6a31`.
Preregistration: `718707058958b8f0ce81e0d6716f9965f92b9708`.
Implementation: `2d446ec5b8a11b0ecd64ec813faaddaa91c9afa7`;
tree: `ce2adcc6ae395089bcb2214cca46cf2bec9c7b3a`.

The [design](../../diagnostic-signed-history-local-design.md) and
[contract](../../diagnostic-signed-history-local-contract.toml) distinguish
signatures and necessary Reserved-projection bounds from authenticated state
transitions and cell completion. The outer predecessor hash is not part of the
intent signature. No read result can append, spawn or recreate acknowledgement.

## Results

Baseline journal: 19 passed. The missing API failed compilation with E0433
before implementation; that raw negative is retained. An initial nine-test
run passed. Two additional expiry/concurrent-inspection cases were then added.
At the final source, all 11 cases pass in each of three repeats; seed
`0x074f` checks 64 legal chains and 64 fully signed mutations per repeat.
This is a seeded property matrix, not a numerical performance measurement.

Final scoped Linux regression: 644 passed, seven ignored manual/host cases.
Windows: 204 passed. Root release contracts: 165 passed.
Linux compile-fail doctests: 44 passed. Formatting, current/MSRV all-target
checks, warnings-denied Clippy/Rustdoc and docs/book checks pass.
The initial Linux regression ran before the two additional cases and passed
642; it remains separate from the final-source run.

The first packet check detected that the scoped Windows command covers only
70 tests, not the previous full-package total of 204. Its reproduced negative
is retained separately. The complete Windows package was then run, and the
verifier now checks the two scopes independently; no test or threshold was
relaxed. A separate verifier failure used the wrong registry field name
`artifacts`; it now checks the actual `required_artifacts` field without
changing the registry contract. The 33 raw logs retain both verifier negatives,
the missing-entry failure and the two Windows scopes.

Final documentation checks passed. A concurrent final Windows Cargo test
could not replace `target/debug/xtask.exe` while doc-check was using it
(OS error 5). The exact returned command output is retained in
`windows-build-lock-negative.log`, identified as tool-returned output rather
than a redirected raw stream. The release-evidence tests were rerun serially
after doc-check exited; their redirected output is `final-release-evidence.log`.
This is a Windows build scheduling failure, not a product or contract failure.
These two supplemental captures are listed separately from the 33 primary logs.

Qualification dry-run and ship admission refuse with exit 1. The former
creates no requested output artifact. The inherited main-plan digest mismatch
is retained, not refreshed: actual LF hash
`cff351817e9d83c3ec72b587e4ad7580bb8b882c8fab154f88c31b63aebb9828`
differs from declared
`6b443b0c766de5dcd131bdf7dcf46e6e8d256063f1fab1685034ae86427aa44d`.
The qualification manifest and its eight input files are unchanged.

## Verification and limits

Run from the repository root:

```powershell
C:/Python313/python.exe docs/testing/performance/0.74/local-runs/diagnostic-signed-history-20261013/verify_packet.py
```

The verifier is read-only: it checks raw log hashes/inventory, exact source/tree
and LF source hashes, named repeated tests/seed totals, aggregate regressions,
contract boundaries, prior packet preservation and frozen qualification inputs.
Commands/platforms and expected/observed exits are in `manifest.json`.

No new process-loss test is claimed: this API has no writes, filesystem opens,
spawn or owned-storage transition. Existing crash harness, fixed publishers and
server remain unchanged. No machine reboot, power loss, positive installed-root
enrollment, real peer/clock or successful product workload is established.
Fixed cells two through four, owned progression, incomplete/reboot handling
and P2 through P5 remain open; no performance, qualification or ship permission.
