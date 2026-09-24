# HydraCache 0.73 local performance screening

`local-screening-073-v1` is the first W0 contract. It makes local development evidence reusable
without allowing a laptop, workstation, WSL, or shared runner to create a release performance
claim.

Validate the checked-in contract and example:

```text
cargo xtask performance-contract-check --release 0.73
```

Capture a privacy-safe local context before screening a candidate:

```text
cargo xtask performance-local-context --release 0.73
```

The generated `target/performance-evidence/0.73/local/context.json` binds the exact source SHA,
dirty-tree state, OS/architecture, logical CPU count, and hashed CPU/toolchain identities. Raw CPU
model text, hostname, username, home path, serial numbers, and network addresses are not retained.
The fingerprint covers only stable identity: timestamps and source state do not silently turn the
same host into a different machine.

`baseline-identities.toml` distinguishes the annotated/peeled published `B72`, the post-publish
branch root `R73`, and the still-unfrozen `I73`. `post-tag-delta.toml` classifies every path between
`v0.72.0` and `R73`; the checker recomputes that Git diff and rejects missing, stale, or status-mismatched
entries. The current ledger has no product-runtime or production-instrumentation path, so none of the
post-tag changes is silently treated as an optimization baseline.

`scenario-matrix.toml` preregisters the complete W2-W9 surface and wave coverage, concurrency/load
lanes, mixed-runtime weights and evidence invariants. It intentionally remains in `pilot` state with
stable rates and measurement windows marked `unmeasured`; candidate measurement is forbidden until
baseline-only calibration and production-instrumentation overhead evidence freeze `I73`.

`instrumentation-overhead.toml` freezes the off-versus-production design and inherits the 2%
goodput and 3% CPU/request and p99 regression ceilings. Profile mode remains classification-only.
Allocation and RSS limits are deliberately unmeasured blockers: baseline-only evidence must freeze
them before five admitted qualification pairs can authorize `I73`.

A bounded plumbing pilot can exercise each mode locally without producing an overhead claim:

```text
cargo run -p hydracache-loadgen --locked -- memory-efficiency --profile memory-efficiency-v1 --provider system --instrumentation-mode <off|production|profile> --output-dir target/performance-evidence/0.73/local/instrumentation-pilot/<mode>
```

This pilot proves the mode adapters, ordered phase timeline and artifact writing only. Its debug-build
elapsed time is not a stable rate, threshold, qualification pair, or input to an `I73` freeze.
With profile `instrumentation-overhead-073-v1`, each phase also emits a schema-validated
`resource-series.jsonl` containing gross allocation bytes per operation and self-process RSS/peak
RSS. The legacy `memory-efficiency-v1` output shape remains unchanged.

After building the loadgen once and generating a clean context, run three local counterbalanced
pairs with `cargo xtask performance-overhead-screen --release 0.73 --context <context> --binary
<loadgen> --output <new-directory> --pairs 3 --seed <seed>`. The output directory is append-only;
every subprocess retains stdout, stderr, receipt and resource-series digests. `screening.json`
contains the per-mode distributions but explicitly marks thresholds `screening_only_unqualified`.

To attribute the mutation/RSS deltas without paying for a dedicated-host run, repeat the same local
screen with `--profile instrumentation-overhead-counters-only-073-v1`. This development-only
variant retains the production counters but omits the async eviction listener. Its receipts are
marked `diagnostic_only: true` and `counter_correctness_eligible: false`: removal accounting is
intentionally incomplete, so the result may locate overhead but cannot prove correctness, freeze
thresholds, or contribute to `I73`.

The narrower `instrumentation-overhead-listener-noop-073-v1` diagnostic keeps the backend listener
registered but replaces HydraCache's callback with a no-op. Comparing it with the counters-only
profile separates backend notification cost from retained-byte accounting and async tag cleanup.
It is likewise ineligible for correctness, thresholds, or promotion.

The first three-pair release-build screen is retained in
`local-overhead-screening-307b3500.toml`. It found no elapsed-time regression and no steady-read
allocation delta, but it did find material mutation allocation and RSS deltas. The result is a
negative, non-promotable blocker—not a threshold proposal. The next step is to isolate the
production-only async eviction-listener cost before repeating the screen.

That isolation is retained in `local-overhead-isolation-5264d96c.toml`. With production counters
enabled and only the listener removed, median fill and expire/delete allocation deltas fell to zero;
post-idle and peak RSS deltas fell to +1.4% and -1.8%. Refill retained a small +3.3% delta. The
elapsed median moved +7.4%, but one fast production sample makes three short local pairs inadequate
for a timing claim. The result attributes the large mutation/RSS cost to the listener and directs
the next implementation step; it does not validate removal-counter correctness or unblock `I73`.

