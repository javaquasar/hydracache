# HC/2 Hermetic Python Generation

## Status and scope

H15 originally added independently executable Python message and gRPC fixture
evidence. The fixture has now been promoted, without changing its authoritative
schema or offline wheelhouse, into the preview production SDK at
`sdks/python/hydracache-client-hc2`. The generated package remains internal;
`hydracache_hc2` supplies the bounded asyncio network API. The authoritative
schema remains the reviewed files under `crates/hydracache-client-hc2/proto`
and the isolated W0 spike envelope under `crates/hydracache-client-plane-spike/proto`.

ADR-0020 keeps version `0.68.0a1` as a repository source preview for the 0.68
Rust release. CI continues to build and install its deterministic wheel in a
clean virtual environment, but no PyPI upload is authorized. External Python
distribution requires the later full Linux/Docker/fuzz/fixed-host promotion
admission.

## Reproducible generation

`cargo xtask client-plane-python-generate --write` performs generation without
a system `protoc` or Python generator package:

1. `protoc-bin-vendored 3.2.0` supplies the platform compiler used for Python
   messages, type hints, and a descriptor set.
2. The repository Rust generator reads that descriptor set and emits gRPC
   channel/servicer bindings for every declared service cardinality.
3. The same descriptor produces deterministic JSON contract metadata containing
   files, packages, messages, field IDs/types/oneofs, enums, and service methods.
4. The checked-in package is regenerated into `target` and compared byte for
   byte. Missing, extra, or changed files fail with the regeneration command.

This avoids a handwritten second operation registry. The custom gRPC emitter is
versioned as `hydracache-hc2-python-1`; changing its output requires an explicit
generated diff.

## Offline runtime supply chain

The SDK validation does not invoke an index or perform a best-effort install. Its
wheelhouse contains exact unmodified wheels for:

- `protobuf==6.33.4`;
- `grpcio==1.76.0`;
- `typing-extensions==4.15.0`.

`wheelhouse.lock.json` verifies the exact filename set, lengths, and SHA-256
digests before environment creation. `requirements.lock` repeats the hashes for
`pip --no-index --require-hashes`; `PIP_NO_INDEX=1` and version-check suppression
are applied to every install/check command. Sdists and dependency resolution
from the network are not fallback paths.

The required matrix is intentionally only CPython 3.12/Linux glibc x86-64 and
CPython 3.13/Windows x86-64. Unsupported Python, OS, architecture, ABI, musl, or
missing wheels fail the required gate. Extending the matrix requires reviewed
wheels, hashes, license records, and an executing CI row.

## Executable evidence

`cargo xtask client-plane-python-check` proves:

- a clean descriptor-driven regeneration;
- exact wheelhouse integrity and supported runtime identity;
- installation into a fresh venv with index access disabled;
- package import and exact runtime/generator versions;
- equality with the Rust/Java HC/2 golden envelope;
- protobuf unknown-field round-trip preservation;
- descriptor-derived contract metadata;
- a real local gRPC bidirectional stream through generated stub and servicer
  bindings.

`cargo xtask client-plane-spike-check` now includes this Python gate after the
existing Rust and Java evidence. Pull-request CI selects Python 3.12 explicitly
and invokes the combined gate. Network access used by GitHub Actions to obtain
the runner/toolchains is outside the fixture; Python dependency installation
itself is forcibly local and hash-bound.

## Maintenance and rollback

Dependency refreshes must be isolated and review PyPI release provenance,
license, supported ABI wheels, digests, generated diffs, golden behavior, and
both required platforms. Never replace a missing wheel with a source build or
relax `--require-hashes`. The generated package may not be removed independently
of the SDK. Runtime API changes require the cross-SDK conformance and package
compatibility gates; Rust/Java generation remains available independently.
