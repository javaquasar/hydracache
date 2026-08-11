# HC/2 retained compatibility evidence

This directory contains immutable client artifacts and the machine-readable
compatibility matrix introduced by H18. It is deliberately separate from the
generated source tree.

The client baseline, `h17-preview-d1d1d44`, was produced from exact commit
`d1d1d44cf5c046b8bad97292b9a9a97210fda134`. H18 adds production-daemon
generation 5 from commit `539d74e7b5e01a555f3ebbe01d7820eb1df7fae1`
and generation 6 from commit `00508c688618658471785c9d8e73dcfa020ba39e`.
The manifest binds every artifact to its byte length, SHA-256 digest, Git
commit, tree, contract blob, platform, and executable identity.

Run:

```text
cargo xtask client-plane-compat-check
cargo xtask client-plane-compat-check --manifest-only
cargo xtask client-plane-compat-check --require-complete
```

The normal command verifies all retained bytes and runs the complete
cross-version matrix. Retained generation-5 Rust and Java clients execute
against the generation-6 conformance peer and checksummed generation-6 daemon;
current clients select generation 5 and execute against the checksummed
generation-5 daemon. The rolling row starts both retained daemons, connects to
generation 5, replaces the connection with generation 6, verifies generation
fencing and post-replacement operations, then drains both processes. A second
Java run verifies the same generation-5 deprecation policy advertised by the
generation-6 daemon. The HC/1+HC/2 row runs the real shared-dispatch daemon
process test.

`--require-complete` is the release-grade fail-closed mode. The current matrix
is 9/9 `pass`; any future `baseline-smoke`, `blocked`, missing row, digest drift,
identity drift, or same-contract self-comparison fails it.

The rolling claim is deliberately narrow: it proves wire compatibility,
connection replacement, generation fencing, and clean lifecycle. The two
fixtures are independent single-node daemons, so H18 does not claim replicated
state migration or cluster rebalance correctness.

Do not replace an artifact in place. Add a new versioned directory and manifest
record. Any byte change under an existing record fails the gate.

Production daemons are retained by manually dispatching
`.github/workflows/hc2-client-plane.yml` with
`retain_hc2_daemon_commit=<exact-lowercase-40-hex-SHA>`. The opt-in job builds
the real `hydracache-server` release binary on `ubuntu-24.04` and
`windows-2025`, refuses checkout drift, records the producer commit/tree and
HC/2 contract blob, and publishes checksummed archives for 90 days. This job is
artifact production only: it does not replace or weaken any of the four H22
release-admission lanes.

The checked-in archives came from successful artifact-production jobs:

- generation 5: run `31499408733`, Linux artifact `9104469005`, Windows
  artifact `9104590034`;
- generation 6: run `31501497494`, Linux artifact `9105308468`, Windows
  artifact `9105463940`.

The Git-tracked copies are the compatibility inputs. Remote workflow retention
is a provenance aid, not a runtime dependency of the gate.
