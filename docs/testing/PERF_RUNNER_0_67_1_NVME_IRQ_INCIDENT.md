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

## Campaign and source recovery rules

- Rejected campaigns are immutable diagnostics. Campaign A/B failures remain
  retained; the corrected run starts as campaign C (or the next unused ID).
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

## Evidence retained from this incident

The durable conclusions are encoded in three places:

- the reference and memory-only host profiles declare the storage-I/O CPU and
  zero-delta contract;
- `reference-host-irq-burn-in.sh` prewarms the network executable before its
  baseline and submits raw NVMe reads only from housekeeping CPUs;
- this incident runbook records the boot failure, Rescue boundary, stale-source
  hazard, and the reason a one-interrupt delta remains fail-closed.

