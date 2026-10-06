# Releases

Release pages are generated from the canonical notes under `docs/releases`. The site wrappers do
not copy their text: mdBook includes the canonical Markdown during every build. This keeps the
repository notes, GitHub release source, search index and public site on the same content.

## Published releases

- [HydraCache 0.71.0](0.71.0.md) — memory accounting, retained-byte evidence and active expiry.
- [HydraCache 0.72.0](0.72.0.md) — Management Center 2.0, bounded operational visibility and
  exact-candidate long-run evidence.

## Release candidates

- [HydraCache 0.73.0](0.73.0.md) — evidence-driven allocation/copy reductions, bounded HC/2
  backpressure and immutable six-/24-hour measurement archives. Publication remains conditional on
  green corrected exact-SHA gates.

## Development drafts

- [HydraCache 0.74.0 (draft)](0.74.0.md) — RESP/native throughput investigation and opt-in
  measurement tooling. No product candidate is accepted or frozen; no release admission is claimed.

## Verification records

- [HydraCache 0.72 verification](../reference/release-0.72-verification.md) — executable gates,
  campaign findings and accepted evidence rules.

Release notes before 0.71 predate the public mdBook release archive and remain available in the
[`docs/releases` repository directory](https://github.com/javaquasar/hydracache/tree/main/docs/releases).
