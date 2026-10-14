# Bounded OIDC roster local evidence

Implementation is `4fee68ab4d58a7f370e5462a06455e99016f5548`, preregistered
in `9c7d8bdb5679aea6f136a4388886c848d0efe9fb` against
`c9df8c06e12b7bf6fe99ba77d87a43f85b4a63c9`. The
[design](../../diagnostic-oidc-roster-design.md) and
[contract](../../diagnostic-oidc-roster-contract.toml) preserve the difference
between configured public-key verification and production enrollment.

## Implemented boundary

The fixed original OIDC reader accepts v1's single-key canonical LF document
within 2048 bytes and the separate v2 shape with one through eight keys within
8192 bytes. Strictly sorted kids and unique modulus/exponent pairs are mandatory.
A token identifies one configured member; signature failure cannot try another
key. Unknown kid, stale snapshot and original source replacement refuse without
refresh, fallback or repair. Worker/controller source budgets remain 2048 bytes.
The original claim, clock, input borrowing and whole-chain refusal gates remain.

## Local checks

The baseline has 10 OIDC/kernel-clock and 16 worker-source/kernel-binding checks.
After the expected missing-API refusal, all 18 focused cases passed in three
final repeats, including eight new roster cases. They cover each signing position,
wrong known kid/signature, unknown membership, count/order/material ambiguity,
canonical/RSA bounds, unchanged origin/header/time windows, source ownership,
constructor/rotation/downgrade refusal and healthy versus refused drop.
Seed `0x07402a11` exercises 64 mutations and four independently owned bindings.

Linux regression passes 49 original-input cases, five public-entry API checks
and 85 compile-fail doctests. Windows root suites pass six registry, 132 contract
and 13 release-evidence cases. Rust/Clippy 1.94 checks and strict all-target/
all-feature lint pass for the supervisor on Windows/Linux and xtask on Windows.
Supervisor all-target MSRV 1.88 checks pass on both platforms; strict Linux
Rustdoc passes. Scoped formatting, diff, documentation, 169-gate registry,
local performance contract, links and book checks pass. Require-ship remains
expected-red with native exit 1. No full release milestone gate was executed.

## Retained negative results

The first baseline log contains two passing test commands but an empty shell-
variable wrapper report and outer exit 1. The corrected set-e wrapper passes
the same baseline. The missing v2 API compile refusal is intentional E0432.
The first root invocation requests nonexistent `gated_test_check`; it runs no
tests and returns 101. The corrected command names `gated_tests`.

The local default Rust/Clippy 1.98 nightly regression log retains all passing
tests/check plus 12 strict lint failures in 11 unchanged files. Those files are
pinned against the baseline. The explicit 1.94 validator, matching Windows,
passes without source changes or lint suppression. The newer nightly is not
reported green and its lint drift remains separate tooling work. No failed
product measurement or qualification is replaced by these corrected commands.

## Reproduction and remaining prerequisites

Run the read-only verifier from this exact implementation or a documentation
descendant whose pinned sources remain unchanged:

```sh
python -E -s docs/testing/performance/0.74/local-runs/diagnostic-oidc-roster-20261014/verify_packet.py
```

It checks source/baseline Git objects, raw capture hashes, unchanged qualification,
both original frozen lock sources, repeat counts and closed admission. Windows-
managed Git worktree metadata needs the exact translated GIT_DIR for WSL reads.
Historical packets with different exact source pins belong to their original
checkouts; this extension does not rewrite them.

Only public synthetic RSA/controller/worker fixtures and owned temporary inputs
were used. The earlier real public JWKS is not promoted, selected or installed.
No actual private credential, token, network refetch, protected job, service,
SSH/server operation, manual SIGKILL or product workload was used here.
Signer/bootstrap custody and immutable request/token handoff are next in the
[job preparation](../../diagnostic-controller-job-preparation.md). Real issuer/
workflow/expiry selection, worker epoch, host/root enrollment, durable replay/
common fence, authenticated original start and all-writer cessation remain
before completion, later cells, server/pilot or qualification admission.

The packet verifier passed on Windows and Linux. Six owned copied-packet cases
passed on Windows, covering an unchanged control, raw-capture mutation, unknown
manifest version, boolean-versus-integer version, an admission flag set true and
a changed source digest. The earlier five-case capture remains retained. The
22 raw captures retain successful checks and negative results without rewriting
the originals. These offline tests perform no refetch or enrollment.
