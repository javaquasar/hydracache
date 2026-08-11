# HC/2 retained-artifact compatibility policy

## Decision

HC/2 compatibility claims must be backed by retained, checksummed binaries.
Generated source comparisons and tests that compile both sides from the current
checkout remain useful schema checks, but they are not old/new compatibility
evidence.

H18 establishes an immutable generation-5 client baseline and retained
generation-5/generation-6 production daemons. The manifest permits three
outcomes for future evolution:

- `baseline-smoke` means a retained H17 client artifact successfully consumes
  the current conformance peer or production daemon, while the contract blob
  is still identical;
- `blocked` means the required production artifact or lifecycle does not exist.
- `pass` means distinct contract generations completed the named executable
  scenario with all required artifact identities verified.

There is no `skip` status. Unsupported and unavailable scenarios are explicit
blocked rows with actionable dependencies. The release-grade
`--require-complete` invocation rejects both non-pass statuses. H18 currently
has nine passes and no incomplete rows.

## Retained baseline

The retained set contains:

- the publishable Rust `.crate`, including its normalized manifest, lockfile,
  process test, contract, and Cargo VCS receipt;
- the Java SDK JAR and its published POM;
- Linux and Windows production-daemon archives for protocol generations 5 and
  6;
- SHA-256, byte length, producer commit/tree, exact protobuf Git blob,
  platform, inner executable digest, and inner executable byte length.

The gate extracts the saved Rust package and runs its own locked test suite
against a separately built mTLS peer. A standalone consumer also compiles
against the extracted package and proves that its generated decoder accepts an
additive unknown field. That consumer then uses the retained public API to
perform PUT/GET against the current production daemon and requires bounded
admin drain plus successful process exit. The saved Java JAR/POM are installed
into an isolated Maven repository; the external consumer proves connection,
capability handshake, operations, metadata, Java unknown-field preservation,
and the same production-daemon lifecycle without resolving the SDK from the
current build. Archive validation opens each daemon tarball and checks the
internal receipt and executable, rather than trusting only the outer digest.

Generation 6 keeps generation 5 inside an explicit compatibility window. The
handshake advertises minimum 5 and preferred 6, and marks a generation-5
selection deprecated. Legacy generation-5 daemons omit those additive fields;
current clients apply the documented generation-5 fallback without weakening
the exact selected-generation check.

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

Missing rows, duplicate IDs, unknown statuses, unbound artifacts, changed
bytes, or a `pass` against the identical contract blob fail closed. The
generation-5 retained clients run against generation 6; current Rust and Java
clients configured for generation 5 run against the retained generation-5
daemon. The rolling row runs retained generation-5 and generation-6 binaries
at once and proves explicit reconnect, connection-generation advancement,
post-replacement invocation, and clean drain. It does not claim data migration
between those independent single-node fixtures. The HC/1+HC/2 row separately
proves shared-dispatch visibility while both production listeners are active.

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

That command must report `pass=9, baseline-smoke=0, blocked=0`. A failure means
artifact identity, an executable cross-version scenario, or the declared
matrix has drifted and release admission must stop.
