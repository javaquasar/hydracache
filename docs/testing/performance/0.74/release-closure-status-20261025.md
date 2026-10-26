# Release 0.74 closure and server launch status

The operator-approved [five-package closure plan](release-closure-execution-plan-20261026.md)
records an honest zero-performance-win scope without freezing C74. Compatible
security updates are authorized with Rust 1.88 and no new deny exceptions.
Original execution is to finish in existing local systemd 257; the server's
observed 255 remains incompatible with required extra-FD delivery. The
[bounded Linux CI retirement repair](linux-ci-retirement-result-20261026.md)
passes focused controls but does not promote package or server admission.

As of 2026-10-26, GET-owner is **terminally rejected for 0.74**, not waiting for
another acceptance run. The release still has zero accepted performance
proposals, no frozen C74, and no production server admission. Isolated diagnostic
experiments have run on the dedicated host; that is different from installing
the release or starting its production execution path.

Latest package result: [the exact V6 attempt](package-closure-v6-result-20261026.md)
passes the three previously failing targets, but reaches the same 780-second
guard. It completes 72/145 targets with 448 passed/zero failed cases; the
incomplete target has three passes and one missing verdict, while 72 targets
never start. Fourteen new review/dispatch controls and nine integrity controls
pass. Full package coverage remains red, independently of the repaired failures;
no unchanged retry or deadline relaxation is admitted.

The subsequent [real Git process-boundary control](git-process-boundary-result-20261026.md)
confirms distinct held launcher/native processes, with sixty passing models and
four integrity controls. It closes the diagnostic counter-scope question, not
the complete helper's CPU attribution or timeout. The old directly-created-child
counters remain valid only for that narrower scope; no native bypass or source
cache is installed and the V6 package verdict is unchanged.

The next [short-call CPU control](git-job-cpu-result-20261026.md) covers
non-launcher descendants from birth during ordinary `git show`, without stdin
holds, Git suspension or native bypass. It retains the first terminal path-query
refusal and succeeds on a separately committed identity-boundary fix. Eighty
models and the new target on both compilers pass. Two first old pipe-fixture
failures expose an elapsed-time stdout comparison and remain unchanged. The
residual includes native CPU, but per-image attribution and observer-overhead
acceptance remain open; no cache, product improvement or package pass follows.

The [subsequent pipe-fixture repair and observer attribution](pipe-fixture-git-overhead-result-20261026.md)
pass all six pipe cases on each compiler, preserving the original exact stream
assertions and both first elapsed-output failures. The sole fixed-Job 64-call
attribution completes, but block Job CPU ratios of 0.856–1.042 and wall ratios of
0.680–1.872 do not establish small overhead. Job-membership cost itself is not
measured. Attribute the saved variance before another cost candidate; no
package retry or admission follows this focused success.

The [offline variance follow-up](git-cpu-variance-result-20261026.md) partitions
Job CPU into worker/descendants and keeps external coordinator CPU separate.
It explains the third block's negative Job delta as +421.875 ms worker and
-671.875 ms descendants, not negative observer cost. All 32 combined profile observation
wall spans sum to 0.6435 ms; missing per-phase CPU/coordinator boundaries prevent
causal API attribution or overhead acceptance. Seventeen tests pass with zero
new workload calls. Preserve the old package verdict and design narrower
telemetry before considering another measurement.

The [coordinator recorder implementation](coordinator-frames-result-20261026.md)
adds six opt-in raw clock brackets without changing the default frame. All 56
model/regression cases pass, including all 36 ordinary clock-fault positions,
partial ownership, preserved operational refusals and the same deadline reads.
AST comparison joins the traced operational body to the original sealed body.
Four injected-clock examples are not native measurements. At that model-only
boundary, a first native control and overhead evidence remained required; it
included no V6, product candidate or server run.

The [first enabled native control](coordinator-native-result-20261026.md) now
passes all sixteen workers and sixty-four Git calls with complete native phase
brackets, unchanged source/HEAD and no cleanup errors. Six independent integrity
controls pass. Both arms trace the coordinator, so this closes only this build's
functional compatibility, not recorder-overhead acceptance. Saved phase data
supports the next offline attribution; package coverage and production admission
remain red, and V6 is not repeated.

The [offline capture follow-up](capture-attribution-result-20261026.md) closes
the new packet's owner/phase arithmetic with 31 passing model/regression cases
and five independent integrity controls. Block 2's +859.375 ms Job CPU splits
into +375 ms worker and +484.375 ms descendants while coordinator CPU falls.
Wait/collection and creation/decoding leaf costs remain unseparated; no candidate
or timeout change is supported. No workload or V6 is rerun.

The [separate leaf recorder](capture-leaves-result-20261026.md) now models
held-owner identity/initial observation and six log-decode boundaries. All 74
model/regression cases and five final integrity cases pass, preserving both
first fixture/verifier errors and exact original operational bodies. That model
stage ran no native leaf control. Wait/pipe-collection internals, recorder
overhead, the package gate and production admission remain open; no source
candidate is accepted.

The subsequent [first native leaf control](capture-leaves-native-result-20261026.md)
passes all sixteen workers and sixty-four Git calls with valid nested clocks
and no cleanup errors. Six integrity controls pass. The saved read/entry span
owns most recorded decode wall, while the large remaining spawn complement is
not pure creation cost. This establishes only exact-build diagnostic
compatibility; overhead, package coverage and production admission stay open.