The follow-up no-op-listener screen is retained in
`local-overhead-listener-noop-5e101e69.toml`. Merely registering the backend listener reproduced
fill allocation +23.9%, expire/delete allocation +25.8%, post-idle RSS +27.4%, and peak RSS +21.3%
while steady reads remained unchanged. The callback is not free, but optimizing it alone cannot
clear the blocker: the production redesign must avoid enabling Moka's listener-backed mutation path
while preserving exact automatic-removal and tag-cleanup semantics.

`proposal-registry.toml` records the resulting redesign as `D1 classified`, not `D2 authorized`.
The locked Moka 0.12.15 source shows that enabling the future-cache notifier activates per-key
locking on insertion and shared boxed notification futures on removal/update paths. The current
0.12.16 API still has no nonblocking post-removal observer. Product mutation and candidate
measurement therefore remain forbidden until a lab-only feasibility spike, recorded review, and
pre-candidate allocation/RSS limits select or reject an exact alternative.

The first backend feasibility result is retained in
`notification-feasibility-bf9f1382.toml`. In three counterbalanced repetitions, registering a
no-op listener increased median allocation per insert by 89.9% on Moka future and 72.8% on Moka
sync; per remove it increased allocation by 33.3% and 11.4%, respectively. Sync reduced the
incremental listener allocation by 24.3% for insert and 66.3% for remove, but did not eliminate the
insert penalty and would change the backend's asynchronous semantics. The result rejects a sync
migration as the instrumentation fix; it remains a diagnostic microprobe, not product evidence or
authorization for D2.

`notification-observer-requirements.toml` freezes the next lab step before implementation. A
nonblocking callback is not sufficient by itself: delayed cleanup for an old entry must not remove
tag membership belonging to a newer value under the same key. The prototype must therefore carry
an immutable entry version, perform only bounded synchronous accounting in the observer, apply
deferred tag cleanup conditionally by version, reject duplicate decrements, and make queue overflow,
cancellation, or an incomplete drain fail an exact snapshot closed. The contract still forbids
product mutation and candidate measurement until the recorded review and threshold freeze authorize
D2.

The executable reference model lives in `hydracache-loadgen::notification_observer`. It implements
the version-conditional index, shared per-entry duplicate guard, preallocated bounded queue, dirty
epoch, fail-closed exact snapshot, reconciliation, and shutdown drain without touching the product
cache. Its local allocation probe can be run with:

```text
cargo run -p hydracache-loadgen --release --locked --bin hydracache-notification-observer-prototype -- --output target/performance-evidence/0.73/local/<new-receipt>.json
```

The receipt is diagnostic and non-promotable. It measures the reference publication/drain
mechanism, not Moka automatic eviction or HydraCache product semantics.

The exact-SHA reference run is retained in `notification-observer-prototype-9a2ca114.toml`. Both
the atomic-counter control and the preallocated versioned observer recorded 0 gross allocated bytes
per operation in all three repetitions. Elapsed values are retained for diagnostics but are not a
timing claim: this tiny reference model does not include Moka automatic removal delivery. The result
establishes that the HydraCache-side lifecycle can be allocation-free; the next uncertainty is the
Moka observer seam itself.

The first isolated Moka patch and its exact-SHA result are retained as
`moka-post-removal-observer-0.12.15.patch` and `moka-observer-spike-5d560170.toml`. The patch adds a
lab-only observer mode that keeps Moka's existing removal delivery but does not create the listener
key-lock map. Insert allocation matched listener-off exactly at 400.44 B/op, compared with 755.74
B/op for the listener. This confirms the fill owner. Observer removal still cost 2,700.16 B/op
versus 2,276.23 off because this first patch intentionally retained boxed listener futures and
cancellation delivery. All Explicit, Replaced, Expired, and Size causes were observed. The next
spike removes that second mechanism without altering the production dependency.

Prepare either patch in a fresh ignored directory with
`scripts/prepare-moka-observer-spike.ps1`. The first patch is the default; pass
`-PatchFile docs/testing/performance/0.73/moka-post-removal-observer-direct-0.12.15.patch` for the
direct observer that bypasses both key locks and listener futures. The standalone locked harness in
`tools/moka-observer-spike` keeps this dependency experiment outside the workspace and product
`Cargo.lock`.

