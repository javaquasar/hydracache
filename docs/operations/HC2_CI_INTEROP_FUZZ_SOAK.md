# HC/2 CI, Interoperability, Fuzz, and Soak Runbook

Status: H22 implemented for the non-production HC/2 client-plane foundation.
This gate protects correctness and compatibility. It does **not** establish an
absolute latency, throughput, capacity, availability, or production-readiness
claim. H01 still owns production listener integration.

## Gate contract

The machine-readable source of truth is
`docs/testing/hc2-ci/h22-gates.json`. The workflow is
`.github/workflows/hc2-client-plane.yml`, and `cargo xtask
client-plane-ci-check` fails closed when their reviewed lane names, timeouts,
toolchains, action commits, or image digests diverge.

| Lane              | Trigger                                        | Runner                                   |  Limit | Purpose                                                                                                                                     |
| ----------------- | ---------------------------------------------- | ---------------------------------------- | -----: | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `linux-required`  | pull request, main push, tag, schedule, manual | GitHub `ubuntu-24.04`                    | 30 min | format, schema/workflow contract, clean generation, Rust/Java/Python lifecycle and golden evidence, clippy                                  |
| `docker-interop`  | pull request, main push, tag, schedule, manual | GitHub `ubuntu-24.04` + pinned container | 45 min | Rust tests plus real Java fixture/installed SDK consumer, offline Python, and retained fault replay in one reproducible process environment |
| `fuzz`            | tag, schedule, opted-in manual                 | GitHub `ubuntu-24.04`                    | 20 min | deterministic-seed, time-boxed mutation of generated HC/2 envelopes, transport codecs, and fault receipts                                   |
| `fixed-host-soak` | tag, enabled schedule, opted-in manual         | labelled self-hosted Linux host          | 90 min | twelve bounded lifecycle iterations with retained host/toolchain metadata                                                                   |

The exact required check names for branch protection are:

- `HC/2 Linux Required`;
- `HC/2 Docker Interop`.

GitHub branch protection is repository administration, not a property of the
workflow file. A maintainer must add these names after their first successful
run. Tags and an opted-in release-admission dispatch additionally require all
four same-commit receipts.

## Local reproduction

Run the required host-native proof:

```bash
cargo fmt --all -- --check
cargo run -p xtask --locked -- client-plane-ci-check
cargo run -p xtask --locked -- client-plane-spike-check
cargo clippy -p hydracache-client-plane-spike -p hydracache-client-hc2 -p xtask --all-targets --locked -- -D warnings
```

Run the digest-pinned Docker process-interoperability proof:

```bash
docker build --pull --file scripts/hc2/Dockerfile.interop --tag hydracache-hc2-interop:h22-local .
docker run --rm \
  --volume "$PWD:/workspace" \
  --workdir /workspace \
  --env CARGO_TARGET_DIR=/workspace/target/hc2-docker \
  --env HC2_SHARED_TARGET_DIR=target/hc2-docker \
  hydracache-hc2-interop:h22-local \
  run -p xtask --locked -- client-plane-docker-interop-check
```

`HC2_SHARED_TARGET_DIR` deliberately lets the xtask launcher, Rust spike, and
Rust peer used by the Java SDK share one build graph. This changes no assertion;
it avoids rebuilding the same candidate in independent target directories. The broader
compatibility and native Rust package gates remain in `linux-required`.

Run the bounded fuzz target when the pinned nightly and `cargo-fuzz 0.13.2` are
installed:

```bash
cd fuzz
cargo +nightly-2026-08-01 fuzz run fuzz_hc2_client_plane -- \
  -seed=220068 -max_total_time=180
```

## Receipts and release admission

Every successful lane writes one
`hydracache.hc2.ci-receipt.v1` JSON document. It contains a full candidate SHA,
lane/outcome, run identity, bounded runner metadata, evidence profile, and the
lane-specific image digest, seed, or iteration count. GitHub retains receipts
and diagnostic logs for 30 days.

Admission accepts exactly one passing receipt for each of the four lane IDs.
All four must name the same full commit and, when supplied, the requested
candidate SHA:

```bash
cargo run -p xtask --locked -- client-plane-ci-admission \
  --receipts target/hc2-ci-admission \
  --commit "$GITHUB_SHA"
```

Missing, duplicate, red, malformed, mixed-SHA, wrong-schema, wrong-image,
unbounded, or lane-inconsistent receipts are rejected. The integration tests
intentionally construct missing-lane, red-lane, mixed-SHA, and substituted-
image canaries and require admission to fail. A skipped optional lane is
therefore never silently converted into a release pass.

## Fixed-host soak tier

Register a clean Linux x64 runner with all labels
`self-hosted`, `linux`, `x64`, and `hydracache-hc2-soak-v1`. Install the pinned
Rust toolchain and ordinary native build prerequisites. Do not place unrelated
work on the host during a retained soak. Set repository variable
`HC2_FIXED_HOST_SOAK_ENABLED=true` only while the runner is intentionally
available for scheduled work; a tag or explicit manual selection requests it
regardless of that schedule variable.

The lane records `uname`, `lscpu`, exact Git SHA, verbose `rustc` identity,
test output, iteration count, runner identity, and the receipt. It is a
fixed-host correctness/stability trend, not the `0.67.1` reference performance
profile. It neither relaxes nor substitutes the reference host's affinity,
IRQ, calibration, spread, SLO, zero-error, or fail-closed rules.

## Pin and update policy

GitHub actions are pinned to full commits. Base images are pinned to immutable
SHA-256 manifest digests; language versions and the fuzz seed are explicit.
To update one pin:

1. resolve and review the upstream version and immutable digest/commit;
2. update the workflow or Dockerfile and `h22-gates.json` together;
3. run `client-plane-ci-check` and the affected local lane;
4. retain the pull-request receipts before changing branch protection or
   making any HC/2 release claim.

Never replace a digest with `latest`, retry a product/test failure, edit a
receipt, or treat GitHub-hosted timing as capacity evidence. Infrastructure
retries must be explicit, bounded, and distinguishable from product results.

## Failure triage

| Failure                         | Meaning                                                                                     | Action                                                                                    |
| ------------------------------- | ------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| `client-plane-ci-check`         | checked-in contract and executable workflow drifted                                         | review the exact pin/lane change; update both files or revert                             |
| Linux generation/golden failure | language/schema output differs or lifecycle contract regressed                              | reproduce host-native; never refresh generated output without reviewing the schema change |
| Docker-only failure             | hermetic dependency, Java/Python packaging, filesystem, or inter-process assumption differs | reproduce with the exact image digest and retain the Docker log                           |
| fuzz crash/timeout artifact     | decoder, envelope, codec, or replay input found a failure                                   | retain the seed/artifact, add a deterministic corpus regression, then fix                 |
| soak iteration failure          | lifecycle/resource behavior is not stable on the fixed host                                 | retain host metadata and first failing iteration; do not average or rerun it away         |
| release admission failure       | evidence set is incomplete, red, duplicated, malformed, or cross-candidate                  | obtain new same-SHA evidence; never copy or relabel an older receipt                      |
