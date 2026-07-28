# HydraCache 0.67.1 Dedicated Performance Reference Bootstrap - Codex Execution Plan

> **At a glance**
> - **What:** qualify one protected non-shared bare-metal runner, retain at least five stable
>   `main` reference runs from one attested machine/contract family, independently review the
>   immutable anchor, rolling baseline, and budgets, then bootstrap `reference-v1`.
> - **Why:** `0.67.0` shipped the measurement machinery honestly without numerical claims.
>   [`TD-0013`](../technical-debt/TD-0013-dedicated-performance-runner-and-baseline-bootstrap.md)
>   remains open because the reference workflow has never run on qualifying hardware.
> - **Depends on:** shipped `0.67.0`.
> - **Unblocks:** official, narrowly scoped reference evidence and the planned `0.68.0` migration
>   conformance release.
> - **Status:** planned.
>
> Roadmap: [`INDEX.md`](INDEX.md) - parent plan:
> [`V0_67_PERFORMANCE_CHARACTERIZATION_PLAN.md`](V0_67_PERFORMANCE_CHARACTERIZATION_PLAN.md)
> - runner runbook: [`../testing/PERF_RUNNER_0_67.md`](../testing/PERF_RUNNER_0_67.md)
> - gates: [`../GATES.md`](../GATES.md) - testing: [`../TESTING.md`](../TESTING.md)

Read [`CLAUDE.md`](../../CLAUDE.md), [`docs/RULES.md`](../RULES.md),
[`docs/PERFORMANCE.md`](../PERFORMANCE.md), and [`docs/GATES.md`](../GATES.md) first.
This is an evidence/bootstrap patch, not an optimization or product-surface release.

## Release Boundary

`0.67.1` does not retroactively change what `0.67.0` shipped. The `0.67.0` release remains a
tooling and methodology release without capacity, sizing, Redis-comparison, metrics-agreement, or
numerical baseline claims.

This patch may publish numerical reference evidence only after every work item below is complete.
Until W6 activates the reviewed contracts and W7 passes on the frozen candidate:

- `reference-v1` remains `unbootstrapped`;
- every bootstrap sample is non-ship exploratory evidence;
- no number may be quoted as a capacity floor, sizing recommendation, Redis advantage, portable
  baseline, or general cluster-capacity claim;
- the existing open-loop schedule, SLOs, repeat counts, zero-error rules, and 15% scenario spread
  rule must not be weakened;
- GitHub-hosted `ci-shared` results remain tripwire-only.

The reference surfaces remain exactly those defined by `0.67.0`: embedded cache, in-process client
surface, one selected node-local RESP endpoint, real daemon control-plane, and library/model
primitives. No native daemon client listener, distributed RESP value plane, aggregate cluster
capacity, or live-reshard performance claim is introduced.

## Infrastructure Reality Correction

The initial provider target is Scaleway, but the contract is provider-neutral.

A Virtual Instance with "dedicated vCPU" is still a virtual machine and does not satisfy the
current `self-hosted-bare-metal-v1` contract. It may be used for exploratory qualification of the
automation only, and all resulting reports must remain ineligible.

The reference candidate must be Scaleway Elastic Metal or equivalent true bare metal with:

- Ubuntu 24.04 LTS, x86_64, cgroup v2;
- at least six physical CPU cores, 16 GiB RAM, and local NVMe;
- four distinct physical measurement CPUs available through the exact cpuset `1-4`;
- unlimited cgroup CPU quota;
- a fixed governor and turbo policy;
- no concurrent workload, unattended upgrade, scheduled maintenance, or autoscaling replacement
  during the retained run window.

The provisional cost-focused candidate is `EM-A610R-NVMe`; the preferred fixed eight-core candidate
is `EM-B230E-NVMe`. SKU selection is not evidence: W2 must attest the observed machine and W3 must
qualify its stability before the fingerprint can be reviewed.

## Preflight Findings

The post-`0.67.0` audit found that most execution machinery already exists:

- `.github/workflows/ci.yml` has a manual, serialized job restricted to trusted `main`, with
  `runs-on: [self-hosted, linux, x64, hydracache-perf-v1]`;
