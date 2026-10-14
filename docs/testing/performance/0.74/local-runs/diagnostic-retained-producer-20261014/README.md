# Retained diagnostic producer local evidence

Implementation is `e4a70f042f9d2049908a0dfab481b21de7652e93`, preregistered
in `23183d1072608db281d98d3016582774e588588b` against
`e30740fe1bc746bf66d30368be0995538e3c3bf9`. The
[retained design](../../diagnostic-retained-producer-design.md) and
[contract](../../diagnostic-retained-producer-contract.toml) define job-byte
consistency, not token or signer-bootstrap authority.

## Implemented and tested

The opt-in Linux sign_prepared_files consumes the original opaque preparation
and earlier audience. Retained canonical body/digest/build/audience checks run
before secret file IO; no public request/policy/receipt path can be supplied.
Selected controller, key metadata/encoding, clock brackets and exclusive synced
output retain the existing implementation. The legacy file CLI delegates but
its separate audience/sign commands still read public files independently.

The baseline has six passing producer cases. Before implementation, the
expected E0425 compile refusal names the missing retained signing functions.
The development run then passes all 12 cases, and three final repeats each pass
the same 12. Six new runtime cases cover original signatures after public file
replacement/removal or caller-buffer change, a different valid body with the
earlier audience, bad audience and expiry before key IO, internal body/hash/
audience drift, selected-key refusal and exclusive output. Seed `0x07403b21`
checks 32 retained bodies and four independent preparations.

Linux API checks pass six cases. All 88 doctests pass, including three new
compile-fail reuse/replacement/clone constraints. Regression passes 18 OIDC and
eight controller/build cases. Windows passes five portable producer cases and
root suites of six registry, 133 contract and 13 release-evidence cases.
Supervisor check/strict Clippy 1.94 and MSRV 1.88 checks pass on Windows/Linux;
xtask check/strict Clippy pass on Windows. Strict Linux Rustdoc, scoped formatting,
diff, documentation, 169-gate registry, local performance contract, local links
and book checks pass. Require-ship retains expected native exit 1. No full
release milestone, manual SIGKILL or performance measurement was executed.

## Reproduction and custody limits

The packet retains raw captures with hashes and exact source/baseline bindings.
The qualification digest, original frozen Git lock objects, builder/worker pins,
observer receipt, source schemas, existing workflows/CLI and host route remain
unchanged. Historical packets belong to their original exact checkouts; this
extension does not rewrite them.

```sh
python -E -s docs/testing/performance/0.74/local-runs/diagnostic-retained-producer-20261014/verify_packet.py
python -E -s docs/testing/performance/0.74/local-runs/diagnostic-retained-producer-20261014/test_verify_packet.py
```

Windows-managed worktree metadata needs the exact translated GIT_DIR for WSL
read-only Git operations. The verifier performs no refetch or enrollment.
Tests use owned temporary files, public builder material and the existing
explicitly synthetic controller key, never the actual controller credential.
No token, remote job, trust installation, host/service/SSH operation or product
workload is used. Audience equality and Rust ownership consumption neither
authenticate a token/executable nor consume a durable nonce.

Actual token coordination must retain this object through retrieval and strict
verification. Independent prebuilt-signer/runtime custody is a separate decision:
the frozen observer builder key purpose does not cover the signer. Actual
artifact/pin enrollment or a distinct signer-build receipt/domain/key role needs
review before implementation/activation. Original replay/common fence, host
epoch/root custody, authenticated start and all-writer cessation remain before
completion, later cells or server/pilot/qualification admission.

The read-only packet verifier passes on Windows/Linux. Five owned copied-packet
tests pass on Windows: control, future/boolean version refusal, raw-log mutation,
false bootstrap promotion and source-digest mutation. Seventeen raw captures
retain the final outcomes, including the preregistered missing-API refusal.
