# Next rental: reproducible reference-host playbook

Status: prepared from the HydraCache 0.67/0.67.1 qualification, bootstrap,
exploratory comparison, resource telemetry, metric-expansion, memory, and failure
investigations completed through 2026-08-03.

This playbook does not claim that the 0.67 release performance objective was
met. The previous server was deleted before a new exact-main qualification and
five accepted bootstrap samples were completed. It records what was learned and
turns the next rental into a profile-driven, auditable procedure.

Before ordering hardware, require a green receipt from the
[six-scenario local orchestration hardening suite](PERF_RUNNER_0_67_1_LOCAL_ORCHESTRATION_HARDENING.md).
That receipt is a rental-readiness check only and is never qualification or
bootstrap evidence.

Also create a local procurement admission from the reviewed
[`perf-procurement/0.67.1.json`](perf-procurement/0.67.1.json) policy. The
receipt names the rental operator, billing owner, deletion owner, approved
hourly/time ceiling, exact green `main` SHA, CI run, provider, and SKU. It must
contain no provider identifiers, IP addresses, credentials, SSH private
material, or runner tokens:

```bash
python3 scripts/perf/reference-rental-readiness.py \
  --output /var/lib/hydracache-perf/procurement-admission.json \
  --rental-operator '<operator>' \
  --billing-owner '<billing-owner>' \
  --deletion-owner '<deletion-owner>' \
  --provider '<provider>' --sku '<bare-metal-sku>' \
  --hourly-rate-eur '<rate>' --authorized-hours '<hours>' \
  --main-sha '<green-origin-main-sha>' \
  --main-ci-run-url 'https://github.com/javaquasar/hydracache/actions/runs/<id>' \
  --decision-reference '<durable-operator-decision>' \
  --approve
```

Run the command from a clean checkout whose `HEAD` equals the locally fetched
`origin/main`, with authenticated `gh`. The script queries the named Actions
run and requires `completed/success` for that exact SHA. This admission
authorizes spend and ownership only. It is explicitly non-promotable and is not
performance evidence.

The safe in-host portion of the next campaign can now be driven by the
fail-closed controller documented in
[`PERF_RUNNER_0_67_1_CAMPAIGN_AUTOMATION.md`](PERF_RUNNER_0_67_1_CAMPAIGN_AUTOMATION.md).
It automates preparation, post-reboot freeze and IRQ burn-in, unique dispatch
correlation, two full-dress runs, five serialized samples, original artifact
retention, resume, and the final deletion handoff. Provider creation/deletion,
credential entry/revocation, and the explicit reboot remain human boundaries.

