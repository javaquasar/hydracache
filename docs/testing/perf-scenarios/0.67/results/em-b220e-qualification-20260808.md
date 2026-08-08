# EM-B220E-NVMe 0.67.1 qualification attempt — 2026-08-08

Status: **host allocation rejected before qualification**. No qualification,
full-dress, bootstrap, or ship evidence was produced.

This report records the preparation, orchestration fixes, host-admission
attempts, retained diagnostics, and final decision for the rented Scaleway
EM-B220E-NVMe allocation. It deliberately omits the public address, provider
identifiers, credentials, SSH material, runner tokens, and raw DMI values.

## Machine and frozen reference class

| Property | Observed value |
|---|---|
| Provider SKU | Scaleway EM-B220E-NVMe bare metal |
| Operating system | Ubuntu 24.04.4 LTS x86_64 |
| Kernel | `6.8.0-137-generic` |
| CPU | AMD EPYC 7232P, 8 cores / 16 threads before SMT isolation |
| Memory | 64 GiB |
| Storage | Two NVMe namespaces in clean RAID1 arrays |
| Network | 10 Gbit/s provider network class |
| Measurement CPUs | `1-4` |
| Housekeeping CPUs | `0,5-7` |
| Isolation | `nosmt`, `isolcpus=domain,managed_irq,nohz,1-4`, `nohz_full=1-4`, `rcu_nocbs=1-4`, `irqaffinity=0,5-7` |
| CPU policy | performance governor; reviewed 1 µs measurement and housekeeping idle-latency caps |

Both RAID1 arrays were checked as `[UU]` before each admission attempt. The
runner account remained unprivileged and outside the `sudo`, `docker`, and
`lxd` groups. Rootful Docker/containerd and rootless Docker were stopped during
host admission. Package timers, snap refresh, periodic storage work,
`irqbalance`, and other reviewed noise sources were handled only by the
versioned allowlist in the Ubuntu 24.04 reference profile.

Provider-image `cloud-init` units reported no datasource after reboot, and
`mdmonitor` reported its provider-image mail/alert configuration gap. These
states were investigated rather than masked. Failed-state markers were reset
only after RAID health and required services had been checked. The runner was
forced offline immediately after every reboot and its remote `offline` /
`busy=false` state was verified through the GitHub API.

## Code and CI identities

| Transition | Commit or run | Result |
|---|---|---|
| Initial exact main | `056b7202051b16cfc634e4ecd4470799f0fbeba0` | Green before rental |
| Scheduler-tunable fix PR | PR #78, head `bf85172c45773085c181c0458775d7a867d2128a`, run `31271482476` | Rust, MSRV 1.88.0, and Shared Tripwire passed |
| Main after PR #78 | `d23dd5a97d4fa2031bc3a2343451777f727faa71`, run `31272309830` | Exact post-merge CI passed |
| Ubuntu iputils fix PR | PR #79, head `3bdaa215ce09c3ebd9b370667302103f926f7cda`, run `31273969979` | Rust, MSRV 1.88.0, and Shared Tripwire passed |
| Final exact main | `e01d2cc48eb0c885b3d31b9fa38fbbb0a64c5d56`, run `31274857599` | Exact post-merge CI passed |

Every new main commit received a fresh procurement admission and a fresh
offline runner-provisioning audit. Old receipts were retained under their exact
source SHA and were never reused for a later commit.

The six-scenario local orchestration suite also passed at PR #79 head
`3bdaa215ce09c3ebd9b370667302103f926f7cda`. Its non-promotable receipt has
SHA-256 `4f71fe2623bc469860b4bc672dc3967a1072a0fcd9bf95e7db39cd7a8872998d`.
It covered the receipt state machine, systemd lifecycle, fault injection,
offline replay, static analysis, and cleanup/recovery. As designed, it made no
bare-metal IRQ or performance claim.

## Admission attempts

### Campaign A — scheduler tunable discovery

- Campaign: `hc0671-em-b220e-20260808-a`
- Exact source: `056b7202051b16cfc634e4ecd4470799f0fbeba0`
- Result: `host-admission-failed` before IRQ burn-in and before any performance
  dispatch.
- Cause: Ubuntu's generic kernel did not expose
  `kernel.sched_migration_cost_ns` through `/proc/sys`, although the mandatory
  value existed at the `CONFIG_SCHED_DEBUG` debugfs interface.

