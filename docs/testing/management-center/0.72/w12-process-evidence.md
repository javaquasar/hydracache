# W12 process, fault, soak, and resource evidence

This work item adds executable proof machinery; it does not synthesize the six-hour candidate or
24-hour ship receipts. Those receipts must be produced from the frozen candidate on the admitted
host and remain W14 release inputs.

## Fast and real-process evidence

- The structural `management_process_072` suite checks all 13 mandatory source-to-management fault
  rows, resolves every registered `path::test` to a real function, tests fail-closed projection,
  deterministic framed SHA-256 digests, append-only retry chains, and all four W12 canaries.
- With `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1`, the one-daemon test starts the production binary,
  resolves the content-hashed module from the served production `index.html`, reads that exact
  embedded asset, proves the bundle sends `management.read` rather than write-admin, exercises all
  management sections, and preserves unknown recovery without a retained source.
- The three-daemon test forms quorum, kills a follower, positively observes partial management
  truth, races 32 readers after cache expiry, proves only bounded 200-partial/429 outcomes, verifies
  write-admin status remains responsive, restarts the same disk, observes recovered membership,
  polls 32 more times, checks p95/FD/RSS budgets, and writes then rereads a binary-bound receipt.
- The fixed seed is `0x0720120000000001`. Schedule and normalized event streams use length-framed
  SHA-256 so concatenation ambiguity cannot change their identity.

Commands used locally:

```powershell
cargo test -p hydracache-server --test management_process_072 --locked
$env:HYDRACACHE_RUN_DAEMON_PROCESS_E2E='1'
cargo test -p hydracache-server --test management_process_072 --locked one_daemon_production_management_surface_is_typed_and_honest -- --nocapture
cargo test -p hydracache-server --test management_process_072 --locked three_daemon_fault_recovery_retains_partial_truth_and_resource_bounds -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_DAEMON_PROCESS_E2E
```

## Retained source proofs and truth boundary

`fault-matrix.toml` links existing durable-runtime, corruption, truncation, ENOSPC, snapshot,
identity, reconciliation, stale-peer and resource tests instead of cloning weaker versions. The
current standalone server does not retain HydraCache's generic `RecoveryReport`; consequently disk
fault rows project as `unknown/status-not-retained`, never `Clean`, PASS, zero loss, or a guessed
exact cause. Commit/apply lag, partial peer collection, deletion subset and overload have direct
management projections. Adding a richer durable diagnosis requires connecting a retained typed
source and changing the source map, tests and canaries together.

## Scheduled/candidate/ship evidence contract

The source commit freezes the scenario tiers and receipt schema. Scheduled 3/5/7-daemon churn,
Prometheus and disk faults, the six-hour candidate poll/load run, the 24-hour ship confirmation and
rolling upgrade/rollback must record the exact candidate binary/UI/schema/SBOM hashes, host
fingerprint, seed, external schedule, activation receipts, redacted logs/traces, resource series,
  endpoint histogram and every linked attempt. A missing environment is a blocking structured result,
  not success; a retry cannot overwrite its predecessor. Shared Windows execution proves structure
  and the short real-process path only, not Linux numerical ship budgets.

The durable source subset was also executed with
`cargo test -p hydracache-cluster-raft --features sled-log-store,test-failpoints --test
snapshot_corruption --test durable_recovery_corpus --test failpoints_crash_safety --locked`: 15
tests were non-empty and green. Running those files without their declared features yields an empty
binary and is not acceptable evidence; candidate automation must keep the feature set and reject an
empty shard.
