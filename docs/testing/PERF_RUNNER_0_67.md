# Release 0.67 reference runner runbook

`reference-v1` ship evidence is intentionally inactive until an authorized bare-metal runner exists. Ordinary pull requests, pushes, schedules, and `run_nightly` dispatches use GitHub-hosted lanes only; they never execute repository code on this runner.

## Host contract

Use one non-oversubscribed x86_64 bare-metal host with:

- Ubuntu 24.04 LTS, cgroup v2, at least 6 physical CPU cores, 16 GiB RAM, and local NVMe storage;
- four isolated measurement CPUs exposed as the exact cpuset `1-4`;
- no cgroup CPU quota (`/sys/fs/cgroup/cpu.max` begins with `max`);
- a fixed CPU governor and turbo policy for every baseline/candidate run;
- no concurrent workloads, automatic package upgrades, or scheduled maintenance during a run.

The committed profile identifies this family as `self-hosted-bare-metal-v1`. It remains `unbootstrapped`; an empty fingerprint allowlist is expected until reviewed reference runs exist.

## GitHub runner registration

1. Create a dedicated runner group restricted to `javaquasar/hydracache` and to the CI workflow.
2. Install the current GitHub Actions runner as a dedicated unprivileged OS user. Prefer an ephemeral runner or re-image the host after each run.
3. Register the exact custom label `hydracache-perf-v1` in addition to `self-hosted`, `linux`, and `x64`.
4. Do not place registration tokens, repository credentials, or cloud keys in this repository or in workflow YAML.
5. Keep the runner offline when no authorized reference run is planned.

Because this is a public repository, never add `pull_request`, `push`, `schedule`, or tag triggers to the self-hosted job. Only a maintainer-triggered `workflow_dispatch` on trusted `main` with `run_reference_performance=true` and `candidate_release=0.67` may select it.

## Pre-run verification

Before bringing the runner online, verify:

```bash
uname -m
lsb_release -ds
cat /sys/fs/cgroup/cpu.max
taskset --cpu-list 1-4 sh -c 'grep Cpus_allowed_list /proc/self/status'
cat /sys/devices/system/cpu/cpu1/cpufreq/scaling_governor
```

The workflow pins every preflight, prebuild, and measurement process with `taskset --cpu-list 1-4`. The independent preflight executes exactly seven calibration probes, retains every sample, and rejects spread above 15% before compilation or measurement starts. Its report is uploaded as `target/test-evidence/0.67/runner-preflight.json`.

## Authorized run and bootstrap

1. Update `main`, ensure the worktree is clean, and keep the host otherwise idle.
2. Bring the runner online and manually dispatch CI on `main` with `run_reference_performance=true`, `candidate_release=0.67`, and `run_nightly=false`.
3. Archive the complete Actions artifact even when a stage fails. Failed W1 canonical validation produces `local.failed.json`, explicitly marked `ship_evidence_eligible=false`.
4. Take the runner offline after the job finishes.

Do not populate the fingerprint allowlist or activate budgets from a single run. Bootstrap still requires at least five eligible, stable, successful `main` runs from the same fingerprint family and independent review of the immutable anchor, rolling window, and budget payload. Until then, GitHub-hosted `ci-shared` results are regression tripwires only and release 0.67 remains no-ship.