The cumulative direct-observer result is retained in `moka-observer-direct-779849b6.toml`. The
observer matched off at 400.69 B/op for insert. For remove it measured 2,275.41 B/op against
2,287.78 off; this small negative delta is treated as no detected allocation penalty, not as an
improvement. The harness also connected real Moka replacement callbacks to the versioned cleanup
model and proved that delayed cleanup for version 41 cannot remove version 42's membership. The lab
implementation is complete. This remains historical lab evidence: it did not authorize itself and
does not become a numerical claim after the later D2 decision.

`notification-observer-d2-review.toml` is the machine-checked review candidate. It proposes a 15%
minimum reduction in the primary fill-allocation metric from the smallest baseline-only observed
listener overhead of 23.892%. Unaffected allocation cells use `max(3%, 16 B/op)` and post-idle/peak
RSS use `max(5%, 1 MiB)` as conservative guards inherited from the reviewed 0.71 practical-effect
contract. Candidate observer measurements are listed separately and are explicitly excluded from
threshold derivation. The project selected the proposal-scoped single-maintainer path. Thresholds
are frozen against the earlier `1e9fd748` commit. The later dependency decision authorizes product
integration but still does not authorize candidate measurement.

`single-maintainer-review-policy.toml` records the exception and its compensating controls. It
requires separate governance, dependency, implementation, and measurement commits; append-only
attempt retention; dedicated-host qualification; explicit self-review wording; panic/reentrancy
falsifiers; and a demonstrated rollback. It does not permit describing the result as independently
reviewed.

`moka-fork-decision-352e53fa.toml` records the resolved dependency choice. The project-owned fork
is pinned to full revision `352e53faa480c9997272b9c70798dd5b5c15d581`, based on upstream Moka
`v0.12.15` at `616473ee923f4cd1429b3d8eb3be7df3eb9906b1`. The receipt binds the source tree,
stable patch id, checked-in prototype-patch digest, fork `Cargo.lock`, CycloneDX 1.5 SBOM, license,
MSRV, maintenance cadence, advisory policy, and one-commit crates.io rollback. Full Moka tests,
Clippy, package verification, all 80 valid feature combinations, `cargo deny`, the isolated
HydraCache harness, panic containment, and Windows/Linux x86_64/Linux aarch64 checks passed. D2 now
allows the separately committed production integration; measurements remain closed until that
integration passes product-level correctness and local admission.

`moka-post-removal-observer-upstream-draft.md` turns the dependency choice into a concrete API
proposal without posting anything externally. The dependency receipt explicitly records
`not-submitted`/`not-requested`, so neither the fork nor D2 can be mistaken for upstream acceptance.
The hardened fork contains panic containment and an explicit nonblocking/non-reentrant callback
contract; HydraCache still has to prove its own product-level reentrancy and shutdown behavior.

`notification-observer-product-73fc38a1.toml` records that product admission. Implementation commit
`73fc38a131d26e78b246fe93d5edd71d33796bbf` pins the fork, replaces the async eviction listener with
a synchronous post-removal observer only when memory instrumentation is enabled, versions entries
and tag memberships, defers conditional cleanup through a bounded 4,096-ticket queue, and makes
dirty or pending epochs fail exact snapshots closed. The existing 72-byte `CacheEntry` estimate is
preserved by storing tags as a boxed slice. Full tests, focused memory/concurrency/model tests,
normal and instrumentation-lab Clippy, feature-leak checking, `cargo deny`, documentation contracts,
and the load-generator observer model passed. A detached worktree at parent commit `03f89354` also
passed the pre-observer check and memory-footprint suite, demonstrating the crates.io-Moka rollback.

The receipt also discloses a preliminary D2 file-ledger omission: several support files required by
the already-authorized surfaces were not listed before implementation. The exact changed-file set
and scope adjudication were recorded before any candidate measurement; thresholds and public API
were unchanged. Product correctness admission now allows append-only local candidate screening as
non-promotable rejection evidence. `candidate_measurements_allowed` remains false for dedicated or
promotable evidence until the host profile is admitted and the full D3 campaign runs.

