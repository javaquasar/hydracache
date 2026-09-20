# Release 0.72 Verification

HydraCache 0.72 is admitted from executable evidence, not from a checklist marked by hand. This
page describes the public verification contract for the Management Center 2 release. The
machine-readable source of truth remains `docs/testing/release-evidence/0.72.toml` together with
`docs/testing/gated-test-registry.toml` in the repository.

## What is verified

The release plan is split into work items W0-W14. Admission covers all of them:

| Area | Verification |
| --- | --- |
| Contract and provenance | Versioned DTOs and schemas, source-to-view mapping, deterministic digests, exact source commit and artifact identity. |
| Read-only security | Management routes require `management.read`; write administration is separate; unknown or unavailable truth is never presented as healthy. |
| Cluster dashboard | State, members, clients, placement, migrations, health checks, storage, streaming, messaging and CP views are tested through API and browser paths. |
| Failure semantics | All 13 registered source-to-management failure classes, partial collection, stale data, overload, deletion subsets and recovery transitions. |
| Real processes | One-daemon and three-daemon tests use the production binary and the content-hashed production console bundle. |
| Protocol coexistence | Real HC/1, mTLS HC/2 and RESP traffic, including shared dispatch, drain and continued observability. |
| Durability and recovery | Corruption, truncation, ENOSPC, snapshot, identity, reconciliation and stale-peer sources; unretained causes remain explicitly `unknown`. |
| Compatibility | Real shipped 0.71 and candidate 0.72 binaries, mixed leadership, old-peer restart, rolling upgrade and rollback. |
| Resources and longevity | Latency, file-descriptor and RSS bounds plus six-hour candidate and 24-hour ship soaks. |
| Robustness | Four decoder fuzz targets, hostile corpus replay, coverage threshold, canary injection and fail-closed admission. |
| Packaging and supply chain | Deterministic console bundle, SBOM, downstream DTO compilation, publication rehearsal and full dependency policy. |

## Test layers

No single layer can replace another:

1. Fast unit, schema and registry tests validate deterministic transformations and ownership.
2. API and integration tests validate authorization, typed envelopes, partial truth and recovery.
3. Browser tests validate desktop/mobile rendering, filtering, accessibility and unavailable states.
4. Production-process tests launch the real daemon and read the exact embedded UI asset.
5. Linux-only resource tests own numerical FD and RSS claims.
6. Mixed-binary tests own 0.71/0.72 upgrade and rollback claims.
7. Candidate and ship soaks own wall-clock stability claims.
8. Fuzz, coverage and expected-red canaries prove decoder and admission boundaries.

An ignored test, an empty feature-gated test binary, a skipped environment or a prose report is not
release evidence. Missing capabilities produce a loud blocking result.

## Candidate and ship gates

The candidate gate runs for exactly six hours. The ship gate first repeats the candidate gate and
then runs for exactly 24 hours on the same labelled self-hosted Linux runner. Both use three
production daemons and a fixed seed. They poll the typed dashboard every second, send HC/1 writes
over a declared 64-key ring and RESP traffic every second, restart a follower hourly, require
visible partial truth followed by full recovery, and enforce latency, file-descriptor and RSS
ceilings. The receipt is persisted before terminal budget assertions so a failed ceiling retains
the measured baseline and final values.

The protocol-coexistence gate separately proves HC/1 and mTLS HC/2 against the same exact candidate.
The compatibility gate uses the full-history `v0.71.0` predecessor and exercises all five registered
mixed-version scenarios, leader change, old-peer restart and rollback. None of these durations or
binary identities can be overridden by an ordinary CI runner.

## Readiness and recovery rules learned from the campaign

Dedicated-host execution hardened the harness in four places:

- Dashboard, HC/1 and RESP readiness are checked independently. Each bounded probe preserves its
  last protocol-specific error and failure of any surface fails the gate.
- The soak topology explicitly enables the production HC/1 listener; a TCP connection alone is not
  accepted as HC/1 evidence.
- Sustained HC/1 traffic uses one bounded, reusable HTTP client and consumes complete responses.
- Chaos Mesh IOChaos deletion has a bounded 120-second controller-reconciliation window. The
  object must still be deleted and the replacement pod must become ready; an uncleared finalizer,
  stale target or timeout remains a hard failure.
- Hourly fault counts use the soak's exclusive deadline. A six-hour run therefore requires the
  five scheduled recoveries at hours 1-5, and a 24-hour run requires the 23 recoveries at hours
  1-23. The endpoint at hour 6 or 24 is the completed duration, not another event inside the run.
- HC/1 writes use a fixed 64-key ring. This preserves one real write per second while keeping the
  Management Center endurance workload cardinality-bounded; dedicated retention campaigns own
  unbounded-cardinality and expiry-churn claims.

These corrections improve fidelity and diagnostics. They do not reduce traffic, recovery or
resource thresholds, and failed earlier attempts are preserved rather than overwritten.

## Evidence identity

Every receipt is bound to one clean 40-character source commit. It records the gate and command
digests, registry digest, binary/UI/schema/SBOM digests where applicable, fixed seed, host
fingerprint, timestamps, samples, observations, resource series and artifact digests. A retry links
to its predecessor and cannot overwrite it.

This has an important consequence: a documentation-only commit is still a new source commit. A
receipt produced for an earlier SHA may remain useful historical evidence, but it cannot be
relabeled as evidence for the new release candidate. The exact-SHA gates must be regenerated for
the commit that is actually promoted.

## Admission decision

The final release-evidence command validates every required receipt before aggregation. It rejects
missing mixed-version, soak, resource, fuzz or coverage evidence; dirty or stale commits; changed
commands or registries; mismatched artifacts; overwritten retries; and expected-red canaries that
do not fail for the registered reason.

Documentation explains the contract but never grants a pass. Only validated artifacts from the
frozen candidate can move 0.72 to ship status.

## Documentation-site checks

Changes to this page are verified with the same site pipeline used for GitHub Pages:

```powershell
cargo fmt --manifest-path docs-site/examples/Cargo.toml --check
cargo check --manifest-path docs-site/examples/Cargo.toml --all-targets --locked
mdbook build docs-site
node scripts/docs-link-check.mjs
node scripts/docs-visual-smoke.mjs
```
