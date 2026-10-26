# Exact historical view of the approved roots lock

This test-source-only adapter accepts exactly the approved current Cargo.lock
from `63cb4d7b9bca1c901b2f7420f86effd3309a6882`. Six compiled inverse edits
restore the complete accepted pre-roots body at
`ba9c5535311ffd0e6759506898077dbc6a56830a`. Whole canonical current and previous
SHA256 checks bracket those edits. Other paths remain byte-identical; stale,
partial, duplicated or unrelated known-path inputs panic before returning text.

The current product lock remains 1.0.10. This view is not compiled into the cache
or supervisor runtime, does not write files and does not admit an older trust
catalog. It precedes the unchanged diagnostic/security views only in source
preservation tests. Existing fresh IO, old rule descriptors, read order and
live validation frequency remain unchanged. Only immutable compiled rule data
are cached, never a current file observation.

The seven old security controls retain every assertion. Their raw helper now
validates each current observation through this boundary before comparing its
exact former producer body. The entry oracle accounts for the single import/
pure-call insertion and rustfmt declaration order; it does not waive source
equality. Seven additional controls cover independent fixed Git producers,
whole predecessors, stale/partial/duplicate mutations, unknown path behavior,
absence of observation reuse, unchanged old helpers and a fixed-seed 24-case
single-byte corpus.

```text
cargo +1.94.0 test --offline --locked -p xtask --test roots_lock_view_074 --test security_input_view_074 --test source_successor_view_074 --test historical_input_view_074
```

The [result report](../../docs/testing/performance/0.74/roots-lock-view-package-v8-result-20261027.md)
retains the baseline and first test defects separately from corrected checks,
fresh V8 source/effect review and the unchanged complete package gate. It grants
no new performance, original-execution, server or qualification authority.
