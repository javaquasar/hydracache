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