PR #78 fixed the producer invariant without making the tunable optional. The
collector now requires either the normal sysctl or the exact root-owned
`/sys/kernel/debug/sched/migration_cost_ns` file on a real debugfs mount with
`CONFIG_SCHED_DEBUG=y`. Backend, canonical locator, and value are frozen and
hashed, so backend drift still fails closed.

### Campaign B — Ubuntu ping invocation

- Campaign: `hc0671-em-b220e-20260808-b`
- Exact source: `d23dd5a97d4fa2031bc3a2343451777f727faa71`
- Result: `host-admission-failed` before any performance dispatch.
- Passed first: trusted source, provisioning audit, CPU isolation, host freeze,
  debugfs scheduler tunable, frozen-state check, absolute IRQ preflight, and
  IRQ-delta baseline.
- Cause: Ubuntu 24.04 `iputils ping` rejected GNU-style long options such as
  `--numeric`; the network stimulus therefore did not run.

PR #79 replaced only the spelling with the equivalent supported short options:
`ping -4 -n -q -c 32 -i 0.02 -w 10`. A regression forbids the unsupported long
forms. The exact replacement command was also smoke-tested successfully from
each measurement CPU on the rented host. Packet count, interval, deadline,
numeric mode, quiet mode, affinity, and all IRQ gates remained unchanged.

After the rejected diagnostic stimulus, offline provisioning correctly found
that IRQ 111 (`nvme1q2`) had acquired effective affinity on CPU 1 and recorded
2048 interrupts. No affinity was rewritten. A stabilization reboot restored
the reviewed initial layout before another audit.

### Campaign C — strengthened NVMe IRQ burn-in

- Campaign: `hc0671-em-b220e-20260808-c`
- Exact source: `e01d2cc48eb0c885b3d31b9fa38fbbb0a64c5d56`
- Result: `host-admission-failed` before any performance dispatch.
- Passed first: exact-main procurement and provisioning receipts, explicit
  reboot proof, early IRQ layout, CPU isolation, service policy, host freeze,
  canonical debugfs scheduler tunable, frozen-state validation, absolute IRQ
  preflight, NVMe read stimulus, and all four network ping processes with 0%
  packet loss.
- Rejection: the immediate post-stimulus delta guard observed 2048 new
  interrupts for IRQ 113 (`nvme1q3`) on a measurement CPU, from a zero
  baseline.

The full 900-second idle window was intentionally not entered after the
immediate zero-delta contract had already failed. Qualification was not
dispatched. Consequently there are zero accepted qualification runs, zero
full-dress admissions, and zero bootstrap samples from this rental.

## Interpretation

The allocation can boot with the reviewed CPU isolation and can present a
quiet initial IRQ layout, but an NVMe read issued from the measurement CPUs
activates an NVMe completion queue on those CPUs. This violates the unchanged
housekeeping-only, zero-new-IRQ admission contract. Rebooting can restore the
initial dormant layout, but it does not make the allocation stable under the
strengthened stimulus. Repeating qualification attempts on this same machine
would therefore have no evidentiary value.

The two software defects found during the rental were fixed and merged, but
the final failure is a host/allocation result, not an identity, parsing, or
command-line bug. The next attempt must use a newly reviewed hardware/profile
combination whose managed NVMe queues remain outside the measurement CPU set
under actual read stimulus. The existing `1-4` contract must not be silently
changed on the server; a different topology requires a new versioned profile,
tests, and fresh qualification family.

## Retained diagnostics and shutdown state

The original campaign and host-state trees for A, B, and C were archived with
numeric ownership metadata. The unchanged local archive is:

`hydracache-0.67.1-bootstrap-samples/rejected-campaigns/hydracache-0671-em-b220e-20260808-rejected-campaigns.tar.gz`

Archive SHA-256:

`f87c36b4e8fda7e529ed685fb04cf698fa9f1a4a7d73176cb52ce92f9ccdcad5`

The archive remains outside Git because it contains raw host diagnostics and a
provider gateway address. Its sidecar digest is retained next to the local
original. The Markdown report contains no public address or credential.

Campaign C was closed and produced `SAFE_TO_DELETE_SERVER=true`. Before the
provider handoff:

- no queued or in-progress CI run targeted the reference sequence;
- the runner was `offline` and `busy=false`;
- runner registration ID 22 was deleted from GitHub;
- the runner service was disabled and stopped;
- rootful and rootless Docker were stopped;
- the verified local archive digest matched the server-side digest.

The provider resource must be deleted/released, not merely powered off, to stop
bare-metal billing.
