# Bundled public roots: tested candidate, trust-scope decision pending

The compatible security candidate updates webpki-roots and webpki-root-certs
1.0.8 to 1.0.10. Its focused tests pass, but it is **not accepted or installed**:
the complete catalog changes more than removing the distrusted emSign key.
The exact candidate manifests, lock, five-test implementation, first failures,
coordinator and published upstream source/licenses are retained in the
[146-member packet](local-runs/roots-security-20261026/index.json).
No separate branch is needed to recover those exact files. The active lock and
RESP manifest are restored to the accepted pre-experiment graph; the candidate's
new test is archived rather than left active against the old vulnerable roots.
Do not describe its eight-finding result as the current graph: **ten findings
remain in the accepted graph** at the previously pinned advisory database.

## What the hypothesis actually changes

The [anchor advisory](https://rustsec.org/advisories/RUSTSEC-2026-0381.html) and
[DER-certificate advisory](https://rustsec.org/advisories/RUSTSEC-2026-0383.html)
identify patched releases starting at 1.0.10. The legacy webpki-roots 0.26.11 shim
reexports its parent catalog; it is not a separately repaired trust store.
The [upstream removal](https://github.com/rustls/webpki-roots/pull/133) agrees with
the public emSign G3 SPKI hash observed in the old catalog:
`8d417db2dd8bf5e3084d1e3f196d583849d81bdd4c00c70b9d39369e96b8c782`.
No private keys are acquired or used.

Full archived-source comparison, rather than only the last upstream PR, finds:

| Exact catalog delta | Observed result |
| --- | --- |
| Anchor and DER entries | 118 to 120 in each representation |
| Complete unchanged anchors, including constraints | 116 |
| Removed | emSign ECC Root CA-G3; Atos TrustedRoot 2011 |
| Added | Telia EC TLS Root CA v3; Telia RSA TLS Root CA v3; SECOM TLS RSA Root CA 2024; SECOM TLS ECC Root CA 2024 |

The independent inventory retains all six changed subjects, public SPKI contents,
full DER SPKI SHA256, certificate SHA256 and constraint values. Each changed
anchor matches exactly one corresponding DER certificate; none are unmatched.
Published source provenance is old commit
`50729cd212ccd332be193856258fa93da2876d6a` and candidate commit
`70760478b9d3ea729b22faa0b5324da38a297c1c`.

The narrow admission gate expected only emSign removal and deliberately fails
on this larger delta. An observational follow-up preserves that same refusal;
the separate successful inventory is not a replacement admission gate. A project
scope question about accepting all four new public authorities is outstanding.
No production issuer/enrollment pin, policy, tenant authorization or live kernel
predicate changes. Changing a public TLS trust catalog still requires an explicit
trust-scope decision; passing compatibility tests does not make that decision.

## Baseline, oracle correction and candidate

Three dev-only edges exercise both representations and the legacy shim. The
baseline resolves them to the old catalogs without changing package versions.
The first test oracle incorrectly assumed identical ordered DER-derived and
anchor stores. Sorting corrects order, but still exposes a real policy difference:
TUBITAK Kamu SM SSL Kok Sertifikasi - Surum 1 has an out-of-band `.tr` constraint
in the anchor catalog, absent from the DER-derived anchor. Both initial failing
oracles and their logs are preserved; they are test defects, not product refusals.

The corrected oracle compares complete subject/SPKI multisets, permits exactly
that registered constraint difference and refuses dropped entries, changed keys
or omitted constraints. It does not declare the two APIs' trust policies equal.
The corrected baseline has three passes and two failures: emSign is still present
in both old representations. With **the same oracle bytes**, the 1.0.10 candidate
passes all five tests. This is catalog membership/policy regression coverage, not
an independent certificate-chain exploit or complete TLS handshake reproduction.

The first resolver also chose root-certs 1.0.11 under the candidate's range.
That lock is retained, never tested or admitted. Before testing, an explicit
resolver step selects root-certs 1.0.10. Full lock comparison proves only two
package versions/checksums and their qualified parent references change, in
addition to the three dev-only test edges; no runtime source/features are edited.

## Focused results, not full release gates

| Candidate check | Result |
| --- | --- |
| Roots/session/SPKI cohort, Windows Rust 1.94 | 10 passed, three targets |
| Same cohort, Rust 1.88 MSRV | 10 passed, three targets |
| Same cohort, local GNU Rust 1.94 | 10 passed, three targets |
| Server mTLS and daemon process | 15 passed, two targets |
| Complete ordinary RESP package | 188 passed, 23 unchanged ignored; 17 target/doc verdicts |
| RESP/server all-targets check, strict all-feature Clippy | Passed |
| RESP formatting, doc-check, non-promotable local performance contract | Passed |
| Offline evidence integrity/tamper controls | Six passed |
| Narrow trust-delta gate | Failed; broader trust change not accepted |
| Pinned-database dependency policy | Advisories FAILED; bans/licenses/sources OK |

These overlapping cohorts must not be summed. The unchanged clean advisory DB
`84d943a6deeb8cc6c6f632531aaf4f1c62275239` reports eight findings **only on the
pending candidate**: RUSTSEC-2024-0449 and RUSTSEC-2026-0367/0368/0369/0370/0371/
0372/0386. The two roots findings disappear on that candidate. This is not a fresh
all-advisory assessment, a full package pass, or a throughput/allocation result.
No exception or MSRV increase is introduced. Tokio and the separate major
YAML/ZIP/D-Bus work remain open.

## Reproducibility and next boundary

Final archive-only checks preserve an additional infrastructure refusal: the
first Cargo-wrapped doc-check reaches its 180-second build guard while compiling
xtask, before any documentation verdict. Its deadline is not extended and its
result is not rewritten. The separately scheduled local contract command passes
and completes compilation in its unchanged guard. A separate doc-check on that
just-built artifact passes in 13.996 seconds; executable SHA256 is
`0077689ea83d65aeaf6b8e99ca321addf15dbe97cb27ebed9ff3fedca5a91031`.
It is not a successful first Cargo-wrapped doc attempt or full-package coverage.
The [36-member final-check packet](local-runs/roots-security-20261026/final-checks/index.json)
retains both distinct outcomes, all six integrity controls, restored-source
checks, exact artifact identity and the first staged-byte verification (133
blobs, zero mismatches). Final index SHA256 is
`05f76b71215a7e5011287664bc069d41213965774379a3885bfab06dbb67aff8`.

Index SHA256 is
`5a66272e834fd917f87f7063f1415d1e50beba5b18f776104a7b8a19c4a6f73d`.
Run `verify.py` and `test_verify.py` from the packet with Python 3.11 or later.
The verifier checks all object bytes, complete lock delta, same corrected oracle,
first failures, counts and refusal/admission boundaries. To recover an exact
file offline, look up its logical member in index.json and read
`objects/<sha256>`; `source/` contains the candidate, `baseline/` its corrected
paired baseline, and `original/` the accepted manifests/lock. Do not copy the
candidate into a live checkout without the trust decision and separate review.

The original qualification manifest digest remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
No original live check/order/frequency or 300/60/10 origin/window changes, V6
retry, original-owner replay, clock/service/server change, release installation,
product load or expensive qualification occurs. Next either admit an explicitly
pinned full catalog after the trust decision, or keep this candidate archived
and handle the still-independent Tokio repair. All lifecycle, full milestone,
production trust and host-admission gates remain separate and open.
