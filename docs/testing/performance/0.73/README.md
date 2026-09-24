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
measurement therefore remain forbidden until a lab-only feasibility spike, independent review, and
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
product mutation and candidate measurement until independent review and threshold freeze authorize
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
implementation is complete; independent review, baseline-only allocation/RSS limits, and explicit
D2 authorization remain mandatory before changing HydraCache's production dependency or path.

`notification-observer-d2-review.toml` is the machine-checked review candidate. It proposes a 15%
minimum reduction in the primary fill-allocation metric from the smallest baseline-only observed
listener overhead of 23.892%. Unaffected allocation cells use `max(3%, 16 B/op)` and post-idle/peak
RSS use `max(5%, 1 MiB)` as conservative guards inherited from the reviewed 0.71 practical-effect
contract. Candidate observer measurements are listed separately and are explicitly excluded from
threshold derivation. The packet remains `awaiting-independent-review`: it cannot authorize D2,
candidate measurement, a dependency change, or product mutation until a distinct reviewer records
an accepted decision.

`moka-post-removal-observer-upstream-draft.md` turns the dependency choice into a concrete API
proposal without posting anything externally. It also records a gap found during review: the lab
patch invokes the observer directly, so production authorization additionally requires a panic
containment falsifier and an explicit reentrancy policy. The same document defines what a pinned
fork must record if upstream does not accept the API in the release window.

`statistics.toml` freezes the inherited paired estimator, confidence, Holm correction, failure
retention, five-pair minimum, and throughput/CPU/p99 regression guards before candidate data. Its
allocation and RSS limits remain explicit unfrozen blockers. `host-profile.toml` is a requirement
template, not an admitted machine: it forbids candidate measurement until a new dedicated-host
fingerprint, serialized lease, pre/post calibration, and the unresolved instrumentation gates pass.

Validate a generated receipt:

```text
cargo xtask performance-contract-check --release 0.73 --receipt target/performance-evidence/0.73/local/<attempt>/receipt.json
```

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
