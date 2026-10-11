# Original fixed Reserved state local evidence

This package inspects an existing fixed state file under exclusively borrowed
original intent storage. It creates nothing, acquires no lock and does not join
authority/intent, enroll a host or authorize execution. See the
[contract](../../diagnostic-reserved-state-local-contract.toml) and
[design](../../diagnostic-reserved-state-local-design.md).

The existing storage unit baseline passed. The missing public API test failed
before implementation. Ten initial unit cases passed; subsequent assertions
expanded private kind/component, read-budget and descriptor error coverage.
Early repeats/regression are retained as that intermediate test coverage, not
the final repeated-source proof. Final literal-filename repeats and regression
cover the expanded tests.
The expanded private test initially shadowed the `parent` helper with a metadata
variable; Linux all-target compilation caught it. The raw compiler failure is
retained and the test-local variable was renamed. No product or guard predicate
was relaxed.

All positive paths use the actual private walker on nonroot temporary modeled
ancestry. Owner/GID/device-negative stamp checks are synthetic, not privileged
chown/mount exercises. FIFO opens are nonblocking and metadata refuses before
read. Enumeration-based state mutations use a logged fixed value, not random
chaos or an exhaustive property proof. Independent concurrent storage roots do
not prove common-host locking. There is no crash or power-loss test here.

The manifest binds closed raw captures and LF-normalized source hashes to the
implementation commit/tree. Historical authority/intent evidence is verified
against its own Git source rather than rewritten for current tests/registry.
Frozen qualification inputs, plan-digest refusal and closed admission remain
unchanged. Tests are scoped overlapping runs, not unique release-wide counts or
performance measurements.

Baseline is `61308bc927f930f8641b1275cf3e75591a7ccb17`; preregistration is
`fd48898304543424fe53c4a414dd702466d3b60e` and implementation is
`6afedec92d29a5bd131dd015127a95a168362af4`. Ten final unit cases pass all three
separate repeats. Final selected Linux regression passes 476 with one existing
ignored; portable Windows passes 196. Root checks pass 111 performance contracts,
13 evidence, six gated-target and 23 governance cases. Fifteen compile-fail
doctests, strict all-target/all-feature clippy, current checks, MSRV 1.88,
formatting, rustdoc with denied warnings and local contracts pass.

Thirty-one closed captures retain the initial missing API, test compilation
failure, intermediate runs, final repetitions/regression, documentation checks
and genuine qualification/ship refusals. Source hashes normalize CRLF to LF; log
hashes preserve exact bytes. The verifier checks historical source and all eight
frozen qualification inputs as well as the current packet.

```text
python docs/testing/performance/0.74/local-runs/diagnostic-reserved-state-20261011/verify-packet.py
```

Next are the common supervisor-owned original host fence and trusted state
publisher, then authority/intent/state composition, fixed consumption publication
and production output/start/watchdog/owned cleanup with real issuer/host enrollment.
The legacy worker-owned state must not be copied into the root namespace.