- the job pins Rust `1.94.0`, builds Redis `7.2.5`, runs the complete canary/contract/preflight,
  prebuilds exact binaries, executes core/RESP/control-plane evidence, checks budgets, aggregates
  receipts, and uploads artifacts even on failure;
- `perf-runner-preflight` already executes seven calibration probes and rejects preflight spread
  above 15%;
- report methodology also fixes the reference scenario spread at 15%; the profile-level 30%
  ceiling is a separate outer validation bound and must not be confused with permission to weaken
  the scenario rule;
- `reference-v1` has an empty fingerprint allowlist and unbootstrapped anchor/baseline/budgets, as
  required while TD-0013 is open.

The missing pieces are material:

1. The runner currently self-asserts `shared_hardware=false`; it does not mechanically reject a
   hypervisor or prove that cpuset `1-4` maps to four distinct physical cores.
2. The stable fingerprint does not bind a privacy-safe physical-host identity, CPU topology, local
   NVMe identity/class, or OS-image contract. Two nominally identical hosts could collide.
3. The current all-in-one workflow has no explicit qualification/bootstrap-sample mode. A first
   unbootstrapped run should upload diagnostics without pretending to be ship evidence.
4. There is no committed five-run acquisition manifest or deterministic reviewed promotion step
   from bootstrap samples to anchor/baseline/budgets.
5. The five existing `0.67` reference gates are deferred. `0.67.1` must make the corresponding
   bootstrap and final reference receipts release-mandatory without rewriting historical
   `0.67.0` evidence.

## Work Items

### W0. Register governance and freeze the 0.67.1 claim boundary

Files:

- `docs/plans/releases.toml`
- `docs/plans/INDEX.md`
- `docs/testing/release-evidence/0.67.1.toml`
- `docs/testing/canary-registry-0.67.1.json`
- `docs/testing/gated-test-registry.toml`
- `crates/xtask` governance tests

Requirements:

- register exact `work_items = ["W0", "W1", "W2", "W3", "W4", "W5", "W6", "W7"]`;
- add release-scoped canary/evidence ownership before implementation work;
- make missing qualification, sample-set, review, activation, or frozen-candidate receipts red;
- preserve the historical `0.67.0` manifest and deferred receipts unchanged;
- make governance land before runner activation or baseline data.

Definition of Done:

```powershell
cargo test -p xtask --test release_governance --locked
cargo run -p xtask --locked -- release-governance-check --release 0.67.1
```

### W1. Provision and secure the dedicated runner

Files:

- `docs/testing/PERF_RUNNER_0_67_1.md`
- `scripts/perf/` host-audit and runner-lifecycle helpers
- optional provider notes that contain no credentials, account ids, tokens, or instance ids

Requirements:

- document hourly create/install/qualify/run/offline/delete lifecycle;
- use a dedicated unprivileged GitHub Actions user and repository-restricted runner group;
- register the exact `hydracache-perf-v1` label;
- keep the runner offline except for authorized `workflow_dispatch` runs on trusted `main`;
- require outbound-only GitHub/tool download access; restrict administrative SSH by source and key;
- disable unattended upgrades, timers, and nonessential services during measurements;
- record package/tool versions and clean-worktree state;
- provide a teardown checklist that archives artifacts before deleting the host.

No cloud credential, GitHub registration token, SSH private key, or crates.io secret may enter the
repository, workflow YAML, artifact, or runner work directory.

Definition of Done:

```bash
scripts/perf/audit-reference-host.sh --mode provisioned
scripts/perf/verify-runner-service.sh --expected-label hydracache-perf-v1
```

### W2. Harden machine attestation and fingerprint v2

Files:

- `crates/xtask/src/perf.rs`
- `crates/hydracache-loadgen/src/profile.rs`
- performance report/baseline schemas and contract tests
- `docs/testing/perf-profiles/reference-v1.toml`

Requirements:

- fail closed when virtualization is detected (`systemd-detect-virt` or an equivalent independent
  kernel/DMI proof);
- prove at least six physical cores and that logical CPUs `1-4` have four distinct
  package/core-id pairs;
