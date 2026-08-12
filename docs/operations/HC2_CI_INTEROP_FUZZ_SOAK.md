# HC/2 CI, Interoperability, Fuzz, and Soak Runbook

Status: H22 implemented for the HC/2 foundation and completed H01 production listener.
This gate protects correctness and compatibility. It does **not** establish an
absolute latency, throughput, capacity, availability, or production-readiness
claim. H03 owns adapter acceptance; H01/H21 now own and test the internal
production metrics mount.

## Gate contract

The machine-readable source of truth is
`docs/testing/hc2-ci/h22-gates.json`. The workflow is
`.github/workflows/hc2-client-plane.yml`, and `cargo xtask
client-plane-ci-check` fails closed when their reviewed lane names, timeouts,
toolchains, action commits, or image digests diverge.

| Lane              | Trigger                                        | Runner                                   |  Limit | Purpose                                                                                                                                     |
| ----------------- | ---------------------------------------------- | ---------------------------------------- | -----: | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `linux-required`  | pull request, main push, tag, schedule, manual | GitHub `ubuntu-24.04`                    | 30 min | format, schema/workflow contract, clean generation, Rust/Java/Python lifecycle and golden evidence, clippy                                  |
| `docker-interop`  | pull request, main push, tag, schedule, manual | GitHub `ubuntu-24.04` + pinned container | 45 min | production daemon mTLS/coexistence/startup/drain tests plus Rust spike tests, real Java fixture/installed SDK consumer, offline Python, and retained fault replay in one reproducible process environment |
| `fuzz`            | tag, schedule, opted-in manual                 | GitHub `ubuntu-24.04`                    | 20 min | deterministic-seed, time-boxed mutation of generated HC/2 envelopes, transport codecs, and fault receipts                                   |
| `fixed-host-soak` | enabled schedule or client-promotion manual    | labelled self-hosted Linux host          | 90 min | twelve bounded lifecycle iterations with retained host/toolchain metadata                                                                   |

The exact required check names for branch protection are:

- `HC/2 Linux Required`;
- `HC/2 Docker Interop`.

GitHub branch protection is repository administration, not a property of the
workflow file. Both names were installed on `main` after their first successful
run, with strict head-branch freshness, GitHub Actions app binding, and
administrator enforcement. Force pushes and branch deletion remain disabled.
Tags and an opted-in hosted-admission dispatch require three same-commit
receipts: Linux, Docker, and fuzz. A separately opted-in Java/Python promotion
dispatch requires those three plus the fixed-host receipt.

## Hosted activation evidence

