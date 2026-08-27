# Reference runner NVMe IRQ incident and recovery

Status: retained operational incident from the 2026-08-25 AX42 qualification.
This document is a recovery and prevention runbook, not qualification,
bootstrap, or ship evidence.

## What happened

The first attempt to eliminate per-CPU managed NVMe MSI-X queues added the
global kernel argument `pci=nomsi`. The pre-reboot checks proved only that each
controller advertised a routed legacy interrupt pin. They did **not** prove
that the NVMe boot path could complete with MSI/MSI-X disabled. The machine did
not return over SSH after reboot and had to be repaired from the provider's
Rescue environment.

The unsafe change was merged by PR #109 at `04134905`, reverted by PR #110 at
`a1e369e8`, and replaced by the managed-IRQ storage-placement contract in PR
#111 at `e967bfb2`.

## Permanent conclusions

1. **Never add global `pci=nomsi` to this NVMe-root reference profile.** A PCI
   `Interrupt: pin ... routed` line is not a bootability proof. The only
   acceptable future exception is a separately reviewed throwaway-host boot
   experiment that does not contain the campaign checkout or evidence.
2. `isolcpus=managed_irq` is best-effort. A managed NVMe vector may have an
   immutable effective affinity on CPUs `1-4`; it is admissible only while its
   blk-mq `cpu_list` is empty and its cumulative interrupt counter is zero.
3. Storage submission is restricted to housekeeping CPUs `0,5-7`. Runtime
   delta guards must reject any new mapping, affinity drift, or counter change
   on a dormant measurement-CPU NVMe vector.
4. `taskset` changes affinity **before** `execve`. Starting a cold executable on
   a measurement CPU can page its binary or shared libraries from the root
   filesystem and activate that CPU's NVMe queue. Network probes and workload
   binaries must be prewarmed on housekeeping before the IRQ baseline.
5. IRQ counters are cumulative for the boot. Once a forbidden vector fires,
   the allocation is rejected for that boot. Do not reset a receipt or reuse a
   rejected campaign directory; fix the cause, reboot, and use a new campaign
   ID.
6. Host audit results are meaningful only when the trusted checkout SHA equals
   the intended exact green `origin/main` SHA. A repaired kernel with a stale
   checkout can otherwise run an obsolete guard and produce a misleading
   diagnosis.
7. Install every campaign dependency, including `gh`, and complete
   `gh auth status` before `freeze`. Package installation after freeze changes
   the package-manifest digest, so `check-frozen` must reject the campaign
   before dispatch. Close that campaign and restart with a new ID; do not
   rewrite its frozen receipt.
8. Do not freeze transient logind unit names such as `session-*.scope` into the
   persistent systemd unit-file manifest. A new SSH session changes that name
   without changing host configuration. Exclude only rows whose unit-file
   state is exactly `transient`; keep permanent unit files and the separately
   captured active service/timer state fail-closed.
9. Verify the repository-side runner registration before dispatch. The local
   service can be healthy and online while GitHub still assigns a quarantine
   custom label, leaving the intended job queued indefinitely. Require one
   idle runner named `hydracache-perf-v1` with custom label
   `hydracache-perf-v1`; do not infer this from the local `.runner` file.
10. The canonical `/var/lib/hydracache-perf/runner-provisioned.json` must be
    republished from the exact-SHA provisioning audit before host admission is
    declared ready. A stale receipt can describe the same machine correctly
    but must still fail because its `source_commit` belongs to an older main.
    Archive the previous root-owned receipt and install the new one atomically.
11. The shell host audit and the Rust runner fingerprint must recognize the
    same turbo-policy backends. On AMD `amd-pstate` active mode, a missing
    global `cpufreq/boost` file is normal and must not fall through to Intel's
    `intel_pstate/no_turbo`. Require the CPU `cpb` capability, driver
    `amd-pstate-epp`, and equal positive `amd_pstate_max_freq`,
    `cpuinfo_max_freq`, and `scaling_max_freq` values for every cpufreq policy.
    Any missing policy, different maximum, or unsupported status remains a
    fail-closed qualification rejection.

## Recognition checklist

Before any reboot or campaign `freeze`, record and review:

```bash
git -C /opt/hydracache/trusted-main rev-parse HEAD
cat /proc/cmdline
cat /proc/mdstat
systemctl is-enabled actions.runner.javaquasar-hydracache.hydracache-perf-v1.service || true
systemctl is-active actions.runner.javaquasar-hydracache.hydracache-perf-v1.service || true
systemctl is-active docker.service || true
```

Required state:

