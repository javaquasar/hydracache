# How to Measure Cache Performance Without Measuring Noise

<!-- article-series:start hydracache-runtime -->
## HydraCache Runtime Series

This article is part of a practical series about building a Rust-native local-first cache runtime.

You are reading: Draft.

- [Part 1: Why Rust Needs Cache Semantics, Not Just Another Cache Map](https://medium.com/@artur.buzov/why-rust-needs-cache-semantics-not-just-another-cache-map-ecf3c4e01191)
- [Part 2: Single-flight Is Not an Optimization](https://medium.com/@artur.buzov/single-flight-is-not-an-optimization-85917bdbe77d)
- [Part 3: TTL Is Not Enough](https://medium.com/@artur.buzov/ttl-is-not-enough-ec4e96d89546)
- [Part 4: Local-first Distributed Invalidation](https://medium.com/@artur.buzov/local-first-distributed-invalidation-87bf0249e935)
- [Part 5: Typed Query Caching in Rust](https://medium.com/@artur.buzov/typed-query-caching-in-rust-aac4352599f0)
- Draft: How to Measure Cache Performance Without Measuring Noise

GitHub:

https://github.com/javaquasar/hydracache

crates.io:

https://crates.io/crates/hydracache
<!-- article-series:end -->

A benchmark does not measure only your program.

It measures your program, the load generator, the kernel scheduler, CPU power management, interrupt
routing, storage activity, network queues, background services, the build system, the telemetry
collector, and whatever else the machine decided to do during the measurement window.

That is why a fast benchmark is easy to run and a trustworthy benchmark is hard to produce.

During HydraCache's dedicated bare-metal campaign, the code that generated requests was only one
part of the work. We also had to make a rented Linux server boot reproducibly, separate measurement
work from orchestration, prove that NVMe interrupts did not enter the measured CPU set, retain every
failed sample, validate artifacts after long jobs, and reject results that looked plausible but did
not satisfy the frozen method.

The final campaign was useful precisely because the method was strict. It gave us stable local,
client-surface, RESP, control-plane, brownout, overload, and same-host Redis observations. It also
showed a real weakness: HydraCache was reasonably close to Redis without pipelining, but scaled much
less effectively at pipeline depth 10.

This article explains the method behind that conclusion and the operational lessons that made the
numbers believable.

## The result, before the methodology

The final comparison used five repeats, alternating execution order, one physical host, one frozen
software environment, one node-local loopback endpoint, and a pinned Redis toolchain.

| Operation | Pipeline | HydraCache median | Redis median | HydraCache / Redis |
| --- | ---: | ---: | ---: | ---: |
| GET | 1 | 59,630 req/s | 87,796 req/s | 67.9% |
| SET | 1 | 59,382 req/s | 87,719 req/s | 67.6% |
| GET | 10 | 136,426 req/s | 757,576 req/s | 18.1% |
| SET | 10 | 132,979 req/s | 746,269 req/s | 18.0% |

These numbers do not say that HydraCache is 68% of Redis in general. They describe four exact
same-box observations. They do not cover a production network, distributed value ownership,
cross-node failover, persistence modes, different payloads, or a different client implementation.

The useful conclusion is narrower and more actionable:

> The measured RESP implementation scales much less effectively with pipeline depth than Redis.

That conclusion survives because the comparison controlled host identity, order bias, errors,
repeats, and environment drift. Without those controls, a similar table would be decoration rather
than evidence.

The complete anonymized result, including host scope, budget boundaries, artifact hashes, and the
Rescue incident summary, is available in the
[HydraCache 0.67.1 AX42 report](https://github.com/javaquasar/hydracache/blob/main/docs/testing/perf-scenarios/0.67/results/ax42-reference-0.67.1-20260911.md).

## Start with a question, not a benchmark command

Before renting hardware, write down what the experiment is allowed to claim.

For a cache runtime, several surfaces can look similar while measuring different things:

- an embedded cache call measures process-local code;
- an in-process client router adds request translation but no socket;
- a loopback RESP endpoint adds parsing, encoding, connection management, and TCP;
- a control-plane cluster measures membership and metadata behavior, not distributed value-plane
  capacity;
- a same-host Redis comparison measures two implementations under one method, not universal
  product superiority.

Do not combine these into one headline throughput number. A local API result cannot be presented as
network capacity, and a three-daemon control-plane result cannot be multiplied into a distributed
data-plane claim.

Define the experiment contract before the first sample:

- exact source commit;
- exact scenario and workload digest;
- payload size and key distribution;
- warm-up and measurement windows;
- open-loop or closed-loop scheduling;
- offered-rate grid and backlog policy;
- repeat count;
- latency SLO and where latency starts;
- permitted error, timeout, and rejection counts;
- stability calculation and limit;
- host and toolchain identity;
- required telemetry and artifacts;
- conditions that invalidate the run.

If a threshold is chosen after seeing the candidate, it is not a release threshold. It is an
explanation of the candidate.

## Treat the benchmark as three separate planes

A useful mental model is to split the system into three planes:

```text
orchestration plane  -> build, Git, containers, artifacts, service control
measurement plane    -> prewarmed server and load process on isolated CPUs
evidence plane       -> raw samples, host state, receipts, hashes, validators
```

The orchestration plane is noisy. It reads files, launches processes, writes logs, talks to GitHub,
and starts containers. The measurement plane should be small, prebuilt, prewarmed, and isolated.
The evidence plane proves what actually ran and preserves enough raw data to diagnose rejection.

Many benchmark setups isolate the server process but forget the load generator, compiler, runner,
or artifact writer. That only moves the noise around. In our final layout:

- CPUs `1-4` ran the prewarmed measurement children;
- CPUs `0,5-7` handled the Actions runner, builds, Docker control work, storage submission, and
  artifact materialization;
- measurement output was written to tmpfs and copied to durable storage only after the timed work;
- IRQ guards ran before and after every important phase.

The exact CPU numbers are specific to one eight-core host. The principle is portable: select whole
physical cores for measurement, reserve enough cores for the operating system, and verify effective
placement instead of trusting the requested affinity.

## Inspect hardware before installing secrets

The cheapest failure is the one found before runner registration or long package installation.

Start with read-only inventory:

```bash
lscpu --json
free --bytes
lsblk --json --bytes --output NAME,TYPE,SIZE,ROTA,TRAN,MODEL
cat /proc/mdstat
systemd-detect-virt || true
lspci -nn | grep -Ei 'ethernet|network|non-volatile'
findmnt --output TARGET,SOURCE,FSTYPE,OPTIONS
```

Review at least these questions:

1. Is this really bare metal, or a VM with noisy-neighbor risk?
2. How many physical cores exist, and which logical CPUs are SMT siblings?
3. Is memory capacity sufficient without swap pressure?
4. Is the root filesystem on NVMe, SATA SSD, network block storage, or RAID?
5. Are all RAID members healthy?
6. What NIC and negotiated link speed are present?
7. Does the host use cgroup v2, and is any ancestor applying a CPU quota?
8. Can the chosen measurement CPUs be separated from normal IRQ and housekeeping work?

Record model and capacity for the report, but do not put IP addresses, MAC addresses, DMI UUIDs,
disk serial numbers, provider identifiers, tokens, or SSH material into Git or public artifacts.
Use privacy-preserving digests when identity must be bound mechanically.

The HydraCache reference host had an eight-core AMD processor, 64 GiB memory class, two NVMe
devices in healthy software RAID1, and an Intel gigabit NIC. That was enough for the intended
method, but the product SKU alone was not evidence. The exact topology and interrupt behavior still
had to pass admission.

## CPU topology matters more than the CPU name

Two machines with the same CPU model can behave differently because of firmware, kernel, SMT,
cooling, cgroup policy, or IRQ placement.

Build a CPU map that includes:

- socket, core, and thread sibling relationships;
- online and offline CPUs;
- NUMA nodes;
- current governor and frequency driver;
- turbo/boost policy;
- idle-state availability and exit latency;
- effective cgroup cpusets and quotas;
- current IRQ effective affinities.

For a small single-socket host, disabling SMT can simplify the contract. It removes sibling
competition and leaves one online logical CPU per physical core. That does not make SMT universally
bad. It makes this particular baseline easier to reproduce.

Pinning should be verified at runtime:

```bash
taskset --cpu-list --pid "$SERVER_PID"
taskset --cpu-list --pid "$LOADGEN_PID"
grep -E 'Cpus_allowed_list|Mems_allowed_list' /proc/"$SERVER_PID"/status
systemctl show your-runner.service --property=AllowedCPUs
```

A command such as `taskset --cpu-list 1-4 cargo run ...` is usually a mistake for strict latency
work. It pins compilation, linker work, Git/file reads, dependency loading, and the final process to
the same CPUs. Build on housekeeping CPUs, prefault the executable and its inputs there, and pin
only the already-built measurement child.

## Power management needs an explicit contract

Frequency and idle policy can move tail latency even when average CPU utilization looks stable.

At minimum, record:

- scaling driver;
- governor for every online CPU policy;
- configured and hardware maximum frequency;
- turbo/boost state;
- enabled idle states and their exit latency;
- thermal throttling counters before and after the run.

Do not infer AMD behavior from Intel-only files or vice versa. One HydraCache qualification passed
the shell audit but failed the Rust fingerprint because the host used active `amd-pstate-epp` and
did not expose the generic or Intel control files the second implementation expected. The fix was
not "assume turbo is enabled." Both implementations were changed to prove the same AMD contract:
CPB capability, the expected driver, and equal positive maximum frequencies for every policy.

The final host used the `performance` governor, enabled AMD P-state turbo under that proof, and
applied a maximum idle-latency cap of one microsecond to both measurement and housekeeping CPUs.
The latter matters: if housekeeping CPUs enter very deep idle states, timer and interrupt work can
wake late and still perturb an otherwise isolated experiment.

## IRQ isolation is where bare-metal benchmarking gets real

CPU affinity does not isolate a core from interrupts.

Inspect `/proc/interrupts` and each relevant IRQ's effective affinity:

```bash
cat /proc/interrupts

for irq_dir in /proc/irq/[0-9]*; do
  irq=${irq_dir##*/}
  printf '%s ' "$irq"
  cat "$irq_dir/effective_affinity_list" 2>/dev/null || true
done
```

Network and storage devices often use MSI-X with multiple queues. Some IRQ affinities can be moved;
managed IRQs may have kernel-controlled effective placement. NVMe adds another layer: blk-mq maps
submission/completion queues to CPUs, and a vector that looks dormant can become active only after
I/O is submitted from a particular CPU.

That creates a strict distinction:

- an IRQ merely has an effective affinity that intersects the measurement set;
- the related queue is mapped to a measurement CPU;
- the IRQ has actually fired on a measurement CPU during the current boot;
- its count changed during the measurement window.

HydraCache allowed one narrow managed-NVMe exception: a vector could have immutable effective
affinity on a measurement CPU only while its blk-mq CPU list was empty and its cumulative count was
zero. Any mapping, prior interrupt, or positive delta rejected that boot.

Why reject a delta of one? Because one interrupt proves the supposedly impossible storage path is
reachable. Raising the tolerance would hide the defect instead of isolating the measurement.

## The dangerous lesson from `pci=nomsi`

Our most expensive host incident came from trying to eliminate NVMe MSI-X globally.

The kernel argument `pci=nomsi` was added after a preflight showed a routed legacy interrupt pin.
That observation did not prove that the NVMe root filesystem could boot with MSI and MSI-X
disabled. The installed system failed to return after reboot and had to be repaired through the
provider's Rescue environment.

The recovery preserved the existing RAID and filesystem, removed only the rejected boot argument,
regenerated the boot configuration, and returned to the installed OS. No benchmark result from the
repair was promoted.

The permanent lesson is simple:

> A PCI capability listing is not a bootability proof.

Never experiment with global interrupt-mode kernel flags on the only machine holding irreplaceable
evidence. If such an experiment is necessary, use a separately reviewed throwaway host. For normal
benchmarking, constrain where storage I/O is submitted, observe queue mappings, and fail on runtime
IRQ deltas.

## Cold executables can turn a network test into a disk test

Another failure looked impossible at first. A network-only probe was pinned to a measurement CPU,
yet an NVMe interrupt fired on that CPU.

The network packet did not touch NVMe. The cold executable did.

`taskset` changes affinity before `execve`. If the executable, dynamic linker, shared library, or
configuration page is not resident, the kernel may read it from the root filesystem after the
process has already moved to the measurement CPU. That storage submission can activate the CPU's
managed NVMe queue.

The corrected order was:

1. resolve names and prepare inputs on housekeeping CPUs;
2. execute the exact binary once on housekeeping;
3. prefault regular-file inputs and required libraries;
4. capture the absolute IRQ state and delta baseline;
5. start only the prewarmed measurement child on the isolated CPUs;
6. keep every durable write on housekeeping CPUs;
7. verify IRQ counts immediately after the phase and after an idle window.

This is also why tmpfs helps. It does not make the workload faster by magic; it keeps evidence writes
from selecting storage queues during the measurement. Copy the evidence to durable storage after
the final guard.

## RAID health and storage placement are separate checks

Healthy RAID does not imply quiet storage, and quiet storage does not imply healthy RAID.

Before every campaign, verify that all expected members are active. For a reviewed two-device RAID1
layout, `[UU]` is required. A degraded array may still serve data, but rebuild activity, changed
latency, and reduced fault tolerance invalidate the reference environment.

Then distinguish these activities:

- product I/O that is intentionally part of the workload;
- page faults and executable loading;
- build and package-manager I/O;
- container image pulls and overlay writes;
- logs and artifact creation;
- RAID checks, trim, scrub, or repair activity;
- telemetry reads from sysfs and procfs.

If the benchmark is intended to measure an in-memory or network path, storage activity on the
measurement CPUs is contamination. If storage is the subject of the benchmark, it belongs in the
scenario and must be measured explicitly rather than treated as background noise.

## Quiet the operating system, but keep the policy reversible

A dedicated host is not automatically idle. Package timers, filesystem maintenance, telemetry
agents, container daemons, login sessions, and provider tooling can wake during a long run.

Create a reviewed allowlist of services that may be stopped, disabled, or masked for the measurement
window. HydraCache quieted package maintenance, update services, trim, selected documentation and
news timers, rootful Docker, and other known background actors. Rootless Docker was started only for
the pinned Redis phase. The runner itself remained offline except for one controller-owned job.

Do not write a script that disables every service it finds. That is difficult to review and may
remove networking, timekeeping, access, or safety services. Record the previous state and make the
policy reversible.

The environment freeze should include at least:

- OS release and exact kernel;
- kernel command line;
- package manifest digest;
- systemd unit-file and active-state digests;
- selected sysctls;
- CPU topology and policy;
- block-device topology and storage identity digest;
- runner provisioning receipt;
- source commit and scenario/profile digests.

After freeze, package installation, kernel updates, sysctl changes, service drift, or a different
source commit should fail before dispatch. Do all mutable work first.

## Use a lifecycle, not a shell history

The final HydraCache controller used an explicit campaign lifecycle:

```text
prepare -> reboot -> freeze -> IRQ burn-in -> run -> verify -> close
```

Each campaign ID was immutable. A rejected campaign was not edited and retried in place. The next
attempt used a new ID and retained the old logs and samples.

The phases have distinct purposes:

### Prepare

- verify exact clean source;
- install all required dependencies;
- build and hash binaries;
- pull pinned container images;
- ensure authentication needed for artifact retrieval works;
- apply the reviewed service and CPU policy;
- record the pre-reboot state.

### Reboot

- activate kernel CPU/IRQ settings;
- clear cumulative IRQ state from earlier attempts;
- prove the machine returns within a bounded window;
- verify RAID, network, source, and runner-offline state.

### Freeze

- capture the exact environment;
- verify no unapproved drift;
- seal host-state digests;
- prohibit further package or configuration changes.

### IRQ burn-in

- run long enough to expose dynamically allocated queues and delayed background work;
- generate reviewed network stimulus on measurement CPUs;
- submit any read-only storage stimulus only from housekeeping CPUs;
- require zero forbidden mapping or counter delta.

HydraCache used 900 seconds. The important property is not that every project must use 900 seconds;
it is that the duration is committed before the candidate and covers the dynamic behavior the
admission is intended to reveal.

### Run and verify

- enable exactly one serialized runner;
- perform a bounded artifact transport canary before the expensive job;
- run the preflight again inside the job;
- execute the ordered measurement families;
- disable the runner immediately afterward;
- verify frozen state and IRQ deltas before accepting output;
- bind artifact, run ID, source, binary, scenario, and host receipts.

### Close

- copy the complete campaign and host-state archives off the rented host;
- retain the original artifact ZIP, not only extracted files;
- verify SHA-256, ZIP/tar readability, and structured JSON/XML parsing;
- commit only anonymized conclusions;
- scan the Git diff for secrets and hardware identifiers;
- revoke the runner registration before authorizing server deletion.

## Open-loop load reveals overload; closed-loop load can hide it

In a closed-loop benchmark, each client waits for a response before sending more work. When the
server slows down, request generation also slows down. Throughput may look stable precisely because
the clients stopped applying pressure.

An open-loop benchmark schedules requests at a fixed offered rate. Latency begins at scheduled send
time, so queueing and missed schedules remain visible. This makes it possible to define sustainable
capacity as the highest offered rate satisfying all of the following:

- achieved rate remains close to offered rate;
- p99 stays within the committed SLO;
- errors, timeouts, and unexpected rejections remain zero or within an explicit policy;
- backlog drains within a bounded interval;
- repeat stability passes.

Do not report the fastest attempted rate as capacity. A point that accepts connections while
building an unbounded queue has not sustained that rate.

Closed-loop tools are still useful for exact paired comparisons such as `redis-benchmark`, provided
the report says what was measured. They should not silently replace the open-loop capacity method.

## Tail latency needs enough observations

At p99, only one percent of observations describe the tail. A repeat with 10,000 requests contains
roughly 100 p99-tail observations. That can be dominated by a small number of scheduler events.

HydraCache initially used 10,000 observations for each RESP connection/pipeline point. All six
points failed the stability rule. Raising the count to 200,000 produced roughly 2,000 p99-tail
observations and distinguished a sampling defect from real loopback scheduling variance.

The correct response to an unstable tail is not automatically "increase the tolerance." First:

1. preserve the raw repeat samples;
2. verify errors, backlog, CPU placement, and IRQ state;
3. inspect whether the window contains enough independent work;
4. increase the precommitted window if one scheduler event dominates it;
5. use a new campaign ID and exact source commit;
6. change a threshold only with an explicit methodological reason.

For five samples, HydraCache records the median and a relative range:

```text
robust_spread = (maximum - minimum) / median
```

The name is less important than freezing the formula and limit before the candidate. Every rejected
spread should include the raw samples so a reviewer can distinguish an outlier from a systematically
short window.

## Warm-up must cover the path you will measure

A generic sleep is not always a warm-up.

Warm the exact behavior:

- binary and shared-library pages;
- allocator and runtime initialization;
- connection establishment and protocol negotiation;
- parser and serialization paths;
- key/value working set;
- container image and executable inputs;
- control-plane membership before event timing;
- JIT compilation for comparison systems that use a JIT;
- caches that are intentionally warm in the reported scenario.

Do not include target setup, key reset, process startup, or container image pulls in a steady-state
latency window unless startup is explicitly the subject of the scenario.

Warm-up length and exclusion must appear in the scenario contract. Otherwise one implementation
may receive warm pages and established connections while another pays cold-start cost.

## Alternate comparison order

Running all HydraCache repeats and then all Redis repeats creates a time-order confound. Temperature,
kernel state, allocator state, and background activity may drift across the batch.

Alternate order across repeats:

```text
repeat 1: HydraCache -> Redis
repeat 2: Redis -> HydraCache
repeat 3: HydraCache -> Redis
...
```

Report medians for each implementation, ratios for paired repeats, stability of both raw series,
and order bias. A stable ratio with unstable raw results is suspicious; so is a large difference
between HydraCache-first and Redis-first subsets.

Pin tool versions and container image digests. "Redis latest" is not a reproducible comparison.

## Metrics: collect enough to explain, not enough to interfere

The primary metrics come from the load generator:

- offered, achieved, and goodput rates;
- latency histogram including p50, p95, p99, and maximum;
- scheduled-send delay for open-loop work;
- errors by bounded category;
- timeouts and rejections;
- in-flight requests and backlog-drain time;
- exact operation, payload, concurrency, connection, and pipeline dimensions.

Host telemetry explains why a result changed:

- per-CPU utilization, run queue, context switches, migrations, and softirqs;
- effective CPU affinity and cgroup quota/throttling;
- frequency, idle-state residency, temperature, and thermal throttling;
- per-IRQ counters and effective affinity before/after each phase;
- block-device IOPS, bandwidth, queue depth, await time, and RAID state;
- NIC bytes, packets, drops, errors, and queue counters;
- process RSS/HWM, anonymous/file-backed memory, faults, threads, and file descriptors;
- cgroup memory current/peak and CPU accounting;
- CPU, memory, and I/O pressure-stall information.

Useful Linux tools include `mpstat`, `pidstat`, `iostat`, `sar`, `perf stat`, `ss`, `ethtool`, and
the relevant files under `/proc` and `/sys`. But collectors consume CPU and generate reads and
writes. Pin them to housekeeping CPUs, choose a low fixed sampling frequency, buffer output in
memory, and measure their overhead separately.

Avoid unbounded metric labels such as key, request ID, session ID, or raw tenant. Detailed identity
belongs in a protected diagnostic artifact, not a time-series label.

## Validate the measurement pipeline before the long run

Several failures in the HydraCache campaign happened after useful work because the surrounding
pipeline was not yet exercised end to end:

- a required Python or Rust audit tool was missing only at final receipt generation;
- artifact delivery temporarily failed after a successful long job;
- a controller child filled a stdout pipe and stalled;
- a validator rejected a legitimate sparse result shape;
- a measurement budget was shorter than the operation it was meant to observe;
- the runner service was healthy locally but had the wrong repository label;
- a stale provisioning receipt described the right machine at the wrong source commit.

The fixes follow one principle: move cheap failures earlier without weakening late validation.

Before the expensive phase:

- run every required tool with a version/probe command;
- validate pinned downloads and CA certificates;
- build the exact binaries and record their hashes;
- run a bounded artifact download canary;
- prove the repository sees one idle runner with the exact label;
- exercise receipt generation on smoke data;
- test controller stdout/stderr draining and watchdog behavior;
- validate the scenario and result schemas;
- confirm enough disk and tmpfs capacity.

If artifact retrieval fails after the workflow succeeds, preserve the run identity and enter an
`awaiting-artifacts` state. Retry the same run's artifact rather than repeating hours of measurement.
Only an expired, corrupt, identity-mismatched, or genuinely failed artifact should make the
measurement terminally unusable.

## A failed run can still be valuable

Fail-closed does not mean throw the data away.

The campaign that recorded one RESP p99 value above the 5 ms SLO was ineligible as release evidence,
but it showed genuine tail variability at the selected 50,000 operations/s knee. Another campaign
completed all long measurements and then failed because workspace evidence tools were absent. Its
performance data remained diagnostic, while the release verdict remained red.

Classify failures explicitly:

| Failure class | Keep measurements? | Promote? | Next action |
| --- | --- | --- | --- |
| Product error, timeout, SLO, or backlog failure | yes | no | diagnose product/workload behavior |
| Unstable repeat spread | yes | no | inspect raw samples and window sufficiency |
| IRQ, host drift, thermal, RAID, or quota contamination | yes, as diagnostics | no | repair/reboot, then use a new campaign |
| Missing late tool or artifact transport outage | yes | no until evidence is recovered | preflight earlier or resume retrieval |
| Source, runner, scenario, or artifact identity mismatch | retain everything | never | reject and restore exact identity |
| Complete green run with matching receipts | yes | yes, within claim scope | archive and publish |

Never delete the uncomfortable sample and rerun until the graph looks smooth. Retained failures are
how the method improves.

## The final HydraCache campaign

After the host and controller corrections, the final frozen candidate completed the whole chain:

- bare-metal admission and 900-second IRQ burn-in;
- exact source and binary binding;
- local, client-surface, RESP, Redis, control-plane, grid, brownout, and overload evidence;
- all 19 numerical budget checks;
- expected-red canaries and release aggregation;
- all 3,025 workspace tests used by the final receipt;
- original artifact ZIP and host-state archives copied off-host and verified.

Selected release checks were:

| Check | Candidate | Boundary |
| --- | ---: | ---: |
| Embedded capacity | 20,000 ops/s | at least 18,000 ops/s |
| Embedded p99 | 1,917 us | at most 2,137.3 us |
| Client-surface capacity | 24,989.1 ops/s | at least 22,490.6 ops/s |
| Client-surface p99 | 2,085 us | at most 2,269.3 us |
| Node RESP capacity | 49,948.1 ops/s | at least 44,954.9 ops/s |
| Node RESP p99 | 3,769 us | at most 3,890.7 us |
| RESP overload goodput | 44,374.5 ops/s | at least 30,153.9 ops/s |

These values are not a universal score. They prove that one exact candidate remained inside a
previously reviewed, five-sample contract on one exact host.

The comparison table then adds a product direction: pipeline depth is where RESP optimization work
is most likely to pay off. A follow-up optimization should be tested against the same method, and it
should preserve errors, tail latency, fairness, and protocol semantics instead of optimizing only
the throughput numerator.

## A practical checklist

Before the server exists:

- define surfaces, workloads, SLOs, repeats, and rejection rules;
- choose a host class and a deletion/retention owner;
- create a dedicated SSH key and secret-handling plan;
- pin OS, toolchain, external tools, and container digests.

Before runner registration:

- prove bare metal, physical-core topology, memory, NVMe, RAID, NIC, and cgroup policy;
- reboot into the intended kernel and inspect real IRQ layout;
- reject unsuitable hardware before installing repository credentials.

Before freeze:

- finish packages and updates;
- create an unprivileged runner account;
- build and prewarm tools;
- quiet only reviewed services;
- configure measurement and housekeeping CPUs;
- verify power, idle, thermal, quota, and IRQ policies.

Before every long run:

- verify exact clean source and scenario digests;
- verify the frozen environment;
- run the artifact transport canary;
- confirm one offline, idle, correctly labelled runner;
- pass the IRQ burn-in and capture a fresh delta baseline;
- test all late evidence tools on smoke input.

After every run:

- take the runner offline;
- verify host drift and IRQ deltas before accepting results;
- retain raw samples even when rejected;
- bind source, binary, scenario, host, run, and artifact identity;
- copy original archives off-host and verify hashes and readability;
- publish only anonymized, scope-limited conclusions.

## The benchmark is part of the product

Performance work often starts with code and ends with a chart. Reliable performance work starts
with a claim and ends with an evidence chain.

The chain matters because the most convincing wrong result is usually not obviously wrong. It is a
clean graph produced by a noisy machine, a closed-loop client hiding overload, a p99 based on too
few tail observations, an interrupt routed onto the measured core, a comparison run in fixed order,
or an artifact that cannot prove which binary actually ran.

The goal is not to eliminate all variance. That is impossible on a general-purpose Linux system.
The goal is to make variance observable, constrain the major sources, reject contaminated runs, and
retain enough evidence to explain every decision.

Only then does a comparison point to engineering work instead of merely producing a number.

## Reproduction resources

The repository keeps the executable version of this methodology:

- [next-rental reference-host playbook](https://github.com/javaquasar/hydracache/blob/main/docs/testing/PERF_RUNNER_NEXT_RENTAL_PLAYBOOK.md);
- [dedicated runner runbook](https://github.com/javaquasar/hydracache/blob/main/docs/testing/PERF_RUNNER_0_67_1.md);
- [NVMe IRQ incident and Rescue recovery](https://github.com/javaquasar/hydracache/blob/main/docs/testing/PERF_RUNNER_0_67_1_NVME_IRQ_INCIDENT.md);
- [review, activation, and frozen-candidate protocol](https://github.com/javaquasar/hydracache/blob/main/docs/testing/PERF_REFERENCE_0_67_1_REVIEW_AND_ACTIVATION.md);
- [host profiles and scenario contracts](https://github.com/javaquasar/hydracache/tree/main/docs/testing).

Those files are deliberately stricter and more mechanical than this article. Use the article to
understand the method and the runbooks to execute it.