The first exact-candidate hosted activation completed on 2026-08-11 for commit
`93f0de21568d60b77aa134fb44afcf8d6ceb586f` in manually dispatched workflow
run [`31488636299`](https://github.com/javaquasar/hydracache/actions/runs/31488636299).
It produced these immutable-at-upload artifacts:

| Lane | Job | Artifact | GitHub artifact digest |
| ---- | --- | -------- | ---------------------- |
| `linux-required` | [`93769701789`](https://github.com/javaquasar/hydracache/actions/runs/31488636299/job/93769701789) | `9100457998` (`hc2-linux-93f0de21568d60b77aa134fb44afcf8d6ceb586f-31488636299-1`) | `sha256:49804be251de21faf994d4f4a538b5ac7c0f0a07dc9809ed6dcba76425ff0b2e` |
| `docker-interop` | [`93769701852`](https://github.com/javaquasar/hydracache/actions/runs/31488636299/job/93769701852) | `9100297478` (`hc2-docker-93f0de21568d60b77aa134fb44afcf8d6ceb586f-31488636299-1`) | `sha256:84ceb68d32238a21207ea828570cd2a46e32bf9b9dafb601d8f0c5034a5faa94` |
| `fuzz` | [`93769701773`](https://github.com/javaquasar/hydracache/actions/runs/31488636299/job/93769701773) | `9100362252` (`hc2-fuzz-93f0de21568d60b77aa134fb44afcf8d6ceb586f-31488636299-1`) | `sha256:be8612b8bfa13c2b8b505a5d16818261e4e2659e2f020d69d6e3e09d5d2d9add` |

The same commit also passed the pull-request-required Linux and Docker jobs in
run [`31488620419`](https://github.com/javaquasar/hydracache/actions/runs/31488620419).
The manually requested fixed-host job was deliberately skipped because no
labelled rented runner was online, so the four-lane client-promotion admission
was also skipped. This is truthful hosted activation evidence, not an admission
for the current 0.68 candidate, a Java/Python distribution promotion, or a
capacity claim. H22 remains open until a same-commit labelled fixed-host soak
receipt exists.

## Local reproduction

Run the required host-native proof:

```bash
cargo fmt --all -- --check
cargo run -p xtask --locked -- client-plane-ci-check
cargo run -p xtask --locked -- client-plane-spike-check
cargo clippy -p hydracache-client-plane-spike -p hydracache-client-hc2 -p hydracache-server -p xtask --all-targets --locked -- -D warnings
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
cargo +nightly-2026-08-01 fuzz build fuzz_hc2_client_plane
timeout --signal=INT --kill-after=10s 180s \
  cargo +nightly-2026-08-01 fuzz run fuzz_hc2_client_plane -- \
    -seed=220068 -timeout=5 -error_exitcode=77 -timeout_exitcode=70
```

The workflow records the fuzz process status through Bash `PIPESTATUS` rather
than the status of `tee`. Exit `124` is the reviewed successful end of the
outer GNU `timeout` timebox; zero is also accepted when libFuzzer terminates
cleanly first. A sanitizer/crash error (`77`), per-input timeout (`70`), any
other early failure, or any file under the failure-artifact directory fails
the lane and prevents a passing receipt. This avoids depending on
toolchain-specific `-max_total_time` exit-code behavior without accepting a
product failure as a scheduled stop.

## Receipts and release admission

Every successful lane writes one
`hydracache.hc2.ci-receipt.v1` JSON document. It contains a full candidate SHA,
lane/outcome, run identity, bounded runner metadata, evidence profile, and the
lane-specific image digest, seed, or iteration count. GitHub retains receipts
and diagnostic logs for 30 days.

The Rust-library admission accepts exactly one passing receipt for each hosted
lane and binds all three to the requested full candidate SHA:

```bash
cargo run -p xtask --locked -- client-plane-ci-admission \
  --receipts target/hc2-ci-admission \
  --scope hosted \
  --commit "$GITHUB_SHA"
```

The Java/Python distribution-promotion admission adds the fixed-host lane:

```bash
cargo run -p xtask --locked -- client-plane-ci-admission \
  --receipts target/hc2-ci-admission \
  --scope full \
  --commit "$GITHUB_SHA"
```

Missing, duplicate, red, malformed, mixed-SHA, wrong-schema, wrong-image,
unbounded, or lane-inconsistent receipts are rejected. The integration tests
intentionally construct missing-lane, red-lane, mixed-SHA, and substituted-
image canaries and require admission to fail. The hosted scope deliberately
does not read a fixed-host receipt; the full scope requires it and never turns a
skip into Java/Python promotion evidence.

## Fixed-host soak tier

Register a clean Linux x64 runner with all labels
`self-hosted`, `linux`, `x64`, and `hydracache-hc2-soak-v1`. Install the pinned
Rust toolchain and ordinary native build prerequisites. Do not place unrelated
work on the host during a retained soak. Set repository variable
`HC2_FIXED_HOST_SOAK_ENABLED=true` only while the runner is intentionally
available for scheduled work. An explicit fixed-host or full client-promotion
dispatch requests it regardless of that schedule variable; a Rust release tag
does not consume an unavailable rented runner.

The reviewed machine contract is exact: Ubuntu 24.04 on x86_64, a non-root
runner service, a safe bounded runner name, the `hc2-fixed-soak-v1` evidence
profile, and the exact `GITHUB_SHA` in a clean tracked checkout. The workflow
runs `scripts/hc2/verify-fixed-host.sh` before the first soak iteration and
fails closed if the operating system, architecture, toolchain commands, runner
environment, exact Rust/Cargo 1.94.0 toolchain, checkout identity, or tracked
worktree differs. The script writes the same metadata file that is uploaded
even when a later soak iteration fails. This preflight verifies the correctness
host profile; it does not turn the host into a `0.67.1` reference-performance
machine.

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
| hosted admission failure        | Linux/Docker/fuzz set is incomplete, red, duplicated, malformed, or cross-candidate         | obtain new same-SHA evidence; never copy or relabel an older receipt                      |
| client-promotion failure        | full set lacks a valid same-SHA fixed-host receipt                                           | rent/register the reviewed host and rerun all four lanes; never waive or emulate the lane |