- checkout equals the admitted exact green `main` SHA;
- `/proc/cmdline` does not contain `pci=nomsi`;
- every RAID member is healthy (`[UU]` for the reviewed two-device layout);
- the Actions runner is disabled and inactive outside one controller-owned job;
- rootful Docker is inactive;
- the absolute guard reports only explicitly verified
  `dormant-unmapped-nvme` exceptions;
- a delta baseline survives the reviewed burn-in with zero monitored changes.

Treat an SSH timeout immediately after an intentional reboot as a possible boot
failure, not as evidence that the host is merely slow. Use the provider console
or Rescue path once the normal bounded reboot window has expired.

## Rescue recovery for an unbootable GRUB argument

These steps are deliberately generic and contain no provider password, IP,
host identifier, or private key.

1. Keep the runner offline. Activate the provider's Linux Rescue environment
   and perform one reset so the next boot enters Rescue.
2. Use a dedicated temporary `known_hosts` file. Rescue and the installed OS
   legitimately present different SSH host keys; do not delete the normal
   production-host identity to suppress the warning.
3. Install only the operator's **public** SSH key in Rescue. Never paste or
   commit the private key or Rescue password.
4. Inspect `/proc/mdstat` and `lsblk`. Mount the existing root and boot RAID
   devices without formatting, recreating RAID, or running an installer.
5. Back up the affected GRUB drop-in on the mounted root. Remove only the
   rejected argument (`pci=nomsi` in this incident); preserve `nosmt`,
   `isolcpus`, `nohz_full`, `rcu_nocbs`, and `irqaffinity`.
6. Bind-mount `/dev`, `/proc`, `/sys`, and `/run`, chroot into the installed
   system, run `update-grub`, and verify the generated `grub.cfg` no longer
   contains the rejected argument.
7. Disable Rescue for the next boot and reboot. Verify the recognition
   checklist above before changing any campaign state.

Do not reinstall the OS as the first response: that destroys diagnostic state
and can erase evidence. Reinstallation is a separate explicit decision only
after RAID and filesystem recovery have been ruled out.

## Cold-exec IRQ reproduction and prevention

The second failure was subtler. A network-only test launched
`taskset --cpu-list <measurement-cpu> ping ...` after taking the IRQ baseline.
One dormant NVMe vector gained a single interrupt. The network device was not
misrouted: the cold `ping` executable was paged in after `taskset` had already
moved the process to the measurement CPU.

The reviewed order is therefore:

1. resolve the numeric network target;
2. run the exact `ping` surface once on housekeeping CPU `0`;
3. run the absolute IRQ guard;
4. capture the delta baseline;
5. run raw NVMe reads only on `0,5-7` and network stimulus on `1-4`;
6. require zero measurement-CPU IRQ delta immediately and after the idle
   window.

Do not "fix" this by allowing a small positive delta. A delta of one proved
that the forbidden storage path was reachable; tolerance would hide the
problem rather than isolate it.

## AMD turbo-policy preflight mismatch

One qualification passed the shell host audit but failed the Rust prebuild
with `turbo policy probe failed: No such file or directory`. The machine used
active `amd-pstate-epp`; it exposed neither the generic global boost file nor
Intel's `no_turbo` file. The shell audit already had an AMD-specific proof, but
the Rust fingerprint did not, so the two admission layers disagreed.

The correction is a shared semantic contract, not a fallback that assumes
turbo is enabled. For active AMD P-state, both layers must prove CPB support and
exact equality of the driver, hardware, and configured maximum frequency for
every policy. A new campaign and a new exact green `main` SHA are required
after this code change; an earlier frozen campaign cannot be resumed against
the changed fingerprint implementation.

## Campaign and source recovery rules

- Rejected campaigns are immutable diagnostics. Each corrected run starts with
  the next unused campaign ID.
- Before another campaign publishes host admission, close the prior campaign.
  Close retires the canonical admission only when its campaign ID, source SHA,
  receipt digest, and bundle digest match the closing campaign; a mismatch is
  fail-closed and must never be removed blindly.
- Never rerun a stale helper pinned to a reverted SHA. Revoke or rename it and
  stage a new helper with the exact admission SHA and admission-file SHA-256.
- Generate procurement admission only from a clean checkout at exact
  `origin/main` after the exact post-merge CI run is `completed/success`.
- A Rescue repair restores bootability only. It does not create performance
  evidence and does not waive `prepare`, reboot, `freeze`, burn-in, or the
  qualification chain.
- If any password is pasted into a chat, terminal transcript, issue, or build
  log, treat it as compromised and rotate it. Documentation must contain only
  placeholders and public-key material may be shown only when needed.

## Long-running controller sudo lease

The AX42 campaign `hc0671-ax42-20260826-i` exposed a separate orchestration
failure after its first full-dress job was rejected. The GitHub run had lasted
longer than sudo's timestamp timeout, so the controller blocked indefinitely
while taking the runner offline in its cleanup path.

