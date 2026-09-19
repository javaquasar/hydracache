# W12 process, fault, soak, and resource evidence

This work item adds executable proof machinery; it does not synthesize the six-hour candidate or
24-hour ship receipts. Dedicated external gates produce those receipts from the frozen candidate
on the admitted host and keep them as W14 release inputs.

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
- Scheduled/tag CI runs these proofs only through the dedicated
  `env.hydracache-run-management-process-072` and
  `env.hydracache-run-management-resource-linux-072` gates. The latter is Linux-only and owns the
  FD/RSS numerical claim; receipts from the older generic daemon/resource targets cannot satisfy
  W12. The taxonomy row is `covered` because this executable proof path is implemented; promotion
  still requires a green exact-SHA receipt from that gate.

Commands used locally:

```powershell
cargo test -p hydracache-server --test management_process_072 --locked
$env:HYDRACACHE_RUN_MANAGEMENT_PROCESS_072='1'
cargo test -p hydracache-server --test management_process_072 --locked one_daemon_production_management_surface_is_typed_and_honest -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_MANAGEMENT_PROCESS_072
$env:HYDRACACHE_RUN_MANAGEMENT_RESOURCE_LINUX_072='1'
cargo test -p hydracache-server --test management_process_072 --locked three_daemon_fault_recovery_retains_partial_truth_and_resource_bounds -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_MANAGEMENT_RESOURCE_LINUX_072
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

The candidate and ship tiers are executable, non-overridable wall-clock tests:

- `env.hydracache-run-management-candidate-soak-072` runs exactly six hours;
- `env.hydracache-run-management-ship-soak-072` runs exactly 24 hours;
- both start three production daemons with RESP, poll the typed dashboard every second, issue HC/1
  writes and RESP traffic every second, restart a follower hourly, require visible partial truth and
  full recovery, enforce p95/FD/RSS ceilings, and retain a binary-bound JSON artifact;
- the paired `tool.hydracache-server.management-hc1-hc2-coexistence-072` gate starts the same exact
  candidate as a real process and proves HC/1 plus mTLS HC/2 shared-dispatch traffic and clean drain;
- the ship workflow first repeats the candidate gate and then runs the ship gate in the same job on
  the same labelled self-hosted Linux runner. The hashed `HYDRACACHE_RELEASE_HOST_ID` is retained in
  both artifacts, and the two standard evidence receipts bind them to the same source SHA;
- ordinary nightly runners cannot shorten either duration. Both gates are `ship_mandatory` and are
  required by W12, so absent self-hosted execution remains a red admission result.

The source commit freezes the scenario tiers and receipt schema. Scheduled 3/5/7-daemon churn,
Prometheus and disk faults and rolling upgrade/rollback must record the exact candidate
binary/UI/schema/SBOM hashes, host
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

## Candidate-campaign hardening record (2026-09-19)

The first dedicated-host attempts exposed four harness defects. They were fixed in the test
machinery rather than waived, and each failed attempt remains historical evidence. These changes
do not weaken any release threshold:

| Commit | Observed failure | Permanent correction | Required proof after correction |
| --- | --- | --- | --- |
| `9d980f6c` | A combined startup probe reported only a generic readiness timeout. | Dashboard, HC/1 and RESP readiness are established independently and retain the last protocol-specific error. | All three surfaces must become ready; failure of any one surface fails the gate with actionable diagnostics. |
| `bedb712a` | The soak topology generated HC/1 traffic without enabling the production HC/1 listener. | The three-daemon soak fixture explicitly enables the HC/1 listener on the observer. | Readiness and every-second HC/1 writes must use the listener started by the exact candidate binary. |
| `3b7b25f8` | The raw socket probe used half-close behavior that was not a faithful long-running HTTP client. | One bounded, reusable `reqwest::Client` now drives HC/1 traffic and consumes complete responses. | The same client must pass readiness and sustained writes for the entire soak; transport errors fail the run. |
| `ad9fd2b8` | Chaos Mesh could retain an IOChaos deletion finalizer beyond the former 60-second cleanup window on the admitted host. | Cleanup keeps the same exact-object and recovered-pod assertions but permits a bounded 120-second controller reconciliation window. | The IOChaos object must disappear and the replacement target must become ready inside 120 seconds; timeout remains a hard failure. |
| Run `35457057202` | The six-hour workload completed, but the final assertion expected a recovery at the exact exclusive deadline (`6h`) even though the loop executes only events strictly before that deadline. Five hourly recovery cycles completed; the assertion incorrectly required six and discarded the receipt. | The expected-cycle calculation now derives the number of scheduled fault instants strictly before the deadline and has boundary tests for zero, one, six and 24 hours. | Candidate requires five completed hourly cycles at hours 1-5; ship requires 23 at hours 1-23. A missed scheduled cycle still fails, and no event after the wall-clock deadline is counted. |

The readiness split is intentionally not a retry mask: every surface has its own bounded deadline,
last error and hard failure. The HTTP-client change proves production protocol behavior instead of
accepting a TCP connect. The IOChaos change adjusts only the observation window; it does not accept
an uncleared finalizer, a stale target, a different pod or an unobserved recovery.

The recovery-count correction likewise does not shorten either soak or remove a fault. The loop's
deadline is exclusive: for an exact six-hour interval, the scheduled instants inside the run are
hours 1-5; hour 6 is the deadline itself. For 24 hours they are hours 1-23. The receipt still
requires the full observed duration and every scheduled pre-deadline recovery.

The candidate run started from `ad9fd2b813b55fe44e70c2cea692495496e50ff2` is evidence for that
exact source commit only. A later documentation commit must not relabel or transplant its receipt.
If the release candidate includes later documentation changes, all exact-SHA admission receipts
required by W14 must be regenerated for the new clean commit. Attempt IDs, workflow URLs, runner
identity, logs and artifacts belong in the generated receipt chain rather than being copied into
this durable design note.
