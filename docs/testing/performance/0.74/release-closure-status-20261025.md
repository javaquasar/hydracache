# Release 0.74 closure and server launch status

As of 2026-10-25, GET-owner is **terminally rejected for 0.74**, not waiting for
another acceptance run. The release still has zero accepted performance
proposals, no frozen C74, and no production server admission. Isolated diagnostic
experiments have run on the dedicated host; that is different from installing
the release or starting its production execution path.

The [approved plan](../../../plans/V0_74_RESP_THROUGHPUT_AND_PIPELINE_EFFICIENCY_PLAN.md)
keeps its original thresholds. Its closing rule is to publish the measurements
and defer changes if none pass, not to tag a documentation-only substitute as
the measured C74. This status does not authorize a new experiment, dependency
change, host upgrade, installation, maintenance or qualification run.

## Preserved experiments and implementation

All existing evidence, first refusals and analysis remain in the development
branch `feat/0.74-resp-native-throughput`. The
[proposal registry](proposal-registry.toml),
[terminal W2 through W9 ledger](terminal-disposition-ledger.toml),
[local attribution](w1-local-attribution.md) and
[optimization article](../../../articles/007-optimization-starts-with-measurement.md)
are the entry points for earlier hypotheses. A screened or unauthorized idea is
not relabeled as an implemented experiment. Previously committed candidates
remain recoverable from Git history even where their runtime integration was
selectively reverted.

| Experiment | Implementation preservation | Analysis and current disposition |
| --- | --- | --- |
| GET-owner real-network D3 | Archive branch `archive/0.74-get-owner-d3-measured-be0499a8c` pins exact measured source `be0499a8cb0a2ad0b7328d6ac0cf6227114cebcc`, including the active on/off runtime and v2 worker. Coordinator source is separately pinned at `53b99dea6eb56f437b0ed765db293169f91858e4` in development history and the packet. | [Terminal result](get-owner-full-d3-terminal-result-20261025.md), preregistered contracts and [raw packet with offline verifier](local-runs/get-owner-full-d3-network-allocation-20261025/README.md). All 240 processes finish; every mandatory mTLS allocation cell fails the original 20% floor. |
| GET-owner retained helper | `crates/hydracache-redis-compat/src/get_response_owner_074.rs` remains test-only on the development branch, with historical unit/property guards. | Runtime selection is removed; the empty Cargo feature is a compatibility tombstone, not activation. Timing/full-retention admission work stops after the mandatory rejection. |
| Rustls security experiment | Archive branch `archive/0.74-rustls-security-e02448617` pins `e02448617c7073e499c1d6ddbf7d3330518db0ae`, including the patch and regression test. | [Historical result](rustls-security-result-20261024.md) and [dependency plan](dependency-security-plan-20261024.md). Reverted under the operator's audit-only instruction; not a currently installed security fix. |
| Earlier product candidates | Committed versions remain in development history. W2 precedes its selective revert `c4e893f39`; W3 includes `2c971cee1` and the separate TCP candidate `4f9d7035d`; W6a precedes `dc9989b52`. | Exact decisions, negative measurements and non-authorized branches of the plan remain in the registry/terminal ledger above. Adaptive W3 was never committed as product code; do not invent a preserved executable version for it. |
| Launch-control and attribution tools | Current source stays under `tools/`, with source-pinned historical inputs and controls under `local-runs/`. | The [six-step continuation](release-six-step-continuation-20261025.md) links the individual preparation, census, capture, package and observer reports. These are tooling findings, not accepted cache throughput improvements. |

The archive branches are historical source references, **not release candidates**.
Do not merge their retired runtime or reverted dependencies back into the product
to preserve them. Reading a historical file needs no checkout:

```text
git show archive/0.74-get-owner-d3-measured-be0499a8c:crates/hydracache-redis-compat/src/get_response_owner_074.rs
git show archive/0.74-rustls-security-e02448617:crates/hydracache-redis-compat/tests/tls_session_security_074.rs
python docs/testing/performance/0.74/local-runs/get-owner-full-d3-network-allocation-20261025/verify.py
python docs/testing/performance/0.74/local-runs/get-owner-full-d3-network-allocation-20261025/test_verify.py -v
```

The Python commands replay retained evidence offline; they do not rerun a
campaign. The packet includes public receipts and hashes, not private diagnostic
keys or built ELF files. Rebuilding or rerunning historical sources requires a
separately scoped worktree, protocol and applicable host authorization; archive
publication itself grants none of those permissions.

## Six remaining release gates

### 1 Choose the release scope and freeze the product

The [composition ledger](composition-ledger.toml) correctly keeps
`accepted_candidate_count=0`, `freeze_c74_allowed=false` and C74 `UNRESOLVED`.
GET-owner no longer supplies a pending route to acceptance. RESP mTLS is an
implemented security prerequisite, not a measured performance win.

