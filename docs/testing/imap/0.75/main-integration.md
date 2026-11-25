# Main integration for release 0.75

The latest integration on 25 November 2026 merges main
`3453c2e7b1b2b1e7806d7b689a08b3f51275f614` at
`87309fb4483c19dfa701882446453dabbc02d1d4`. It imports the Rust 1.88-compatible
`kube-client` OpenSSL CA backport while retaining the admitted supervisor dependency graph.
The [D1 production extraction record](d1-local-backend.md) describes the continuing work.
The earlier integration history and its test results below remain tied to their original source.

The release owner requested integration of current main on 25 November 2026.
The integration combines the 0.75 branch at
`c8109470ce860179a955a6fd02f55e25e3108370` with main at
`6d41962292363fc6c84eaed1d7b1179f1915451d`.
The expanded code checks ran on 0.75 with runtime base `e6ccc2563`; the final main
advance records registry publication in three documentation files. The follow-up integration
rechecks documentation, governance, the IMap contract and frozen performance-schema hashes.

## Predecessor identities

| Input | Recorded identity | Use in 0.75 |
| --- | --- | --- |
| Stable maintenance release | `v0.74.0`, source `bcba511f0c1f4b3e965c9e17f81b248deb0ea331`, binary published 23 November 2026 | B74 behavior and signed binary for compatibility and D1 remeasurement |
| Published registry maintenance source | `e6ccc2563db20efb0c59818f0692c580ea4ad504`, tagged `v0.74.1` | Source and dependency base for development |
| Current main with registry publication admission | `6d41962292363fc6c84eaed1d7b1179f1915451d` | Records 0.74.1 as shipped; no code or dependency change from the tagged source |
| Existing 0.75 Linux qualification | `de4bc2a0b32ff8acffa8128f10c603316dca9a05` | Historical foundation proof for its exact source |

The stable release notes record signed-binary compatibility, rolling upgrade/rollback and a
six-hour maintenance run on AX42. They also record that paired W11 performance qualification
remains open. The 0.74.1 notes describe registry dependency admission and rolling compatibility
for reviewed forks. The integrated workspace inherits their exact dependency choices.
[0.74.0 release notes](../../../releases/0.74.0.md).
[0.74.1 release notes](../../../releases/0.74.1.md).

## Integration scope

Resolve the lockfile against the merged manifests, using main's lock as the initial input.
The resulting lock retains all 678 main package identities and checksums and adds 19 packages
for the 0.75 tools and testkit. A final fetch found the documentation-only 0.74.1 publication
admission, which was integrated after the expanded code checks.
Keep the existing 0.75 testkit, proof workers, Java campaigns and rollback rehearsal.
Bring in the server TLS/configuration fixes, native-path guards, simulator and Raft validation,
registry dependency aliases, and current CI/docs gates.
The compatibility register retains the 0.75 provisional entries and the 0.74 maintenance evidence.

The integration also registers the eight Linux-only supervisor test targets in the existing gated
test registry and fixes LF checkout for the W11 schemas whose SHA-256 values are frozen in the
controller contract.
The two internal campaign tools inherit the workspace's Apache-2.0 license explicitly.
Profile timing tests use thread-local, unit-test-only elapsed samples (zero and 37 ns) to verify
exact accumulation, disabled instrumentation and reset. The request-path test retains its
operation/copy/counter assertions and accepts zero-duration real-clock observations. Production
timing and coverage thresholds are unchanged.

## Local integration checks

On Rust 1.94.0 and Windows, `cargo check --all-targets --locked --offline` passed for
`xtask`, `hydracache-server` and `hydracache-cluster-testkit`.
Targeted suites passed 601 tests across the client transport, server, IMap testkit and xtask.
The client transport also reports two scheduled retention tests as ignored.
The performance-schema, release-governance and documentation suites passed after the checkout
and registry corrections.

The expanded workspace run (excluding xtask) passed 3,547 tests and doctests, with 82 existing
gated tests ignored. The complete xtask library/integration run passed 946 tests; four
actual-artifact qualification tests remain ignored until their current-source preparation exists.
After integrating the final main publication admission, 89 targeted documentation, governance,
IMap-contract and frozen-schema tests passed again.
The timing fix also passed package `cargo check --all-targets` and
`cargo clippy --all-targets --all-features -- -D warnings`.