- attest cgroup v2, unlimited quota, CPU affinity, governor, turbo policy, RAM, kernel, OS image,
  and local NVMe;
- add a privacy-safe stable-host digest derived from the physical host identity without emitting
  the raw DMI UUID, disk serial, cloud instance id, or account metadata;
- bind the fingerprint to topology, host digest, storage class, runner contract version, and
  prebuild/toolchain contract;
- reject VM/dedicated-vCPU exploratory hosts even if they carry the custom GitHub label;
- add negative canaries for hypervisor acceptance, SMT-sibling cpuset acceptance, non-NVMe
  acceptance, and host-identity omission.

Changing the fingerprint schema invalidates earlier exploratory fingerprints by design.

Definition of Done:

```powershell
cargo test -p xtask preflight_tests --locked
cargo test -p xtask --test perf_budget_067 --locked
cargo test -p hydracache-loadgen --test performance_contract_067 --locked
```

### W3. Add a qualification mode that cannot become release evidence

Files:

- `.github/workflows/ci.yml`
- `crates/xtask/src/perf.rs`
- release registry/evidence tests

Requirements:

- add an explicit trusted-manual `qualify` mode;
- run host attestation, seven-probe preflight, exact prebuild, and bounded smoke/reference
  diagnostics;
- stop before anchor/budget activation and mark every report
  `ship_evidence_eligible=false`;
- upload complete diagnostics on success or failure;
- refuse qualification from PR, push, schedule, tag, fork, dirty source, or a non-`main` ref;
- keep the existing serialized concurrency group and six-hour hard timeout;
- prove via canary that renaming a VM runner with `hydracache-perf-v1` cannot pass.

Qualification proves that a host is worth collecting on. It does not approve its fingerprint and
does not count as one of the five retained baseline runs.

### W4. Collect and retain five bootstrap-eligible main runs

Files:

- `docs/testing/perf-bootstrap/0.67.1/sample-set.toml`
- artifact ingestion/validation in `crates/xtask`
- GitHub workflow artifact metadata

Requirements:

- collect at least five successful runs while keeping the same physical host, fingerprint v2,
  kernel, governor/turbo policy, cpuset, toolchain, prebuild digest, scenario digest, and SLO
  contract;
- run only clean, pre-activation `main` commits; no candidate may baseline itself;
- distinguish `bootstrap_eligible=true` from `ship_evidence_eligible=false`;
- retain run ids, commit SHAs, artifact digests, timestamps, observed noise, and exact runner
  fingerprint;
- reject failed, unstable, quarantined, stale, mixed-fingerprint, mixed-contract, manually edited,
  or missing-artifact samples;
- require every scenario's original repeats, zero-error rule, SLO, and 15% spread bound.

The sample-set manifest contains digests and provenance, not copied headline numbers.

### W5. Build and independently review anchor, rolling baseline, and budgets

Files:

- `docs/testing/perf-anchors/0.67.1/`
- `docs/testing/perf-baselines/0.67.1/reference-v1.toml`
- `docs/testing/perf-budgets/0.67.1/reference-v1.toml`
- deterministic bootstrap/review commands in `crates/xtask`

Requirements:

- derive candidate anchor/baseline/budgets deterministically from the committed five-run sample
  manifest;
- never select the fastest run or silently discard an inconvenient eligible run;
- emit a machine-readable review payload binding input/output digests and reviewer decision;
- require a reviewer distinct from the artifact-producing automation identity;
- reject absolute/capacity claims that lack report, scenario, fingerprint, profile, commit, method,
  and claim scope;
- add a no-silent-rebaseline canary: changing any numerical payload without changing reviewed
  provenance must fail.

The review may reject the host or request more runs. Rejection keeps TD-0013 open and is a valid
outcome; it must not be converted into wider tolerances.

### W6. Activate reference-v1 and resolve TD-0013 only from reviewed evidence

Files:

- `docs/testing/perf-profiles/reference-v1.toml`
- reviewed anchor/baseline/budget payloads
- `docs/{PERFORMANCE,TESTING,GATES,POSITIONING}.md`
- `docs/technical-debt/README.md`
- `docs/technical-debt/TD-0013-dedicated-performance-runner-and-baseline-bootstrap.md`
- `docs/releases/0.67.1.md`

