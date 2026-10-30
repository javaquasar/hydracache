# Development resolved inventory and unsigned provenance

`cargo xtask imap-sbom --release 0.75` runs the fixed managed preparation build and
generates `target/imap-075/development-inventory.json`, a pointer copy of the complete
provenance document in its fresh build directory. That directory also contains
`inventory.json`, `bom.cdx.json`, `capture.json`, actual compiler JSON streams,
Maven resolution journals and the separate dependency-policy report. The preparation
v6 receipt retains a fixed `development-provenance.json` reference. Replay:

```text
cargo xtask imap-sbom-check --release 0.75 --receipt target/imap-075/development-inventory.json
```

Replay requires clean current source, the current producer, unchanged actual target
artifacts, compiler/Maven/JDK input hashes and the same resolved closure. Receipt
fields never select commands. CycloneDX 1.6 is validated against the pinned offline
official schema and its two registered references. Inventory is sorted and contains
no build timestamp, random consumer path, host directory or serial number; per-run
paths/nonces and raw-input hashes live in provenance separately.

Five targets are bound to actual bytes: current server, standalone Rust consumer,
reference soak worker, contract JAR and native JAR. Rust compiler instances retain
their actual features, target kind, build/test profile and invocation. The full
locked Cargo metadata graph is host-filtered; edges retain normal/build/dev and
target predicates for packages actually compiled in that invocation. This describes
the native-host development build/test input closure, including build feature
unions; it does not claim exact per-instance production runtime linkage. Registry
checksums and Git/fork revisions are retained. The product lock is never updated.
Optional edges are activated from actual compiler features; dev/build edges require
the corresponding test/build-script context. Target predicates are independently
evaluated against actual `rustc --print cfg`: Cargo metadata can retain alternative
predicates such as `cfg(any())` even in a host-filtered graph. Rust compiler-driver,
stdlib and distribution library inputs are hashed in addition to the launchers.

Maven builds use the fixed credential-free settings and a workspace-owned repository.
An EventSpy compiled against the installed Maven captures actual resolver inputs,
including dynamically loaded Surefire providers, plugin dependencies, POMs and BOMs.
Its explicit successful terminal is mandatory because Maven can swallow spy errors.
Runtime/compile/test/provided transitive relationships come from an unfiltered
dependency-plugin 3.8.1 JSON tree. Plugin/test resolver material is separately marked
as build input, not asserted to be runtime linkage. JDK module image/launchers and
Maven distribution JARs are hashed. OS/native-loader/system-DLL inventory is outside
this profile. Only selected public POM fields are parsed, with DTD/external entity
resolution disabled; no settings credentials, environment or system properties dump.

License declarations retain their direct metadata origin. Missing/inherited-only
assertions remain `unknown`, never inferred as approved. Source repository/fork
coordinates and registry checksums are metadata, not legal or host trust approval.

`inventory_valid` is independent of `dependency_policy`. A bounded actual
`cargo deny --format json check` stores its original output and cleanup facts. A
known policy rejection remains failed even in a captured prefix; tool/DB/runtime
failure stays unknown. Java policy is currently unknown. Therefore this combined
development profile never becomes a passing security/release gate merely because
its inventory is valid. No dependencies are upgraded or advisories waived here.
See [cargo-deny output format](https://embarkstudios.github.io/cargo-deny/cli/common.html)
and [advisory diagnostic codes](https://embarkstudios.github.io/cargo-deny/checks/advisories/diags.html).

This is unsigned development evidence. Production IMap remains disabled fail-closed;
W0, final protocol/SDK identities and real-cluster release admission remain open.
