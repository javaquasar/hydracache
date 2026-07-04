# TD-0003: Dependency upgrade policy and backlog

## Status

Open. Tracks the workspace dependency upgrade backlog and the policy for acting
on it. Reviewed whenever `cargo update` runs or an IDE/Dependabot flags a newer
version.

## Context

IDE tooling (and Dependabot-style scanners) highlight "newer version available"
for many `[workspace.dependencies]` entries. That signal is **noisy**: it reports
the latest *published* version regardless of (a) semver compatibility with the
existing requirement, (b) pre-release status (`alpha` / `rc`), and (c) transitive
pins that block an upgrade. Acting on it indiscriminately risks pulling
pre-release storage/ORM crates into a project that claims production posture, or
chasing an upgrade that a transitive dependency forbids.

This document sorts the backlog into three buckets — **do now**, **schedule**, and
**blocked / do not** — so upgrades happen deliberately and under the release gates
(`cargo xtask verify`, `cargo deny check`), not by IDE highlight.

## Policy

1. **Semver-compatible refreshes are routine, not migrations.** The manifest uses
   default caret requirements (e.g. `bytes = "1.11.1"` means `^1.11.1`), so newer
   compatible minor/patch versions are already permitted — they only need a
   `Cargo.lock` refresh via `cargo update`. Do this regularly; it keeps security
   patches current. Gate: `cargo xtask verify` must stay green.
2. **Major bumps are dedicated PRs.** One crate (or one cohesive group) per PR,
   with the full gate suite and the relevant integration tests run. Never bundle a
   major bump with feature work.
3. **No pre-release dependencies in the default build.** `alpha` / `rc` versions
   are not adopted for production crates (storage, ORM, consensus). Wait for the
   stable release.
4. **Transitively-pinned upgrades are architectural decisions, not bumps.** If a
   transitive dependency pins an old version, the upgrade is tracked as its own
   debt item and resolved through the owning decision (see TD-0002).
5. **`cargo deny` stays authoritative.** Advisory ignores live in `deny.toml` with
   a reason and a TD reference; an upgrade that clears an advisory must also remove
   the corresponding ignore.

## Backlog

### Bucket A — Do now (semver-compatible, `cargo update` only)

These are within the existing major and allowed by current requirements; refresh
the lockfile and run the gates. Examples flagged at time of writing: `axum`,
`bytes`, `syn`, `serde` / `serde_json`, `tokio` / `tokio-stream`, `tower`,
`utoipa` / `utoipa-swagger-ui`, `proc-macro-crate`, `quote`, `trybuild`,
`proptest`, `toml`.

Action:

```bash
cargo update
cargo xtask verify
```

This is lockfile hygiene, not a migration. Consider automating with a scheduled
`cargo update` PR (Dependabot/renovate) plus the existing `cargo deny` gate.

### Bucket B — Schedule (real major bumps; dedicated PR + tests)

| Crate | Current → flagged | Notes / effort |
| --- | --- | --- |
| `sqlx` | `0.8` → `0.9` | Core of the DB adapters; breaking API. Run `hydracache-sqlx` / `hydracache-db` suites; coordinate with TD-0001 (MSRV / sqlx transitive). |
| `reqwest` | `0.12` → `0.13` | Cluster HTTP client path; moderate breaking changes. |
| `sha2` | `0.10` → `0.11` | RustCrypto bump; low effort, may cascade with other RustCrypto crates. |
| `criterion` | `0.5` → `0.8` | Dev-dependency (benches) only — no runtime impact; lowest priority. |

2026-07-03 evaluation: `sqlx 0.8 -> 0.9` was attempted by changing the
workspace requirement to `0.9` and running `cargo update -p sqlx`. Cargo failed
before lockfile resolution because `sqlx 0.9.0` no longer provides the
`runtime-tokio-rustls` feature used by `hydracache-db`, `hydracache-sqlx`, and
`hydracache-sandbox`; `cargo info sqlx@0.9.0` also reports `rust-version:
1.94.0`, while HydraCache declares MSRV `1.88`. Next unblock condition: make an
explicit MSRV decision for Rust `1.94` or newer, then map the old SQLx runtime
TLS feature to the new `runtime-tokio` plus `tls-rustls-*` feature set and rerun
`cargo test -p hydracache-sqlx -p hydracache-db --locked`.

Each lands as its own PR, green under `cargo xtask verify` and the named suites.

### Bucket C — Blocked / do not migrate now

| Crate | Flagged | Why not |
| --- | --- | --- |
| `sled` | `0.34.7` → `1.0.0-alpha.*` | **alpha** of a durable storage engine — unacceptable for production data. `0.34` is the de-facto stable line and is what the 0.43 `sled-log-store` gate validates. A future default-engine decision (sled vs rocksdb vs redb) is strategic storage work, not a version bump. |
| `sea-orm` | `1.1.17` → `2.0.0-rc.*` | **release candidate**, not stable. Stay on `1.1.x` until `2.0` ships stable. |
| `protobuf` | `2.28` → `4.x` | **Transitively pinned by `raft 0.7.0`** (the `protobuf-codec` feature pulls `protobuf 2.x`; no newer `raft` is published on crates.io). Cannot be bumped in isolation. This is the `RUSTSEC` debt tracked in **TD-0002** — resolved by the raft-layer decision (upgrade raft via the TiKV monorepo / change codec / replace raft), not by touching `protobuf`. |

## Definition of done (per action)

- Bucket A: `cargo update` applied, `cargo xtask verify` green, `Cargo.lock`
  committed.
- Bucket B item: dedicated PR, named integration suites green, `cargo deny check`
  clean, `cargo xtask verify` green; if the bump clears an advisory, the matching
  `deny.toml` ignore is removed.
- Bucket C: no action beyond keeping this table and TD-0002 current; re-evaluate
  when a stable release ships or the raft-layer decision lands.

## Related

- `TD-0002-raft-protobuf-advisory.md` — the `raft 0.7` / `protobuf 2.x` advisory
  that blocks the `protobuf` upgrade.
- `TD-0001-msrv-pinned-sqlx-transitive-dependencies.md` — interacts with the
  `sqlx 0.9` bump (MSRV / transitive pins).
- `docs/GATES.md` — the gates every upgrade must pass.
- `deny.toml` — advisory ignores and the supply-chain gate.