`local-observer-product-screening-06a8bd95.toml` retains the resulting local comparison. We rebuilt
the pre-observer control at exact commit `03f89354` and the admitted candidate at `06a8bd95`, then
ran five counterbalanced off/production pairs for each source on the same privacy-safe host
fingerprint with seed 7302. Candidate production fill allocation fell from 3,040.38 to 2,479.31
B/op (`-18.45%`), clearing the frozen 15% local rejection minimum, while the within-candidate fill
overhead was approximately zero. Expire/delete fell 26.23%, refill 24.95%, post-idle RSS 23.89%,
and peak RSS 19.50%; steady-read allocation increased by only 1.25 B/op (0.40%). The summary binds
both clean source contexts, binaries, raw `screening.json` digests, and the unchanged thresholds.
It is still `promotable: false`: the campaigns were sequential on a local Windows workstation and
do not replace admitted-host calibration, confidence intervals, CPU/p99 gates, or D3 qualification.

`statistics.toml` freezes the inherited paired estimator, confidence, Holm correction, failure
retention, five-pair minimum, throughput/CPU/p99 guards, and the single-maintainer-reviewed
allocation/RSS proposal before product mutation. `host-profile.toml` is a requirement template,
not an admitted machine: it forbids candidate measurement until a new dedicated-host fingerprint,
serialized lease, pre/post calibration, dependency decision, and production-instrumentation gates
pass.

Validate a generated receipt:

```text
cargo xtask performance-contract-check --release 0.73 --receipt target/performance-evidence/0.73/local/<attempt>/receipt.json
```

This command validates receipts produced by `performance-local-receipt`. The per-attempt
`receipt.json` written by `performance-overhead-screen` is a loadgen memory-efficiency receipt with
a different schema; the screen command validates that receipt internally and binds its digest in
`screening.json`. Do not pass the loadgen receipt to `performance-contract-check --receipt`.

Generate that receipt with `cargo xtask performance-local-receipt`. The command takes the context,
prebuilt binary, frozen scenario, raw-series and outcome JSON paths plus the pair metadata shown by
`cargo xtask --help`; it hashes all file inputs and refuses schema or accounting violations before
writing the receipt. This keeps manual hashes and copied host/source identities out of the workflow.
`local-screening-outcomes.example.json` provides the required accounting shape for runner adapters;
real attempts replace its counts and retain every non-success outcome.

Every receipt binds the exact source, prebuilt binary, scenario, host fingerprint, raw series,
instrumentation mode, pair order, and complete operation outcomes. Failed and invalidated attempts
remain append-only. Local receipts always carry `promotable: false` and
`numerical_claim_eligible: false`; `--require-ship` therefore fails until later W0/W10 contracts
add dedicated-host and integrated-candidate evidence.

The first account-backed step is intentionally cheaper than D3. The manual
`Performance Host Admission 0.73` workflow targets the repository's Linux x86_64 self-hosted
runner through the `hydracache-release` label and the protected `performance-reference-073`
environment. It captures fresh pre/post host fingerprints and five-sample calibrations around an
exact-source release load-generator build, holds one serialized lease through workflow concurrency,
and uploads an immutable admission packet. The packet explicitly keeps
`candidate_measurement_authorized: false`: it must be reviewed and bound into the checked-in host
contract before the five-pair observer qualification is dispatched. The old 0.71 admission is not
reused even when both workflows happen to land on the same physical machine.

Run `36060837195` completed that admission at source `3ba09fcc`. The pre/post fingerprint was
`sha256:702282...465d`; calibration spread was 3.05% before and 1.49% after the release build,
inside the frozen 5% ceiling. `host-admission-3ba09fcc.toml` binds the raw packet and retains the
two earlier failed run IDs: both exposed an unsupported `pidstat --version` probe and neither
started candidate measurement. The host is now eligible, but candidate measurement remains closed
until baseline-only pilots freeze scenario windows and stable offered rates.

`baseline-pilot-contract.toml` preregisters the first such pilot. It uses the standalone
`tools/performance-observer-073` harness to run the same 40% get / 25% tagged-put / 15%
remove-refill / 10% tag-invalidate-refill / 10% TTL-put workload in off and production modes.
Four offered rates receive three counterbalanced, independently started five-second windows each.
The pilot can select an I73 knee and the 25%/60%/85% D3 rates, but it carries no candidate role,
cannot change thresholds, and remains non-promotable.

Run `36064788229` executed that contract at exact source `4ba93a1a`. All 24 attempts completed,
both instrumentation modes sustained at least 99.96% of every offered rate, p99 remained below two
milliseconds, and the host passed its pre/post calibration boundary. Nevertheless, none of the four
rates was eligible: production exceeded the frozen 3% CPU-per-operation ceiling by 4.14%, 6.46%,
10.01%, and 5.27%. Production was slower in CPU time in all 12 paired observations, independent of
which mode ran first, while its allocation cost was consistently about 20 bytes per operation.
`baseline-pilot-insufficient-4ba93a1a.toml` binds the retained artifact and records the result as
negative baseline evidence. I73 remains unfrozen, candidate measurement remains closed, and the
ceiling is unchanged. The next step is a short four-mode attribution run, not an unrecorded retry.