The detailed historical evidence remains in the immutable
[`explore-0.67-telemetry-20260803`](https://github.com/javaquasar/hydracache/tree/explore-0.67-telemetry-20260803)
archive tag, under:

- `docs/testing/PERF_RUNNER_0_67_1.md`;
- `docs/testing/perf-scenarios/0.67/exploratory-preparation-and-measurement-report.md`;
- `docs/testing/perf-scenarios/0.67/exploratory-three-target-resource-report.md`;
- `docs/testing/perf-scenarios/0.67/exploratory-memory-allocation-report.md`;
- `docs/testing/perf-scenarios/0.67/exploratory-experiment-backlog.md`;
- `docs/testing/perf-scenarios/0.67/future-cluster-resilience-test-plan.md`;
- the immutable raw/results archives below `results/`.

## Decision: pin Ubuntu 24.04, then freeze the exact machine

The supported reference image for the next rental is **Ubuntu Server 24.04 LTS
x86_64**. That is the right level to pin permanently for this campaign:

- the current audit and provisioning receipt already require `ID=ubuntu` and
  `VERSION_ID=24.04`;
- Docker, systemd, cgroup v2, kernel isolation, rootless Docker, and the runner
  instructions were exercised on this distribution;
- changing the distribution while trying to close 0.67 would introduce a new
  causal variable and make the previous investigations less useful.

The version pin has two layers:

1. The versioned compatibility profile
   `docs/testing/perf-host-profiles/ubuntu-24.04-reference-v1.json` requires
   Ubuntu 24.04, x86_64, bare metal, the `6.8.0-*-generic` kernel family, cgroup
   v2 without a CPU quota, NVMe, minimum RAM/core capacity, and the reviewed CPU
   contract.
2. `freeze` records the **exact** kernel release, command line, installed package
   manifest, systemd state, sysctls, source commit, profile digest, and
   provisioning receipt. Those exact values must not change within a
   qualification/bootstrap sample family.

Do not put an everlasting equality check for one patch kernel such as
`6.8.0-136-generic` into release policy. Security updates and provider images
move. A new kernel patch may be accepted only before a sample family starts,
after it passes the same provisioning/IRQ checks. Once frozen, any kernel,
package, profile, image-digest, source, service, sysctl, CPU-isolation, or runner
contract change invalidates the family and restarts it from qualification.

## New automation and its safety model

The preparation flow is now split into four reviewed components:

| Component | Responsibility |
| --- | --- |
| `ubuntu-24.04-reference-v1.json` | Machine-readable OS, hardware, CPU, service, and freeze contract |
| `ubuntu-24.04-memory-only-v1.json` | Additive non-ship profile for guarded diskless measurement windows; see [`PERF_MEMORY_ONLY_HOST_PROFILE.md`](PERF_MEMORY_ONLY_HOST_PROFILE.md) |
| `reference-host-tuning.sh` | `plan`, allowlisted `apply`, `verify`, `freeze`, and recorded-state `restore` |
| `check-reference-host-freeze.sh` | Pre-dispatch drift check against the frozen exact environment |
| `prepare-reference-host.sh` | Small stage wrapper that keeps reboot, runner registration, and Docker startup explicit |

The service tuner is deliberately conservative:

- it changes only unit names present in the versioned profile;
- it refuses duplicate actions or any overlap with protected units;
- it records every present/absent unit and its pre-apply `LoadState`, active
  state, and unit-file state in `plan.json`;
- `apply` requires root, an offline GitHub runner, and stopped rootless/rootful
  Docker;
- `restore` reconstructs the pre-apply enabled/masked and active/inactive state;
- it never runs a wildcard `systemctl disable` or attempts to guess which
  unknown services are safe to remove.

Protected services include SSH, the available time-synchronization provider,
journald, logind, and udev. Candidate desktop/network services are reported but
not changed automatically. This matters because a generic “disable everything”
command can lock out SSH, break clock-based evidence, hide kernel failures, or
damage provider networking.

### Why the allowlisted services are quieted

| Class | Units | Reason |
| --- | --- | --- |
| Package maintenance | `apt-daily*`, `unattended-upgrades`, `packagekit*` | Avoid package downloads, dpkg locks, CPU, and disk activity during samples |
| Periodic storage work | `fstrim`, `e2scrub_all`, `fwupd-refresh`, `man-db` timers | Avoid large or bursty I/O after the sample baseline is taken |
| Snap refresh | `snapd.service`, `snapd.socket`, `snapd.refresh.timer` | Avoid network, unpacking, mount, and metadata work not used by the harness |
| IRQ policy | `irqbalance.service` | Prevent userspace from undoing the reviewed housekeeping-only IRQ layout |
| Rootful containers | `docker.service`, `docker.socket`, `containerd.service` | Only the unprivileged rootless Docker lifecycle is allowed |

Run any desired package upgrades, firmware checks, filesystem checks, and a
manual `fstrim` **before** applying the quiet-window policy. Never perform those
operations between qualification and the final bootstrap sample.

## Machine lifecycle for the next rental

Use a fresh state directory for every physical rental, for example:

```bash
export HC_PROFILE="$PWD/docs/testing/perf-host-profiles/ubuntu-24.04-reference-v1.json"
export HC_STATE="/var/lib/hydracache-perf/host-tuning-rental-2026-XX-XX"
```

The strict reference profile remains the default and the only profile used by
the 0.67.1 qualification/bootstrap campaign. Selecting the memory-only profile
is explicit, produces non-ship receipts, and must use its separate guarded
window launcher. Never pool its results with strict-profile evidence.

Do not reuse a state directory from another server. It contains the reversible
pre-state and the sample-family fingerprint.

### 1. Rent and identify the machine

Request bare metal with the same or reviewed-equivalent topology, at least six
physical cores, at least 16 GiB RAM, and NVMe. Install Ubuntu Server 24.04 LTS
x86_64. Provider “power off” normally does not stop bare-metal billing; verify
the provider terms and **delete/release** the machine when the campaign is done.

Before secrets or a runner are installed, archive the provider order/SKU, image
identifier, creation timestamp, region, and billing/deletion semantics. Do not
commit IP addresses, tokens, SSH private material, DMI serials, or runner
credentials.

### 2. Finish mutable operating-system work

Follow the package, Docker repository, user, SSH, rootless Docker, and GitHub
runner installation sections of `PERF_RUNNER_0_67_1.md`. Complete all of the
following before freeze:

- `apt update` and the intended `dist-upgrade`;
- exact Docker package selection and version recording;
- one reboot into the intended kernel;
- time synchronization confirmation;
- filesystem/firmware maintenance;
- creation of the unprivileged `github-runner` account;
- installation, but not continuous activation, of rootless Docker;
- preparation of the runner package and service instructions without entering
  a registration token or registering the service yet.

The runner account must not belong to `sudo`, `docker`, or `lxd`. Rootful Docker
must be stopped and disabled. Credentials never enter Git, shell history, logs,
receipts, or artifacts.

### 3. Check out the exact code and inspect the plan

Use the commit intended for qualification and require a clean worktree. Then:

```bash
sudo scripts/perf/prepare-reference-host.sh preflight \
  --profile "$HC_PROFILE" --state-dir "$HC_STATE"
sudo jq . "$HC_STATE/plan.json"
```

`preflight` is read-only apart from its receipt. It fails if OS, kernel family,
architecture, virtualization, capacity, storage, or cgroup contract is wrong.
It also records which allowlisted services exist and what would happen to each.
An incompatible machine is rejected; it is not silently “adapted” into a new
benchmark class.

### 4. Apply the quiet-window service policy

Stop the runner and rootless Docker first, then apply:

```bash
sudo scripts/perf/runner-service.sh offline
sudo -iu github-runner "$PWD/scripts/perf/rootless-docker.sh" stop
sudo scripts/perf/prepare-reference-host.sh apply-services \
  --profile "$HC_PROFILE" --state-dir "$HC_STATE"
```

`offline` disables and stops the runner service. This is a persistent baseline:
an unattended reboot must not reconnect the runner before a new frozen-state
check and an authorized dispatch.

Review `plan.json` and `applied.json`. The latter binds the applied operation to
the SHA-256 of the captured pre-state. If the policy needs amendment, make and
review a new profile version rather than editing receipts on the server.

### 5. Install CPU/IRQ isolation and reboot explicitly

```bash
sudo scripts/perf/prepare-reference-host.sh install-isolation \
  --profile "$HC_PROFILE" --state-dir "$HC_STATE"
sudo reboot
```

The wrapper prints `REBOOT_REQUIRED=true` and intentionally does not reboot.
After reconnecting, verify that no pending reboot/package operation remains.
The isolation drop-in is deliberately named to sort after provider-owned GRUB
drop-ins, and installation fails unless the fully resolved command line contains
every isolation argument exactly once.
The installed root-owned CPU policy sets the exact `performance` governor,
sets AMD P-State EPP to `performance` when the driver exposes it, and enforces
the reviewed idle-latency cap before the runner can start.
The reviewed contract is:

- SMT off;
- online CPUs `0-7`;
- measurement CPUs `1-4` on four distinct physical cores;
- housekeeping CPUs `0,5-7`;
- `isolcpus=domain,managed_irq,nohz,1-4`;
- `nohz_full=1-4`, `rcu_nocbs=1-4`, `irqaffinity=0,5-7`;
- performance governor, turbo available, and the reviewed idle-latency policy;
- runner and rootless Docker orchestration confined to housekeeping CPUs;
- no active IRQ affinity into measurement CPUs, except the narrowly reviewed
  dormant/unmapped NVMe case that must still have zero interrupts.

Before entering a GitHub registration token or completing the remaining runner
setup, execute the early IRQ layout probe:

```bash
sudo scripts/perf/prepare-reference-host.sh irq-layout-preflight \
  --profile "$HC_PROFILE" --state-dir "$HC_STATE"
```

It inventories effective IRQ affinity after the reviewed isolation reboot and
applies the unchanged absolute IRQ guard. A rejection means this rental
candidate is unsuitable under the current contract; do not attempt post-hoc
affinity writes or weaken the guard. Keep the non-evidence receipt, then decide
whether to release the server before spending time on runner registration.

Only after this probe passes, complete registration of exactly one runner with
the `hydracache-perf-v1` label. Keep the new service offline.

### 6. Verify, audit, and freeze

With runner and Docker offline and the repository clean:

```bash
sudo scripts/perf/prepare-reference-host.sh verify \
  --profile "$HC_PROFILE" --state-dir "$HC_STATE"
sudo scripts/perf/prepare-reference-host.sh freeze \
  --profile "$HC_PROFILE" --state-dir "$HC_STATE"
```

`verify` composes the service policy with
`provision-reference-isolation.sh verify`. `freeze` additionally runs the full
provisioned-host audit and produces:

On kernels where AMD P-State active mode does not expose the legacy global
`cpufreq/boost` knob, the audit remains fail-closed: it records boost as enabled
only when the CPU advertises CPB, every online policy uses `amd-pstate-epp`, and
the driver, CPU-info, and scaling maximum frequencies are identical. The
provisioning receipt records the selected turbo-policy backend.

- `plan.json` and `applied.json`;
- `freeze/host-freeze.json`;
- exact `packages.tsv`;
- exact systemd unit-file and active service/timer manifests;
- selected sysctl values;
- `lscpu.json`, privacy-safe selected `lsblk.json`, `/etc/os-release`, kernel
  command line, and apt holds;
- the existing privacy-preserving runner provisioning receipt.

Ubuntu 24.04 generic kernels can expose `kernel.sched_migration_cost_ns` through
the `CONFIG_SCHED_DEBUG` debugfs interface instead of `/proc/sys`. The freeze
collector keeps this tunable mandatory: it accepts only the root-owned
`/sys/kernel/debug/sched/migration_cost_ns` file on a real debugfs mount when
`CONFIG_SCHED_DEBUG=y`, and records the backend plus canonical locator in
`sysctls.tsv`. A missing source, a non-numeric value, or any backend/value drift
rejects the sample family.

Copy this root-owned directory unchanged into the qualification artifact. Keep
the original mode/owner metadata in the archived receipt. Hash the exported
archive before and after transfer.

### 7. Gate every dispatch against drift

Before qualification, each full-dress run, and every serialized bootstrap
dispatch, return the machine to its offline baseline and run:

```bash
sudo scripts/perf/prepare-reference-host.sh check-frozen \
  --profile "$HC_PROFILE" --state-dir "$HC_STATE"
```

The check fails on source/profile/plan/applied/provisioning digest drift, kernel
or command-line drift, package changes, systemd unit-file or active-state drift,
or selected sysctl drift. Do not “fix” the receipt to match a changed host. Find
the cause; either restore the frozen state or start a new qualification/sample
family.

Only then bring the runner online (which starts the still-disabled service for
this dispatch only), dispatch exactly one job, and avoid all other
performance jobs. The job starts rootless Docker only for the required target
phase and returns it to stopped state. After the job, take the runner offline,
run the drift check again, and validate the IRQ post-guard before accepting the
artifact.

### 8. Qualification and sample acceptance

The first successful job is a qualification for the exact main SHA. It is not a
bootstrap sample. Accept it only after verifying:

- run/job/artifact identity and exact source SHA;
- storage digest and complete original artifact;
- provisioning receipt, host attestation, tmpfs evidence, prebuild manifest,
  and service/freeze contract;
- pre/post runtime IRQ guards;
- source, runner, and prebuild hashes;
- SLO, repetitions, calibration, affinity, quota, privacy, zero-error, and
  fail-closed gates without weakening any threshold.

Next execute the exact full workload twice in `performance_0671_mode=full-dress`:

1. Dispatch the first run with `full_dress_predecessor_run_id` empty. Archive its
   `performance-0671-full-dress-receipt` artifact and verify that the receipt is
   qualification-only and non-promotable.
2. Only after accepting that artifact, dispatch the second run with
   `full_dress_predecessor_run_id=<first-run-id>`. It downloads the first
   byte-exact receipt and publishes `performance-0671-full-dress-admission` only
   when source SHA, runner fingerprint, immutable runner-provisioning receipt,
   prebuild contract, scenario contract, and distinct run identities all agree.

Then acquire exactly five **serialized successful** bootstrap samples with one
unchanged environment fingerprint. Every dispatch sets
`full_dress_admission_run_id=<second-full-dress-run-id>` and its exact
`bootstrap_sample_index`. Sample 1 leaves `bootstrap_predecessor_run_id` empty;
sample N (2-5) names the run id that produced accepted sample N-1. Never queue
the next run before the predecessor artifact exists. Failed, cancelled, unstable,
identity-mismatched, IRQ-contaminated, or drifted runs do not count and
cannot authorize a successor. Keep original downloads unchanged and build any
analyses from copies.

## What previous experiments taught us

These conclusions must influence the next campaign:

1. A completed workload is not automatically valid evidence. A post-run NVMe
   IRQ on a measurement CPU invalidated otherwise useful exploratory data.
2. CPU affinity must be checked from the effective process/container state.
   Docker `--cpuset-cpus` alone previously left a container with `0-15`; explicit
   process affinity plus a fail-closed readback was required.
3. Baselines must be taken after target readiness and immediately before the
   measured phase. Earlier baselines can miss dynamic IRQ allocation.
4. Filesystem placement matters. Measurement evidence and orchestration use
   tmpfs/housekeeping policy so evidence writes do not create measurement-CPU
   NVMe interrupts.
5. Identity equality gates must remain strict. The W4B/W5C manifest alias bug
   was fixed at the producing invariant; the equality check was not weakened.
6. Memory numbers need layers. Container RSS, process RSS/HWM, cgroup current/
   peak/limit, allocator behavior, and JVM heap are different metrics. Missing
   JVM telemetry must be marked unavailable, never replaced with container RSS.
7. Hazelcast Community comparisons must record the exact official image digest
   and the tested data structure/API. The completed exploratory work used the
   documented Hazelcast map/RESP-facing comparison contract, not an unspecified
   generic “Hazelcast cache”.
8. HydraCache, Redis, and Hazelcast results remain exploratory unless the whole
   causal evidence contract passes. The archived Stage 3 metric expansion was
   diagnostically valuable but its post-IRQ failure prevents release claims.
9. The runner should be online only while an intentional job needs it. Rootless
   Docker should be running only while a target phase needs it. This both reduces
   noise and narrows privilege/exposure.
10. Automatically queued follow-up jobs must be cancelled when they are not part
    of the reviewed sequence. Sample number is not evidence quality; a 13th or
    14th attempt is useful only if the previous rejection cause has been removed.

## Adaptation policy

The current profile is exact for the already reviewed reference class. A future
server with a different CPU topology, Ubuntu release, kernel ABI family, storage
layout, time-sync provider, or service set is handled as follows:

1. run `preflight` and retain the rejection/plan output;
2. create a new versioned profile rather than editing `v1`;
3. update isolation, IRQ, runner affinity, and host-audit checks together;
4. add/adjust regression tests in `crates/xtask/tests/perf_host_tuning.rs`;
5. run a fresh qualification and a fresh five-sample family;
6. never combine fingerprints from different profiles in one release claim.

This gives automation without allowing the script to normalize away important
differences between rented servers.

## End of rental

Before deleting the server:

1. stop the runner and rootless Docker;
2. download every accepted and rejected diagnostic artifact needed for the
   record, preserving originals and hashes;
3. archive the host tuning/freeze directory and provider deletion receipt;
4. revoke/remove the GitHub runner and any temporary credentials;
5. confirm no GitHub Actions job is queued for the label;
6. delete/release the bare-metal server in the provider console/API;
7. verify billing has stopped; powered-off is not assumed sufficient.

`restore` is available when the host must be returned to a general-purpose state
before deletion or reuse:

```bash
sudo scripts/perf/reference-host-tuning.sh restore \
  --profile "$HC_PROFILE" --state-dir "$HC_STATE"
```

Do not run `restore` between qualification and bootstrap samples.
