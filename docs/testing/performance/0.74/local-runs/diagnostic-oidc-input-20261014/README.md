# Configured OIDC origin local evidence

Implementation is `94fad838c25351f1f9f311caa0f9eed95b7fa21a`, preregistered in
`5c9b0139` against `35210757cc505e159345c58608677aa537c4bdd1`.
The [approved local design](../../diagnostic-oidc-input-design.md) and
[contract](../../diagnostic-oidc-input-contract.toml) retain the distinction between
configured signature observation and independently enrolled GitHub provenance.
No actual trust/key/workflow selection, production token or host operation follows.

## Final checks

Ten focused cases pass in each of three final repeats: nine OIDC cases and one
original kernel-clock case. Earlier nine-case repeats remain retained as well.
The matrix includes original JWS bytes, strict origin/time/type/encoding checks,
exact scalar-count/key/value bounds, duplicate claims and headers, RSA/key confusion,
snapshot/config/owner drift, constructor refusal, healthy drop and permanent
whole-chain refusal. Seed `0x07552074` covers 32 signature mutations and four
independent bindings. Five new compile-fail constraints pass within 83 doctests;
the registered Linux API target passes four checks.

Regression passes controller/build 8, controller trust 8, worker trust 11,
worker/kernel 5 and kernel 19 with the existing manual case ignored. Corrected
Linux artifacts/reviewed-trust/worker-policy suites pass 29/8/15; Windows passes
15/8/14. Root suites pass 6 registry, 129 contract and 13 evidence tests.
Current supervisor check and strict Clippy, xtask strict Clippy and MSRV 1.88
checks pass on Linux/Windows. Linux MSRV Clippy is unavailable and was not run.
Strict Linux Rustdoc, scoped formatting, documentation/registry/local-contract,
link and book checks pass. Require-ship retains its expected native exit 1.
This is not a full milestone gate, manual SIGKILL proof or performance measurement.

## Retained negative results

`missing-api.log` has native 101 for absent module/binding. `development-tests.log`
has six of seven passing before the test wrongly expected a changed nonempty jti
to refuse. The corrected empty-jti test leaves production validation unchanged.
The formatter was initially invoked before its declared test file existed; adding
the test module resolves that construction-stage error, which is not a raw-log
functional result. `development-check.log` retains the library check pass.

`root-contracts.log` has native 101 with 127/129 passing: a formatting-sensitive
new marker and a historical lockfile/current-tooling mismatch. The corrected gate
still enforces original frozen lock hashes against both exact original Git sources,
and separately permits only the two approved supervisor dependency edges.
`linux-regression.log` retains the same lock mismatch in synthetic artifact
fixtures: 10/29 passing. Fixtures now derive the exact original lock bytes from
the unchanged package graph and assert the original SHA, rather than pretending
the new controller lock is the frozen observer lock. No runtime artifact verifier,
receipt hash, qualification input, version or checksum is relaxed.

## Source and custody boundaries

The only tooling-lock changes add direct Linux supervisor dependencies on already
locked base64 0.22.1 and ring 0.17.14. Every package/version/checksum and every other
dependency is unchanged. The observer standalone lock, signed build constants,
real selected public keys and frozen qualification remain unchanged. Historical
exact-source verifiers belong to their original checkouts; this new packet pins
the extended tooling separately.

The RSA PKCS8 test fixture was generated explicitly as public synthetic data with
OpenSSL and committed for repeatable tests. It is never a GitHub or deployment
credential. Synthetic RSA/controller/worker authorities and nonroot private
config/journal trees accompany actual local Linux packets. No real private key,
OIDC bearer token, secret, SSH, service or remote workflow is accessed.

Run `python verify_packet.py` for read-only source/raw-capture integrity, frozen
Git lock objects and exact dependency-delta checks. Full local Git history is
required; a missing original object refuses without network fetch or fallback.
Actual authenticated GitHub key acquisition/custody/rotation and a protected
signing-job producer remain before durable replay/common fence, original start,
watchdog/writer cessation, completion and cells two through four.
