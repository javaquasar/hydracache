# Pre-capture test-authoring refusal

The additional suffix-extraction assertion first failed in native
`cargo test --locked -p xtask --test authority_inventory_074 -- --test-threads=1`:
one passed, one failed. The PowerShell enclosing command returned 1; Cargo
reported its ordinary test failure. It selected the first `if !` (the observed
insertion guard), not the expected-comparison suffix. No measurement had run.

Anchor extraction after the generation assignment, then require the exact
expected-comparison prefix. Keep the stronger frozen reference/proposal
assertions. Neither runtime nor frozen workload changes. A separate
pre-capture receipt records focused/check/clippy on this final canary; earlier
Before xtask logs remain unchanged. This is a test-authoring refusal, not a
runtime correctness or performance result.
