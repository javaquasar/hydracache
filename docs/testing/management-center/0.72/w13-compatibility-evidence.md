# W13 compatibility, packaging and publication evidence

This is development evidence, not a ship receipt. Exact-candidate admission remains fail closed until
W14 validates an annotated candidate and the shipped 0.71 predecessor artifact.

## Implemented and exercised

| Proof | Result | Evidence |
| --- | --- | --- |
| Mixed-version RPC fencing | pass | `management_aggregation_072`: eight tests, including a real HTTP 0.71-style endpoint proving capability GET returns unsupported and snapshot POST count stays zero |
| Management API rollback switch | pass | `HYDRACACHE_MANAGEMENT_API_ENABLED=false` returns 404 for `/management/v1/**` while `/cluster/overview` remains 200 |
| Compatibility registry | pass | five schema-v1 artifacts and five mixed scenarios in `compatibility.toml`; xtask registry tests pass |
| Fresh downstream compile | pass | `hydracache-post-publish-consumer` compiles all five public DTO families and preserves future enum values as `Unknown` |
| Clean console install/test | pass | `npm ci`: 0 vulnerabilities; 16 unit tests, four deterministic package/supply-chain tests and 46 Playwright cases pass on desktop/mobile projects |
| Deterministic console bundle | pass | source/embedded bytes, four source registries and CycloneDX SBOM are bound by per-file and set SHA-256 plus exact source commit |
| Mixed artifact canary | expected red | `MC72-W13-MIXED-ARTIFACT` substitutes `console/app.js`; verifier rejects its digest |
| Publication ordering | pass | all 23 publishable crates pass `cargo package --list --locked` in the same order as release readiness |
| Partial publication recovery | pass | injected interruption after `hydracache-macros` retained a two-item prefix; `-Resume` continued from item 3 and completed all 23 |
| Bootstrap archive verification | pass | `hydracache-core` and `hydracache-macros` both pass `cargo package --locked` archive build and clean unpacked verification |
| Supply chain | pass | `cargo deny check`: advisories, bans, licenses and sources green after upgrading `h2`, `chacha20` and `spin` |
| Real mixed-binary machinery | implemented, receipt pending | dedicated ship-mandatory gate accepts only the full-history `v0.71.0` tag, starts real 0.71/0.72 daemons, exercises all five scenarios, and retains binary/provenance/observation digests |

## Required evidence that is unavailable

`git tag --list "v0.71*"` and `git ls-remote --tags origin "refs/tags/v0.71*"` both return no
artifact. The available `feat/0.71-memory-footprint-retention-efficiency` branch still declares
workspace version `0.70.0`; it is not a substitute for a shipped 0.71 binary. Consequently the
implemented `env.hydracache-run-management-mixed-072` gate is **blocked and non-promotable**, not
skipped or passed. Its executable scenario covers:

- old leader/new followers and new leader/old follower with actual 0.71/0.72 executables;
- leader change and old-peer restart during the mixed window;
- rollback to the shipped 0.71 binary before and after browser observation;
- package/archive launch against crates that depend on unpublished internal 0.72 packages.

The gate has no 0.65 development fallback. It requires `v0.71.0` in full history and verifies it is
an ancestor of the candidate. An explicit predecessor binary is accepted only when its supplied ref
and 40-hex commit equal that tag. Otherwise the gate builds the predecessor in a detached worktree,
refuses byte-identical binaries, and stores all five observations in
`target/test-evidence/0.72/management-mixed-071-072.json`.

The last archive limitation is expected in a staged crates.io release: after bootstrap crates are
published and indexed, runtime and adapter packages must be verified in order. W13 creates archives
only for the independently resolvable bootstrap set and uses `cargo package --list` for every other
crate. It does not publish packages or claim their post-publication consumer lane.

## Reproduction

```powershell
cargo test -p hydracache-server --test management_aggregation_072 --locked
cargo run -p xtask --locked -- evidence-run --release 0.72 --gate env.hydracache-run-management-mixed-072
cargo test -p hydracache-post-publish-consumer published_management_v1_contract_smoke_test --locked
cargo test -p xtask --test management_compat_072 --locked
npm --prefix console ci
npm --prefix console test
npm --prefix console run package
npm --prefix console run verify-package
./scripts/rehearse-publication.ps1 -Version 0.72.0 -FailAfter 2
./scripts/rehearse-publication.ps1 -Version 0.72.0 -Resume
./scripts/package-publishable.ps1 -Set bootstrap
cargo deny check
```
