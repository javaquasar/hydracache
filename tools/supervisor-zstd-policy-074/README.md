# Private supervisor Zstd review

This closes the direct unpublished supervisor edge only. Root Chitchat, Q
adoption, full package/workspace, production execution and server admission stay
open. See the [result](../../docs/testing/performance/0.74/supervisor-zstd-security-result-20261030.md).

Offline data verification does not run archived Python or Rust:

```powershell
python -B tools/supervisor-zstd-policy-074/verify.py
python -B -m unittest discover -s tools/supervisor-zstd-policy-074 -p test_*.py -v
python -B tools/supervisor-zstd-security-074/test_review.py
```

The exact historical lock is recovered from its retained content object, or
the fixed instrumentation commit `edf39086d241c461c39b7e0915bb31cea5d03f86`.
It does not depend on preserving `target`. `graph.review` accepts only the whole
exact byte transformation, retaining every unrelated block's order/content.
Tests reject unrelated byte/edge changes, changed historical input, corrupted
stored bodies and coherently rehashed promotions of Q/workspace/release claims.

Live phases in `../supervisor-zstd-security-074/run.py` are intentionally fixed
and write-once in `target/supervisor-zstd-security-20261030`. Recorded phases
cannot be rerun in place. A fresh live context needs a separate reviewed capture;
do not delete refusals or widen any envelope. The actual Root archive matrix is
four tests × Windows/GNU × Rust1.94/1.88. The separate six-payload wire fixture
has newer helper dependencies and is explicitly not exact-Root wire evidence.

`run.py` here performs two advisory-only scopes against a separately frozen
RustSec tree, with no new ignores and only a storage-location policy difference.
`format.py` is the targeted Windows formatter after the retained full-workspace
OS206 refusal. `publication.py` computes actual public-root resolve reachability;
it does not run registry consumers. `gates.py` owns fixed final offline commands
with the original CI helpers and1200-second bound; the unchanged xtask image is
pinned to the preceding sealed r3 verification. Non-promotable performance
contract success is not a failed security/workspace stage bypass.

The primary packet exports sanitized intent settings rather than inherited
environment values. Original private/exported hashes remain explicit.
`retain_verification.py` records final checks in a separate immutable packet.
