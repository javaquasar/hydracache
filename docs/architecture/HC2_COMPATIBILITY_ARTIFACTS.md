# HC/2 retained-artifact compatibility policy

## Decision

HC/2 compatibility claims must be backed by retained, checksummed binaries.
Generated source comparisons and tests that compile both sides from the current
checkout remain useful schema checks, but they are not old/new compatibility
evidence.

H18 establishes the first immutable preview baseline. It intentionally records
two different outcomes:

- `baseline-smoke` means a retained H17 client artifact successfully consumes
  the current conformance peer, while the contract blob is still identical;
- `blocked` means the required production artifact or lifecycle does not exist.

There is no `skip` status. Unsupported and unavailable scenarios are explicit
blocked rows with actionable dependencies. The release-grade
`--require-complete` invocation rejects both statuses.

## Retained baseline

The `h17-preview-d1d1d44` baseline retains:

- the publishable Rust `.crate`, including its normalized manifest, lockfile,
  process test, contract, and Cargo VCS receipt;
- the Java SDK JAR and its published POM;
- SHA-256, byte length, producer commit/tree, and exact protobuf Git blob.

The gate extracts the saved Rust package and runs its own locked test suite
against a separately built mTLS peer. A standalone consumer also compiles
against the extracted package and proves that its generated decoder accepts an
additive unknown field. The saved Java JAR/POM are installed into an isolated
Maven repository; the external consumer then proves connection, capability
handshake, operations, metadata, and Java unknown-field preservation without
resolving the SDK from the current build.

## Matrix semantics

The required scenario set is fixed in the gate:

1. retained Rust client to current peer;
2. retained Java client to current peer;
3. old client to new production daemon;
4. current client to old production daemon;
5. concurrent HC/1 and HC/2 listeners;
6. rolling production upgrade;
7. capability negotiation;
8. additive unknown fields;
9. planned deprecation.

Missing rows, duplicate IDs, unknown statuses, unbound artifacts, changed bytes,
or a `pass` against the identical contract blob fail closed. Production rows
remain blocked until H01/H02 provide daemon listeners and a later preview gives
H18 a genuinely older server/client generation.

## Reproduction

```text
cargo xtask client-plane-compat-check --manifest-only
cargo xtask client-plane-compat-check
```

The first command is offline structural evidence. The second additionally runs
the retained consumers. Before promoting a release compatibility claim, run:

```text
cargo xtask client-plane-compat-check --require-complete
```

That command is expected to fail at this stage. Its failure is evidence that no
production old/new or rolling-upgrade claim has been smuggled through the
preview baseline.
