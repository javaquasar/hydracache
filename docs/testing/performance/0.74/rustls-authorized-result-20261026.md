# Authorized rustls security update, Rust 1.88 retained

Later on the same date the [separate WebPKI repair](webpki-security-result-20261026.md)
is applied. This report preserves the rustls-only boundary and its eleven
remaining findings; the WebPKI follow-up has its own paired baseline and gates.

The operator explicitly authorized compatible security updates on 2026-10-26,
superseding the earlier audit-only restriction. Commit `c626ea951` reapplies the
reviewed rustls minimum 0.23.46 and resolves only rustls 0.23.45 to 0.23.46 plus
its registry checksum. Every other lock package record is identical. Rust 1.88,
deny.toml, original live predicates/frequency/windows and frozen qualification
remain unchanged. This creates a new dependency/build identity, not a C74 freeze.

The upstream patched versions for [0346](https://rustsec.org/advisories/RUSTSEC-2026-0346.html),
[0347](https://rustsec.org/advisories/RUSTSEC-2026-0347.html),
[0348](https://rustsec.org/advisories/RUSTSEC-2026-0348.html) and
[0349](https://rustsec.org/advisories/RUSTSEC-2026-0349.html) were rechecked against
the primary records. The restored ordinary regression source is exactly the
archived `e02448617c7073e499c1d6ddbf7d3330518db0ae` test after newline normalization.
Its real full/resumed TLS 1.2/1.3 handshakes, binary application exchange and
compact/pretty session Debug oracle exercise the session secrecy fix. They are
not new exploit reproductions of all four advisories. No secret values are logged.

The [earlier first fixture failures, vulnerable-version refusals and rollback](rustls-security-result-20261024.md)
are retained and not rerun. New current-source candidate gates have separate
receipts; old green measurements/build/qualification receipts are not reused.

## Current focused results

| Gate | Result |
| --- | --- |
| Session regression and oracle, Windows Rust 1.94 | 3 passed, none ignored/filtered |
| Same regression, Windows MSRV 1.88, separate existing target directory | 3 passed, none ignored/filtered |
| Same regression, local Ubuntu GNU Rust 1.94 | 3 passed, none ignored/filtered |
| Complete ordinary RESP package | 181 passed, 23 unchanged ignored, 15 successful target/doc verdicts |
| Production-path mTLS and actual daemon lifecycle tests | 15 passed across two targets (11 mTLS, four daemon), none ignored/filtered |
| RESP/server all-targets check | Passed |
| RESP/server all-targets/all-features Clippy, warnings denied | Passed |
| RESP package format, doc-check, local non-promotable 0.74 performance-contract | Passed |
| Independent archive integrity and changed-index/object refusal | 3 passed |

These scopes overlap and must not be summed into a workspace milestone. TLS
controls are synthetic test traffic, not a product performance cohort. No
original-owner live replay, clock/service change, server installation or product
qualification follows from this patch. No CPU/allocation/throughput claim is made.

## Dependency policy remains red

The exact `cargo deny --locked check --disable-fetch` against clean advisory-db
commit `84d943a6deeb8cc6c6f632531aaf4f1c62275239` reports advisories FAILED, with
bans/licenses/sources OK. The four rustls findings disappear and eleven remain:
WebPKI, two trust-root packages, Tokio, YAML, three D-Bus packages and three ZIP
findings. This is a comparison against the pinned historical database, not an
assertion that no newer advisory exists. A fresh complete milestone remains due.
No new exception, feature/target exclusion or ignored warning is introduced.

Next isolate WebPKI/trust-root repairs and Tokio's FD-safety change, preserving
certificate rejection and descriptor lifetime behavior. YAML/ZIP and D-Bus
migrations remain separate. In particular zbus 6 requires a newer compiler than
the approved Rust 1.88 floor; do not silently raise it or suppress the findings.

The [71-member packet](local-runs/rustls-authorized-20261026/index.json) has index
SHA256 `5406e2d000d98e77b0d40e0bfa0772f8b54c0955454af4e12a8821136ba81b14`.
It retains exact baseline/current manifests and lock, regression source, unchanged
deny/qualification, current commands/results, policy output and the earlier index.
All current runs retain source/HEAD; only the explicit resolver step may change
Cargo.lock. The changed lock SHA256 is
`552f8c1fbb9fb06aadefca3fffb4f90cc1b78f4aed3e2c001cdf399b9b6bf0b1`.
The full package and production execution gates remain independently open.
The [21-member final-check packet](local-runs/rustls-authorized-20261026/final-checks/index.json)
retains the sealer, three integrity controls, final doc-check and staged Git-byte
verification. Its index SHA256 is
`f7b8b1b0db879718c332c6d06c5d70d98d4905b761e01a19a5fab95a982e001a`.
An additional final staging check validates all 189 sealed Git blobs across the
new closure/security/environment packets with zero byte mismatches.