`cpu-attribution-contract.toml` freezes that diagnostic before execution. At 10,000 operations per
second it runs five independent, Williams-counterbalanced blocks across `off`, `counters-only`,
`observer-noop`, and `production`. The adjacent deltas distinguish counter work, backend observer
registration/delivery, and the real HydraCache callback. The two ablations are deliberately
incorrect configurations, so the workflow has no numerical acceptance threshold and cannot freeze
I73; it succeeds only when all attempts and the admitted-host calibration envelope are complete.

Run `36066691730` completed all 20 attempts at exact source `ed339846`; pre/post calibration
spread was 4.27% and 2.09%, inside the unchanged 5% bound. Counter-only CPU was 0.20% below off,
which is classified as no detected counter cost. Empty observer delivery added 1.42%, and the full
callback/cleanup path added a further 1.99%. The complete production path was 3.24% above off and
allocated 20.23 additional bytes per operation. `cpu-attribution-ed339846.toml` binds the raw
packet. The result authorizes only a local optimization of the empty-drain fast path; it does not
accept production overhead or reopen candidate measurement.

Commit `affe4390` implements the bounded local follow-up: `RemovalObserver::drain` now compares its
accepted and acknowledged sequences before taking the async receiver mutex. A test-only acquisition
counter proves that empty drains skip the lock, pending work still forces one acquisition, and the
next empty drain skips it again. The existing duplicate, saturation, pending-barrier, versioned-tag,
memory-footprint, and exact-reconciliation suites remain green. The checked-in
`removal-drain-fast-path-affe4390.toml` receipt authorizes only a repeat of the unchanged
baseline-only contract; it does not authorize candidate data or change the 3% CPU ceiling.

Run `36067885432` repeated that unchanged contract. The fast path moved the 10,000 and 20,000
rates inside the CPU ceiling at 2.80% and 2.08%, while 2,500 and 5,000 both remained at 3.52%.
With two stable rates instead of the required three, the workflow correctly retained another
`insufficient-baseline` packet and left I73 unfrozen. The result is progress, not acceptance.

`removal-queue-contract.toml` freezes the next local step. It replaces the bounded Tokio mpsc and
its async receiver mutex with the already-locked `crossbeam-queue 0.3.12` `ArrayQueue`, preserving
the 4,096-ticket bound, nonblocking callback, duplicate/version rules, dirty-on-overflow behavior,
and accepted/acknowledged barrier. No host repeat is allowed until all local correctness,
reconciliation, clippy, standalone-harness, and supply-chain gates pass.

Commit `daffd71b` implements that contract. Publication now uses a preallocated bounded
`ArrayQueue`; draining uses a single atomic consumer claim whose RAII guard releases the claim even
when the future is dropped. A busy consumer never acknowledges another consumer's work, so the
accepted/acknowledged exactness check continues to fail closed. The queue capacity, overflow and
slot-collision dirty behavior, version-conditional tag cleanup, and reconciliation path are
unchanged. Focused observer, memory-footprint, accounting, formatting, Clippy, standalone-harness,
and supply-chain checks passed.

`removal-queue-product-daffd71b.toml` records the implementation and one scope variance: the
standalone harness lockfile also had to record the exact dependency because it resolves the
workspace crate under `--locked`. This is local correctness admission, not performance evidence.
It authorizes only another run of the unchanged baseline-only contract; the 3% CPU ceiling and the
minimum of three stable offered rates remain frozen.

Run `36069607878` executed that repeat at exact source `90a40510`. All 24 attempts completed and
pre/post calibration spreads were only 0.23% and 0.15%. The 10,000 and 20,000 rates passed at 2.48%
and 0.99% CPU overhead, but 5,000 narrowly missed at 3.23% and 2,500 measured 7.04%. The result
therefore retains exactly two stable rates and remains insufficient. A duplicate manual dispatch,
run `36069619626`, was cancelled before its job started; it is not a hidden measurement retry.

