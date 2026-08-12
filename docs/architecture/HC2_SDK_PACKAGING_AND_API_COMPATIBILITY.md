# HC/2 SDK Packaging and Preview API Compatibility

## Purpose

HydraCache 0.68 introduces independently consumable HC/2 SDK artifacts. The
Rust `hydracache-client-hc2` crate is part of the publishable `0.68.0` Rust
distribution. The Java modules and Python wheel remain repository-built source
previews and are not published to Maven Central or PyPI in this release. A
workspace build is not sufficient evidence that any of these artifacts can be
consumed. This contract freezes the minimum preview API and requires every
ecosystem to pass a clean-package consumer proof.

The authoritative manifest is
[`../compatibility/hc2-sdk-api-v1.json`](../compatibility/hc2-sdk-api-v1.json).
It binds protocol generation 6 to four preview packages:

- Rust `hydracache-client-hc2`;
- Java `io.hydracache:hydracache-client-hc2`;
- Java `io.hydracache:hydracache-hazelcast-facade`;
- Python `hydracache-client-hc2`.

`preview-frozen` means that every listed public symbol must remain available
through the 0.68 preview line. Additive APIs are allowed. Removing or renaming
a listed symbol, changing its package coordinate, or changing its meaning
requires an explicit manifest-version and compatibility review. This v1
manifest is a minimum source-surface baseline; H18 retained-artifact replay and
future ecosystem-native compatibility tooling remain responsible for binary
compatibility beyond the clean consumers compiled here.

## Reproduction

Run the unified gate from the repository root:

```text
cargo run -p xtask --locked -- client-package-check
```

The command uses only fixed scratch paths below `target/hc2-package-check` and
performs the following proofs.

### Rust crate

1. `cargo package --locked` creates the publishable `.crate` archive.
2. The gate extracts that archive rather than using workspace source paths.
3. A new standalone Cargo project imports every frozen Rust symbol.
4. `cargo check --offline` proves the packaged crate is sufficient to compile
   the consumer with the already locked dependency supply.

The Rust package retains the current workspace version until the isolated
`0.68.0` release-cut commit. The API manifest records the checkout's exact
version; the release cut must update the workspace and manifest together. The
deferred `0.67.1` dedicated qualification does not block this code release and
does not authorize numerical performance claims.

### Java JARs

1. The Java SDK reactor builds and installs the native client and facade JARs,
   source JARs, Javadoc JARs, manifests, and capability resources.
2. The gate verifies that every frozen Java type has a public source file.
3. An external Maven project under `tests/java-hazelcast-facade-consumer`
   compiles representative `HydraMap` and `HydraFencedLock` calls against the
   installed facade JAR.
4. Its test proves the class was loaded from a JAR, the explicit
   `Hazelcast-Wire-Compatible=false` manifest claim is present, and the packaged
   capability resource remains readable.

GitHub Actions repeats the Java build and external-consumer test on Temurin 17
and 21. Java 17 remains the emitted bytecode floor; Java 21 proves runtime and
toolchain compatibility without raising that floor.

### Python wheel

The Python SDK uses the repository-owned, zero-dependency PEP 517 backend in
`sdks/python/hydracache-client-hc2/build_backend`. The backend is intentionally
small because the release gate must work with `--no-index` and
`--no-build-isolation`; it emits a pure `py3-none-any` wheel with stable entry
ordering, timestamps, permissions, metadata, and RECORD hashes.

The gate builds the wheel twice in separate directories and requires identical
SHA-256 digests. It then creates a fresh virtual environment, installs pinned
runtime dependencies from the checked-in platform wheelhouse with
`--require-hashes`, installs the produced wheel, and checks the exact version
and exported symbol set in isolated Python mode.

## CI and release use

The required Linux HC/2 lane runs the complete package gate. A dedicated Java
matrix gives explicit Java 17/21 evidence. Neither lane itself publishes
artifacts. Publishing the Rust crate remains prohibited until:

- the 0.68 release-evidence manifest accepts all required exact-candidate
  receipts;
- cross-SDK conformance passes;
- version and provenance review is complete; and
- the exact commit passes the hosted Linux, digest-pinned Docker, and fuzz
  admission scope.

Publishing the Java or Python clients remains prohibited until the full
four-lane client-promotion admission, including the fixed-host lane, is green
and a later release decision changes their preview versions.

Generated-code drift, semantic parity, real-process behavior, and fixed-host
soak evidence are separate gates. A green package gate must never be presented
as proof of those properties.