Requirements:

- add exactly the reviewed fingerprint v2 to the allowlist;
- change bootstrap state only in the same commit as the reviewed anchor/baseline/budgets;
- move TD-0013 to resolved only when W1-W5 receipts are present and valid;
- keep `0.67.0` release notes historically unchanged;
- scope any `0.67.1` numerical statement to this runner/scenario/method; no portability claim;
- document how hardware, kernel, topology, governor, turbo, storage, or contract drift forces
  requalification rather than automatic baseline migration.

### W7. Run the full frozen-candidate reference pipeline and ship evidence

Files:

- `.github/workflows/ci.yml`
- `docs/testing/release-evidence/0.67.1.toml`
- final release receipts and artifacts

Required final stages:

1. runner attestation and seven-probe preflight;
2. exact release prebuild and digest binding;
3. core reference evidence;
4. RESP plus pinned same-box Redis evidence;
5. real 3/5/7-daemon control-plane evidence;
6. budget and rolling-baseline verdict;
7. canary sweep, exact receipts, artifact-integrity validation, and release aggregation.

The final run must execute from one clean frozen `0.67.1` candidate commit. It may consume the
reviewed pre-candidate baseline, but it must not be a member of that baseline.

Definition of Done:

```powershell
cargo run -p xtask --locked -- canary-sweep --release 0.67.1 --tier all
cargo run -p xtask --locked -- release-evidence --release 0.67.1 `
  --receipts-dir target/release-evidence/receipts --require-ship
```

## Execution Order

1. Land W0 governance.
2. Provision one hourly bare-metal host and complete W1.
3. Land W2 attestation hardening before trusting any machine output.
4. Run W3 qualification. Replace or reconfigure the host if it fails.
5. Collect W4 runs without changing the host or execution contract.
6. Generate and independently review W5 payloads.
7. Land W6 activation as a dedicated reviewable commit.
8. Run W7 on the frozen candidate.
9. Tag and publish `0.67.1` only after ordinary CI and the full reference pipeline are green.
10. Archive evidence, take the runner offline, and delete hourly infrastructure when retention is
    no longer needed.

## Cost and Failure Policy

- Use hourly billing during qualification/bootstrap; do not buy a savings plan before the host
  passes W3.
- Keep the same physical server for W4. Recreating an equivalent SKU does not preserve identity.
- A failed or noisy run is evidence about the host and stays archived; it is never edited away.
- If fewer than five eligible runs remain, collect more runs rather than relaxing eligibility.
- If provider maintenance, CPU topology, kernel, or storage changes, begin a new fingerprint family.
- Infrastructure cost is bounded by an explicit operator-set lifetime; workflow timeout remains
  six hours per run.

## Release Gates

Fast/structural gates:

```powershell
cargo fmt --all -- --check
cargo test -p xtask --test release_governance --locked
cargo test -p xtask preflight_tests --locked
cargo test -p xtask --test perf_budget_067 --locked
cargo test -p hydracache-loadgen --test performance_contract_067 --locked
cargo run -p xtask --locked -- doc-check
```

Dedicated gates:

```text
qualification -> five bootstrap runs -> reviewed activation -> frozen-candidate full reference run
```

Every dedicated stage is manual, serialized, trusted-`main` only, artifact-bound, and fail-closed.

## Final Release Decision

Ship `0.67.1` only when:

- W0-W7 are complete with exact receipts;
- one true bare-metal fingerprint v2 is approved;
- at least five stable pre-candidate `main` runs from that fingerprint are retained;
- anchor, rolling baseline, and budgets are independently reviewed;
- `reference-v1` is bootstrapped without candidate self-baselining;
- the complete frozen-candidate reference pipeline is green;
- TD-0013 is resolved and documentation states the narrow claim scope;
- no SLO, repeat, zero-error, spread, fingerprint, or evidence-integrity check was weakened.

If the machine is unavailable or fails qualification, keep `0.67.1` planned and TD-0013 open.
Infrastructure availability is not permission to manufacture a baseline.
