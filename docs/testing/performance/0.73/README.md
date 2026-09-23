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

The first three-pair release-build screen is retained in
`local-overhead-screening-307b3500.toml`. It found no elapsed-time regression and no steady-read
allocation delta, but it did find material mutation allocation and RSS deltas. The result is a
negative, non-promotable blocker—not a threshold proposal. The next step is to isolate the
production-only async eviction-listener cost before repeating the screen.

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
