# Approved public TLS roots for Release 0.74

The operator approved the complete upstream webpki-roots/webpki-root-certs
1.0.10 catalog on 2026-10-26, conditional on exact key and constraint audit.
That audit passes, and both catalogs are now applied to the accepted Tokio/TLS
graph. This closes the public-root scope decision, not the complete security
policy, release qualification or production enrollment.

The [earlier pending-candidate report](roots-security-result-20261026.md) and its
first narrow-delta refusal remain historical. Its eight-finding pre-Tokio
candidate is not the current combined graph. The new check uses the same pinned
advisory database and finds seven remaining advisories, rather than the preceding
active graph's nine. No deny exception or MSRV increase is introduced.

## Exact trust scope

The [approval record](../../../../tools/roots-trust-admission-074/approved-catalog.json)
lists all six changed public SPKI SHA256 and certificate SHA256 identities.
Complete source comparison confirms 118 entries become 120 in both forms:
116 complete anchors including constraints and 116 DER certificates are
unchanged. Removed roots are emSign ECC Root CA-G3 and Atos TrustedRoot 2011.
Added roots are Telia EC/RSA TLS Root CA v3 and SECOM TLS RSA/ECC Root CA 2024.

Each changed anchor maps to exactly one changed DER certificate by parsed
subject and SPKI fields, not substring presence. All four added roots have
critical BasicConstraints CA=true, no path-length field and no NameConstraints;
complete DER fingerprints bind every remaining certificate field. Old name
constraints are not dropped. The corrected regression oracle still distinguishes
the TUBITAK anchor's out-of-band `.tr` constraint from DER-derived policy rather
than falsely declaring both representations identical.

Cached `.crate` archives match the published Cargo checksums. Registry source,
manifest, VCS metadata and license bytes match the sealed earlier upstream
copies. Their old and new VCS identities are
`50729cd212ccd332be193856258fa93da2876d6a` and
`70760478b9d3ea729b22faa0b5324da38a297c1c`. The complete delta, rather than the
last upstream removal PR alone, is the basis of this acceptance. The
[anchor advisory](https://rustsec.org/advisories/RUSTSEC-2026-0381.html) and
[DER advisory](https://rustsec.org/advisories/RUSTSEC-2026-0383.html) identify
1.0.10 as patched; the legacy 0.26.11 shim depends on its resolved 1.x catalog.

This accepts those four public authorities in the existing bundled public TLS
store. It changes no production issuer/enrollment pin, tenant authorization,
custom mTLS CA policy or original-execution admission. It is not an independent
audit of each CA's issuance practices or a TLS exploit reproduction.

## Implementation and checks

`Cargo.lock` changes exactly two catalog versions/checksums and the qualified
legacy-parent reference. Three dev-only edges in the RESP manifest enable both
representations and the shim. New catalogs are constrained exactly to `=1.0.10`,
preventing the earlier range-based selection of untested 1.0.11. Every other
package/feature and the root manifest remain unchanged. The five Rust regression
tests are byte-identical to the archived corrected baseline/candidate oracle,
SHA256 `655728cfa43e04ef1e26eabbfba8b382d5ddab8ce6232c1ef8d3603327c83f73`.
No runtime Rust source is edited.

Implementation commit: `63cb4d7b9bca1c901b2f7420f86effd3309a6882`.

| Current combined graph check | Result |
| --- | --- |
| Exact trust/publication audit and Python refusal controls | Passed, eight controls |
| Roots/session/SPKI on Windows Rust 1.94 | 10 passed, three targets |
| Same cohort on MSRV 1.88 and GNU Rust 1.94 | 10 passed per environment |
| Server mTLS and daemon process | 15 passed, two targets |
| Complete ordinary RESP package | 188 passed, 23 unchanged ignored, 17 target/doc verdicts |
| RESP/server all-targets check and strict all-feature Clippy | Passed |
| Formatting, doc-check and local performance contract | Passed |
| Pinned advisory database | Seven remaining findings; policy still fails |
| Old V7 dispatcher | Refuses auxiliary drift before dispatch, as required |

These overlapping cohorts are not summed. The advisory DB remains clean at
`84d943a6deeb8cc6c6f632531aaf4f1c62275239`; the remaining IDs are
RUSTSEC-2024-0449 and RUSTSEC-2026-0367/0368/0369/0370/0371/0372.
Bans, licenses and sources pass. No fetch occurs, so this is a reproducible
pinned-database comparison, not an assertion that no later advisory exists.

## Retained evidence

The [144-member packet](local-runs/roots-approved-20261026/index.json) retains
20 phases, the accepted-input baseline, exact current manifests/lock/oracle,
approval record, audit implementation, published upstream bytes and the old
scope refusal. Its index SHA256 is
`284eb6ced97d68fe7ff7711c2611a723265fe4a08a2894de293ed4a14f6018d9`.
Its [data-only verifier](local-runs/roots-approved-20261026/verify.py) and seven
tamper controls pass without Git, processes or execution of archived source.

The first sealer invocation fails before index/object creation because implicit
Windows cp1252 cannot decode the UTF-8 dependency log. The exact first sealer
source and a clearly labeled transcription of the tool-reported error are
retained; its initial stdout/stderr were not redirected as separate raw files.
Explicit UTF-8 decoding repairs only evidence assembly, with every command log
and first policy/source-scope refusal unchanged.

## Preserved boundaries and next gate

The accepted-input snapshot precedes the resolver. Full graph comparison accepts
only the two root updates and three test edges, and all previously tracked
runtime source remains identical. The old packet verifier still passes with
its original pending/refusal flags; the old archive is not relabeled approved.
The new approval, audit and current results are retained separately.

V7's reviewed auxiliary catalog and the earlier exact historical lock views
predate this accepted update. Its new preflight records `reviewed auxiliary
input drift`, with no package dispatch. Preserve V7's earlier capacity refusal
and original pins. Next implement a narrow, whole-body-bound historical input
successor, perform fresh source/effect review and run a distinct full package
under the same 780-second/four-thread/floor contract. No old view or review pin
is broadly refreshed by this trust approval.

The qualification manifest remains SHA256
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
All live predicates, observation frequency and original 300/60/10 origins/windows
remain unchanged. No performance hypothesis/claim, original-owner replay,
server/service/clock operation, installation or qualification occurs.