Decide whether to close the performance work as published negative findings and
defer product changes, or explicitly approve a different release scope. Only
then review the complete runtime/features/dependencies delta, update the scope
through its normal contract process and freeze an exact clean source/package
identity. Do not reopen rejected hypotheses merely to populate a release.

### 2 Obtain a complete green package and security milestone

The [first current-source full package gate](package-closure-v4-result-20261025.md)
is retained: unchanged 780-second/four-thread run, 114/138 targets completed,
777 passed/46 failed cases in completed targets. The incomplete performance target
adds 104 passing/30 failing observations; 23 targets are unstarted. This is still
red. Its exact repair frontier and raw/offline replay are linked in that report.

Complete a newly reviewed whole-package replay within the unchanged
780-second/four-thread guard, including the previously unstarted suffix. The
separate 321-case performance-contract pass and repaired focused cohorts do not
complete the interrupted package. Refresh the exact source/effect review for the
final tree; retain the old reviews' correct stale-source refusal. The actual
Cargo target/output-lifetime adapter still needs its reviewed real caller and
dispatch gate, as recorded in the six-step continuation.

The last full milestone stops at fifteen dependency advisories. Dependency
changes remain **audit and planning only**. Approve separate repairs/migrations
for TLS/WebPKI/Tokio, YAML, ZIP and D-Bus with an explicit Rust 1.88 strategy;
the audited zbus 6 metadata requires 1.89. No deny exception or hidden MSRV
override is an alternative. Run complete formatting, lint, tests, supported
target/feature, dependency/license and packaging gates on the final source.

### 3 Complete original execution and all writer retirement

The present original `HeldDelivery` ends at `ReadyObserved` or refusal; the
bootstrap explicitly withholds worker release/executable handoff. Finish original
preparation within unchanged 300/60/10-second origins/windows and actual manager
delivery of all eighteen roles under the same consumed owner/full validation
frame. Prove executable, streams, process, cgroup, invocation, correlated reply
and authenticated child identity; readiness alone is insufficient.

Select an explicitly approved compatible manager/host environment. The retained
audit finds systemd 255 on the dedicated host and no upstream transient extra-FD
setter in 255/256, versus its presence in 257. This is not a live certificate of
distribution capabilities. Do not silently upgrade, weaken descriptor custody or
substitute another manager. Then integrate original worker release with the
autonomous watchdog and every writer/FD alias. Owner loss, hang or expiry must
prove cessation of all writing before completion or fence release.

### 4 Finish lifecycle and prove crash and reconnect behavior

Join durable completion to that retirement proof, implement cells 2–4 and legal
adjacent transitions, and handle uncertain attempts without replay. At every new
publication/fsync/ACK boundary run the complete real SIGKILL/lost-ACK matrix,
including guardian loss and duplicated writers. Keep initial results and
deadlines; a kill request or leader exit is not writer-retirement evidence.

On the **new original execution path**, disconnect SSH/controller, keep the
attempt autonomous and reattach to that same attempt. Require unchanged
PID/start ticks, boot/cgroup/invocation and continuous checkpoints. Existing
snapshot observation and finite diagnostic disconnect controls do not close
this production integration gate.

### 5 Enroll the real host and prepare the exact installation

Approve the production policy body, digest/epoch, issuer keys, independent
receiver/workflow pins and protected configuration. Bind enrollment to the exact
build receipt and original context; the response cannot provide its own trusted
pins. Prove directory ownership, UID/GID and systemd/cgroup preflight, with no
active campaign or lifecycle fixture. Synthetic signing/enrollment controls are
not production host admission.

### 6 Rehearse then qualify and publish the frozen candidate

After gates 1–5, build and sign the exact bundle, verify its receipt, and obtain
separate installation/maintenance agreement. Run bounded nonproduct
lifecycle/fault rehearsal on the admitted host and verify supervisor restoration.
Only after that gate admit a limited measurement pilot. The earlier approved
GET-owner maintenance is complete, not blanket permission to install a release.

For a product release under the present plan, finish matched native/RESP,
same-box Redis, compatibility/rollback and interaction gates, followed by
separately admitted six-hour qualification and 24-hour confirmation on the same
frozen C74. Publish package hashes, SBOM, canaries, release evidence/notes and
rollback instructions from the actual results. The unchanged
[qualification manifest](qualification-manifest.toml) remains
`prepared-not-authorized`, with expensive runs disabled and candidate unresolved.

## Immediate decisions and next implementation boundary

The next decisions are release scope, dependency/MSRV remediation authority and
the supported manager environment. The next execution implementation boundary
is original worker release joined to watchdog/all-writer retirement, not another
GET-owner benchmark. Full-package source review and nonexecuting integration
work can continue independently, but cannot turn those decisions into consent.

The GET-owner maintenance restored the stable supervisor, verified its installed
binary/receipt and retained the recovery record. No release installation or
production enrollment was performed. The frozen 0.73 artifacts, live-check
frequency/order, initial time origins and numerical thresholds remain unchanged.
