# WebPKI distrust repair: paired public-key regression

Commit `8d9dbdd3d` changes only rustls-webpki 0.103.15 to 0.103.16 and its registry
checksum in Cargo.lock, plus an ordinary regression and upstream license notice.
Cargo.toml, Rust 1.88, deny policy and frozen qualification are unchanged. This
is the next separately authorized compatible security repair, not a performance
candidate or a repeated V6. The earlier rustls-only result remains historical.

## First baseline, unchanged oracle, isolated implementation

The primary [RustSec record](https://rustsec.org/advisories/RUSTSEC-2026-0382.html)
and [upstream backport](https://github.com/rustls/webpki/pull/546) identify eight
distrusted public SPKIs and the 0.103.16 fix. Public vectors are pinned to upstream
commit `8ecdcc76f56b68fda67a6adf7976f5eed5006a4c`, with per-vector SHA256 and exact
hex retained. No compromised private key is downloaded, embedded or used.

The new test uses each public point as a leaf key in a valid certificate signed
by a synthetic trusted CA. Certificate verification needs the CA signature, not
the leaf private key; this does not claim possession or a full TLS handshake with
those compromised keys. All eight chains are accepted by the first 0.103.15
baseline, which fails the rejection assertion. The other baseline test passes:
ordinary valid chain accepted, foreign hostname, not-yet-valid/expired times and
foreign issuer rejected. Fixed test times do not adjust any live system clock.

The exact same oracle bytes on 0.103.16 reject all eight with CertDistrusted,
while the ordinary checks still pass. This directly checks leaf-SPKI distrust;
it is not an independent exploit reproduction for every intermediate/cross-sign
path in the upstream change. Existing mTLS tests additionally retain invalid
client, CA/name, authorization/tenant, fragmentation, cap, timeout and shutdown
behavior. No original-owner predicate, observation frequency or deadline changes.

## Focused gates and remaining security boundary

| Scope | New result |
| --- | --- |
| SPKI and session tests, Windows Rust 1.94 | 5 passed, none ignored/filtered |
| Same tests, MSRV 1.88 in separate existing target directory | 5 passed, none ignored/filtered |
| Same tests, local GNU Rust 1.94 | 5 passed, none ignored/filtered |
| Combined session/SPKI/server mTLS/daemon cohort | 20 passed across four targets |
| Complete RESP package | 183 passed, 23 unchanged ignored, 16 successful target/doc verdicts |
| RESP/server all-targets check and all-targets/all-features strict Clippy | Passed |
| RESP format, doc-check, local non-promotable performance-contract | Passed |
| Independent packet/index/public-vector integrity controls | 3 passed |

These scopes overlap; do not sum them into a workspace milestone. The lock repair
protects this exact locked workspace. It does not raise rustls's upstream
transitive minimum or certify downstream consumers that retain an older lock;
their resolution must separately include rustls-webpki 0.103.16 or a patched
version. No product throughput/allocation result or production admission follows.

Against the unchanged clean advisory-db commit
`84d943a6deeb8cc6c6f632531aaf4f1c62275239`, cargo deny still reports advisories
FAILED, bans/licenses/sources OK. Ten findings remain after WebPKI's finding
disappears: two trust-root packages, Tokio, YAML, three D-Bus packages and three
ZIP findings. This is a pinned-database comparison, not a fresh all-advisory
assessment. No ignore, exception or target exclusion is added. Next isolate the
bundled trust roots and Tokio repair; major parser/build/D-Bus work stays separate.

## Retained evidence

The [79-member packet](local-runs/webpki-security-20261026/index.json) has index
SHA256 `edcd229bcf2afac2a6fc1e099e94777e9af2946140d8df2e3cda55b588164c80`.
It retains the first vulnerable-version failure, exact baseline/current lock,
unchanged manifest/policy/qualification, same oracle, upstream provenance and
license, and all focused commands/results. Only the explicit resolver step
changes Cargo.lock; other runs retain source/HEAD. No original live replay,
clock/service/server change, installation or product qualification occurs.
Full package coverage and new exact-source compatibility/effect review are still
required before a complete package dispatch; old receipts are not refreshed.
The [21-member final checks](local-runs/webpki-security-20261026/final-checks/index.json)
retain archive integrity/refusal tests, final documentation and staged Git-byte
verification; index SHA256 is
`9dbe582c0356ba3271ab8ffa465d4c8abccc010a4278ae24ec4be263a4760c86`.
Final staging verifies all 285 sealed Git blobs in the four new packets with
zero mismatches, not just hashes of unchanged working files.