The controller now authenticates sudo once when a mutating command starts,
refreshes that timestamp only while the controller remains alive, and invokes
every privileged operation with `sudo -n`. An expired or lost credential must
therefore reject the campaign instead of waiting for input in a detached SSH
session. Do not remove `-n`, bypass the bounded lease, or replace it with a
NOPASSWD rule for scripts from a mutable checkout.

## Preserve samples when stability rejects evidence

The first full-dress run reached W2 but rejected the `concurrent_inflight=1000`
point because its robust spread was just above the committed limit. The error
reported the ratio and dimensions but omitted the underlying repeat samples.
That made a valid fail-closed rejection unnecessarily hard to diagnose: the
artifact could not distinguish one scheduler outlier from a measurement window
that was systematically too short.

Scalar spread failures must therefore include their finite repeat samples in
the validation diagnostic. The report remains rejected and no invalid JSON is
published as canonical evidence; the extra values are diagnostic only. Never
raise a tolerance or retry inside the rejected campaign based only on the
aggregate ratio. First retain the samples, explain the variance, change the
committed scenario only if the data supports it, then start a new campaign at
an exact green `main` SHA.

Campaign `hc0671-ax42-20260826-i` subsequently proved that the RESP connection
and pipeline matrix used only 10,000 observations per repeat. Its p99 therefore
represented roughly 100 tail observations, and all six matrix points failed
the unchanged 15% repeat-stability contract even though the much longer A/B/C
capacity knees were stable and every IRQ guard passed. The reviewed correction
keeps the 15% limit, raises the matrix to 200,000 observations at 2,000 offered
pipeline exchanges per second, and records that offered rate in both the
scenario digest and point dimensions. This supplies roughly 2,000 p99-tail
observations while keeping the pipeline-10 logical rate at 20,000 operations
per second, below the accepted 50,000 operations-per-second A/B/C knee.

The later immutable campaign `hc0671-ax42-20260826-q` ran that corrected
200,000-exchange matrix. All six points completed without errors, timeouts,
rejections, backlog, or IRQ-policy violations, but five repeat sets had robust
spread between `0.1653` and `0.2138`. Their retained p99 samples stayed in the
same approximately 1.7--2.5 ms scheduler-latency band; increasing the sample
count by 20x removed the earlier sampling defect but did not make loopback TCP
p99 satisfy the generic 15% scalar limit. The reviewed W3 matrix contract uses
a 25% robust-spread limit for this scheduled-send p99 measurement only. It
keeps five repeats, 200,000 exchanges per repeat, and all fail-closed transport
and IRQ checks. This is not a retry waiver: a point above 25% still rejects the
campaign and preserves its samples for diagnosis.

## Aggregate synchronized single-flight bursts

Campaign `hc0671-ax42-20260826-j` passed host admission and qualification but
rejected W1 before reaching the corrected RESP matrix. The
`hot_key_single_flight_miss_stampede_cost` repeat measured one synchronized
four-request cold-miss burst whose useful interval was only a few
milliseconds. The observed samples (`1875.73`, `727.70`, and `1903.18`
operations per second) showed a single scheduler delay dominating one repeat,
while every burst still proved four misses, zero hits, and exactly one loader.

The correction keeps three independently validated samples and the unchanged
15% robust-spread limit. Each reference sample now aggregates 64 separately
reset cold-miss bursts, checks the single-flight invariant after every burst,
and records the burst count and total loader/miss counts in the point
dimensions. Reset and target setup remain outside the timed interval. Smoke
coverage still uses one burst so routine CI stays fast. A rejected campaign is
not retried; the new window requires a new exact green `main` SHA and campaign.

Campaign `hc0671-ax42-20260826-k` proved that correction: W1 passed and the
core gate advanced to W2. W2 then rejected only the
`concurrent_inflight=1000` point with samples `104370.12`, `103239.14`, and
`122627.57` operations per second (18.58% spread versus the unchanged 15%
limit). The committed 4,000-operation window supplied only four request waves
at that concurrency, so one wave could dominate a repeat. The reviewed W2
correction raises the fixed reference window to 100,000 operations, providing
100 waves at `inflight=1000`, and records the window in every point dimension.
It does not relax the spread limit or retry the rejected campaign.

## Evidence retained from this incident

The durable conclusions are encoded in three places:

- the reference and memory-only host profiles declare the storage-I/O CPU and
  zero-delta contract;
- `reference-host-irq-burn-in.sh` prewarms the network executable before its
  baseline and submits raw NVMe reads only from housekeeping CPUs;
- this incident runbook records the boot failure, Rescue boundary, stale-source
  hazard, and the reason a one-interrupt delta remains fail-closed.