The full Windows `cargo xtask verify` passed documentation, feature-leak, formatting,
workspace/common-allocator Clippy lanes and dependency bans. The separate license check
passed after the internal tool metadata correction. Verification stopped at dependency policy: the Linux-only
supervisor dependency `zbus 5.19.0` brings in `zvariant`, `zbus_names` and
`zvariant_derive`, now covered by RUSTSEC-2026-0370, RUSTSEC-2026-0371 and
RUSTSEC-2026-0372 (unmaintained). No advisory exemption was added.

The [upstream zbus 6 migration guide](https://z-galaxy.github.io/zbus/upgrading-to-6.html)
requires Rust 1.89 and removes the blocking API used by `systemd_unit.rs`. A blind
dependency bump would invalidate the retained Rust 1.88 matrix lane. Before release,
replace the D-Bus client with an admitted MSRV-compatible implementation, or explicitly
revise the toolchain contract and migrate/test the asynchronous API. The release gate
remains open until dependency policy and the subsequent full verification stages pass.

Run the existing backend and daemon checks through these entry points.

```powershell
cargo test --locked -p hydracache-client-transport-axum --lib --tests
cargo test --locked -p hydracache-server --test imap_dispatch_characterization_075 --test hc2_daemon_process --test redis_mtls_074 --test server_lifecycle
cargo test --locked -p hydracache-cluster-testkit --lib --test imap_backend_contract_075 --test value_plane_model_075 --test value_plane_online_transfer_075 --test value_plane_cluster_runtime_075
cargo test --locked -p xtask --test imap_contract_075 --test imap_pre074_075 --test imap_ci_075 --test performance_contract_074 --test release_governance --test doc_check
cargo test --workspace --exclude xtask --locked --offline -j 1 --no-fail-fast
cargo test --locked --offline -p xtask --lib --tests --no-fail-fast
```

## Next product steps

The subsequent [supervisor dependency admission](supervisor-dependency-admission.md) replaces
the unmaintained graph while preserving Rust 1.88. The blocked verification above records the
initial integration; the follow-up retains its own checks and W12 evidence.

1. Bind the retained B74 binary, package and compatibility evidence to the published tag.
   Capture the dependency identity of the integrated 0.75 tree separately.
   Resolve the supervisor dependency-policy blocker above before the release qualification run.
   The subsequent [D1 B74 intake](d1-b74-intake.md) records the verified signature and isolated
   smoke and complete raw maintenance replay; compatibility replay and paired measurements remain open.
2. Freeze W0's remaining owner decisions in the release ledger. OD-1's selected read barrier is
   recorded in [ADR-0024](../../../adr/0024-imap-quorum-backed-read-barrier.md); its implementation
   and cost admission remain open. OD-2 needs an admitted time provider and uncertainty bound,
   OD-3 needs recovery targets,
   and OD-6 through OD-10 need their scope, latency, routing, max-idle and schedule decisions.
3. Start D1 with a mechanical extraction of the production local backend. Reuse the existing
   daemon characterization and backend contract tests. Compare the bounded dispatch variants
   through local, direct-native and RESP entry points against the remeasured B74.
4. Continue D2-D5 with canonical key/partition identities, the admitted clock provider, the
   partition executor, authenticated owner invocation and synchronous replica proof. Close the
   three-daemon skeleton gate before widening the operation families.
5. Run the Linux W12 matrix on the committed integrated tree. Retain its own four source-bound
   receipts and coverage reports, then proceed through the D6-D16 product and release gates.

The preparation commands named `imap-pre074-*` retain their existing fixture and receipt format.
The machine status now records the published source and the remaining evidence admission work.
W0 closure and production capability admission continue through their executable gates.

## Retained local integration records

Logs and `lock-integration.json` are under `target/imap-075/integrate-main-20261125/`.
The passing expanded run is `workspace-tests-recheck.log`; the full xtask run is
`xtask-all-tests.log`. These are integration checks, not W12 qualification receipts.

The original 0.75 tip is retained by `backup/075-before-main-20261125-main` and the complete
bundle `target/imap-075/integrate-main-20261125-main/pre-merge.bundle`.
`git bundle verify` passed for that bundle.