The [offline leaf follow-up](leaf-attribution-result-20261026.md) now closes the
saved packet's worker/block projections and parent CPU partitions. All 115
model/regression cases and six independent integrity controls pass. Identical
reference log sizes have widely varying read brackets; mixed spawn CPU block
deltas do not establish causal API or instrumentation cost. The first Windows
format command-line refusal is retained. No new workload, source optimization,
V6 retry or server run occurs; the six release gates below remain unchanged.

The [separate spawn prototype](spawn-frames-result-20261026.md) now models
preparation, create-to-retention and membership/release with 130 passing cases
and five independent integrity controls. No new observation enters the successful-
create/publication gap; original CI/caller/worker bytes remain unchanged. This
unwired model does not establish native cost, recorder overhead or admission.
Its next boundary is explicitly gated integration, not another package retry.

The [first integrated native spawn control](spawn-capture-native-result-20261026.md)
passes its exact 16-worker/64-call healthy path through a separate gate with
unchanged old modes. All 142 models and seven integrity controls pass, preserving
first fixture/reader failures. New span ordering and CPU arithmetic close without
an ownership-gap probe; healthy compatibility is not native fault/overhead or
product admission. No V6 or dedicated-server action is performed.

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

The [subsequent Windows CI cleanup repair](ci-watchdog-owned-cleanup-result-20261026.md)
passes the unchanged old descendant-marker case and all twelve old cases on each
compiler. Nineteen deterministic cases and separate native first-result controls
cover owned Jobs, creator loss before resume/while writing, setup/error refusal
and stream/argv compatibility. This closes the Windows helper failure on changed
source, not the interrupted package. POSIX early-leader-exit cleanup remains a
separate gap. Existing validation-cost attribution and a new exact complete
source/effect review are still required before another full command.

The [next history/status compatibility repair](history-status-readers-result-20261026.md)
closes those two additional reader failures locally, preserving all twelve old
cases/assertions and adding exact construction/refusal controls. This does not
replace the full result below. A separate held-object taskkill diagnostic records
terminal original root/leaf objects and an existing marker; without write/cut
chronology it does not close the watchdog failure. Owned cleanup, existing
validation cost and a new complete exact source/effect review remain before any
changed-source full dispatch. That earlier observation does not substitute for
the Windows repair above; old source pins must continue refusing the new graph.

The [subsequent v5 exact review and full result](package-closure-v5-result-20261026.md)
cover 365 source identities/42 auxiliary inputs and the 144-target catalog.
The first unchanged-bound full command still times out: 76 targets complete,
457 cases pass/three fail; the incomplete four-case target has three passes and
one missing verdict; 67 targets never start. First raw evidence and cleanup errors
remain. The focused follow-up above closes those reader consumers, not this full
result. Investigate owned watchdog cleanup and attribute existing validation cost
before a new exact source/effect review and full run. No unchanged-source retry
or limit increase.

The [first current-source full package gate](package-closure-v4-result-20261025.md)
is retained: unchanged 780-second/four-thread run, 114/138 targets completed,
777 passed/46 failed cases in completed targets. The incomplete performance target
adds 104 passing/30 failing observations; 23 targets are unstarted. This is still
red. Its exact repair frontier and raw/offline replay are linked in that report.

The [separate inventory phase repair](inventory544-phase-result-20261026.md)
closes the 544→530→446 inventory boundary with 14 targeted cases per compiler.
Its first changed-source twenty-case integration has thirteen failures at its
downstream body/COMPAT views. The [subsequent exact source successor](source-successor-view-result-20261026.md)
closes those failures: all twenty unchanged artifact cases pass, with separate
whole-body reconstruction and refusal controls. Old ledgers/assertions and the
first red package remain unchanged. Complete the other authority/current-tree and
bootstrap boundaries and a new source review before another package.

The [subsequent authority reader phase](authority-reader-phase-result-20261026.md)
closes five further failures in its seven-target subset, preserving all 22 old
cases and adding six routing/predicate controls. Five unchanged successor controls
also pass on both compilers. The complete current-tree audit and remaining
bootstrap inputs still need their own exact accounting; this does not replace
the failed/interrupted full package.

The [bootstrap document-reader repair](bootstrap-doc-reader-result-20261026.md)
subsequently closes fifteen bootstrap failures with 69 cases per compiler:
53 old bootstrap, ten preceding proof and six new preservation/refusal controls.
It changes only the two exact document views, retaining every old assertion and
the first lint/compile refusals. At that boundary the independent audit remains
red at 26 inputs, addressed by the following separately tested repair.

The [current-tree frontier successor](current-frontier-result-20261026.md)
subsequently closes all 26 audit inputs using fixed independent producers and
unchanged protected selection. Twenty-one cases pass per compiler, including
the complete audit and old 9/81 controls. Other current reader targets and a new
complete source/effect review remain; this is not a full package pass.

The [remaining reader repair](remaining-readers-result-20261026.md) subsequently
closes fresh-kernel and activation-ingress's two current failures. The three
other selected reader targets already pass and are rerun as controls. Sixty-one
cases pass per compiler, including the complete audit and old proof chain.
No validator/ledger is relaxed. The next boundary is a new exact whole-package
source/effect review, followed by the complete unchanged-bound package command.

The [contextual module inventory](module-context-v5-result-20261026.md) then
closes the current graph at 365 Rust sources, preserving the old scanner's first
two-edge refusal. Twenty-three controls and seven offline integrity controls
pass. The 21 changed/21 added AST rows and auxiliary data still require manual
effect review; no new package run or admission is inferred from source closure.

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
