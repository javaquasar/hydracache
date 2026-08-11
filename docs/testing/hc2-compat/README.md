# HC/2 retained compatibility evidence

This directory contains immutable client artifacts and the machine-readable
compatibility matrix introduced by H18. It is deliberately separate from the
generated source tree.

The first baseline, `h17-preview-d1d1d44`, was produced from exact commit
`d1d1d44cf5c046b8bad97292b9a9a97210fda134`. The manifest binds every artifact
to its byte length, SHA-256 digest, Git commit, tree, and contract blob.

Run:

```text
cargo xtask client-plane-compat-check
cargo xtask client-plane-compat-check --manifest-only
cargo xtask client-plane-compat-check --require-complete
```

The normal command verifies all retained bytes and runs executable baseline
smoke tests against both a separately built mTLS conformance peer and the
current production `hydracache-server`. The retained Rust crate and Java JAR
each perform PUT/GET through the production listener, after which the harness
drains the daemon and requires a successful zero-resource exit. It succeeds
while printing every row that is still blocked. `--require-complete` is the
release-grade fail-closed mode: it fails until every row is a genuine
cross-version `pass`.

`baseline-smoke` is not a compatibility pass. The H17 artifact and current
tree still use the same protocol contract. This label prevents self-comparison
from becoming a release claim. A row may become `pass` only after distinct old
and new production artifacts exist and the exact retained binaries complete
that scenario.

Do not replace an artifact in place. Add a new versioned directory and manifest
record. Any byte change under an existing record fails the gate.