`baseline-pilot-insufficient-90a40510.toml` binds the full packet. The stronger high-rate result is
consistent with less drain synchronization work, but it does not prove that the queue caused the
change or justify dropping low rates. `removal-sequence-contract.toml` preregisters the next bounded
local step: replace two checked atomic CAS loops with single `fetch_add` operations while preserving
dirty-on-overflow and every exactness rule. No further host run is allowed until those local gates
pass.

Commit `549fbaeb` implements that micro-optimization. The returned pre-increment value detects
`u64::MAX`; wrap immediately marks the epoch dirty, and `ensure_clean` checks dirty before comparing
the now-wrapped sequences. A new test forces overflow on both accepted and acknowledged counters,
proves that equal wrapped values cannot look exact, and proves that only reconciliation restores a
clean state. All preregistered local gates passed. `removal-sequence-product-549fbaeb.toml` therefore
authorizes one unchanged baseline-only repeat, not a candidate campaign or numerical claim.

Run `36070743841` retained a third insufficient v1 packet. Only 20,000 operations/second passed;
CPU overhead was 6.62%, 10.13%, 5.46%, and 1.63% across the four rates. All attempts completed and
pre/post calibration spreads remained below 1.21%. The immediately preceding run had measured
3.23% at 5,000 and 2.48% at 10,000, so the new packet does not support a causal regression claim for
the two-instruction sequence change. The v1 estimator uses a ratio of independently summarized mode
medians from only three pairs; its observed repeatability is now a falsified assumption.

`baseline-pilot-v2-contract.toml` corrects the method without relaxing it. The workload, offered
rates, seed, outcome rules, 2% goodput ceiling, 3% CPU and p99 ceilings, and three-stable-rate rule
remain unchanged. V2 increases each cell from three five-second pairs to five ten-second pairs and
aggregates within-pair differences with the already-frozen Hodges-Lehmann estimator. Its workflow is
manual-only, so ordinary branch pushes cannot spend the dedicated-host budget. The incorrect-SHA
dispatch `36070717512` was cancelled before environment approval and executed no job.

Commit `4979e2e1` implements v2 without touching product code. The harness passes its profile id to
every independently started process, retains pair/repeat identity, computes each relative or
absolute overhead inside the pair, and then applies the one-sample Walsh-average Hodges-Lehmann
estimator. It hard-rejects reduced repeats, windows, rates, warmup, or a different seed. Synthetic
unit tests cover estimator construction and regression signs; workflow/contract tests require the
protected lane and absence of a push trigger. `baseline-pilot-v2-product-4979e2e1.toml` admits one
first v2 run and no candidate measurement.

Run `36072021641` executed v2 at exact source `72d491ac`. All 40 attempts completed. Paired CPU
overhead was 3.85%, 1.23%, 3.86%, and 0.94%; only 5,000 and 20,000 operations/second passed. The
10,000 cell contained one 13.61% pair, but the Hodges-Lehmann result remained 3.86%, close to three
other positive pairs, so the outlier did not decide the cell alone. P99, goodput, and calibration
passed. `baseline-pilot-v2-insufficient-72d491ac.toml` therefore retains a higher-power negative
result with I73 still unfrozen.

The next local target is now narrower and evidence-backed. Removal accounting updates eight
retained-memory atomics; each currently uses a checked compare-and-swap loop while enclosed by an
active mutation guard. `memory-counter-atomic-contract.toml` permits replacing only those additions
and subtractions with one atomic operation plus wrap detection. Overflow or underflow must fault
before the guard releases quiescence, so an exact snapshot cannot observe a wrapped value as clean.
Active-mutation, version, epoch, public estimates, workload, and thresholds remain untouched.

Commit `a205ce0a` implements that boundary exactly. Each retained counter now uses one `fetch_add`
or `fetch_sub`; the returned old value decides whether the permanent fault bit must be set. Forced
overflow and underflow tests keep the mutation guard alive and prove that `barrier` sees
`CounterFault`, not merely `NotQuiescent`, before guard release. Capture also refuses the wrapped
state, and the fault remains after release. The active-mutation, version, and epoch counters keep
their checked CAS algorithms because their synchronization windows are not interchangeable with
data-counter accounting. `memory-counter-atomic-product-a205ce0a.toml` retains the full local gate
receipt and authorizes one unchanged, manual v2 baseline repeat; it makes no speed claim itself.

The same push exposed a separate cost-control issue: the host-admission workflow still had an
automatic path and queued run `36072022062` behind the shared performance concurrency group. It was
cancelled before environment approval and ran no job. Commit `e0dc4df5` makes host admission, like
pilot v2, manual-only; a subsequent ordinary push created no performance run.
