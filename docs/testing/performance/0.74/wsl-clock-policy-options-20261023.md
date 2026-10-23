# Clock policy options before the remaining fault tests

Recommendation: keep the current WSL clocks and services untouched, and plan a
separate Linux kernel for the remaining functional lifecycle/fault tests. A
local Linux VM is the preferred local-only option, subject to a read-only
capacity/capability check and separate approval to create it. A separately
approved native Linux server fixture is the alternative if local resources are
unsuitable. Neither environment is provisioned or admitted by this decision
document. No clock setting, service operation or new live run is authorized.

This is a proposed test-environment policy, not production policy, signed
enrollment or a change to the qualification manifest. Moving the tests does not
implement the remaining original-delivery, writer-retirement or lifecycle work.

## Facts that constrain the decision

The [first read-only series](wsl-clock-readonly-20261023.md) recorded backward
REALTIME through direct syscalls while BOOTTIME continued forward. The
[discipline analysis](wsl-clock-causality-20261023.md) explained a large
MONOTONIC/RAW rate difference using saved kernel tick/frequency state. These are
different observations, not proof of one historical writer or a product defect.

The [bounded trace](wsl-clock-setter-20261023.md) then observed successful
tick/frequency requests under kernel task name chronyd, absolute settime calls
under init thread IDs and offset injections under systemd-timesyn. Nested
instrumented calls are not independent physical corrections. Its all-increasing
sampled REALTIME series did not exclude opposing corrections between samples.

The later [root metadata](wsl-clock-root-namespace-uac-attempt-20261023.md)
identifies current chronyd PID 210 and timesyncd PID 1157, with NSpid
`1157 193 176`. The init IDs are threads of TGID 912. They share the observed
time namespace `4026531834`. These later executable identities do not
retroactively authenticate the earlier trace. Chronyd's root cgroup and the
init executable do not establish a safe service/configuration control to use.

Microsoft describes WSL2 distributions as containers in its managed VM, not
independent Linux VMs. That architecture and the recorded shared time context
are why masking a service only inside HydraCache074Native did not establish
clock isolation. [WSL architecture](https://learn.microsoft.com/en-us/windows/wsl/compare-versions)

The [Linux time namespace interface](https://man7.org/linux/man-pages/man7/time_namespaces.7.html)
does not virtualize CLOCK_REALTIME. Merely adding a container or a time
namespace is therefore not the required wall-clock boundary.

MONOTONIC is subject to incremental discipline, whereas RAW excludes those NTP
adjustments. BOOTTIME includes suspend time. A difference between these clocks
is not by itself a UTC calibration. [Linux clock semantics](https://man7.org/linux/man-pages/man3/clock_gettime.3.html)
The [Linux adjtimex interface](https://man7.org/linux/man-pages/man2/adjtimex.2.html)
manages kernel tick/frequency parameters, not just daemon-local state;
stopping a writer must not be assumed to restore a safe rate.

## Options and affected components

| Option | Components affected if separately approved | Disposition |
| --- | --- | --- |
| A Keep current WSL unchanged | None; saved-byte analysis and offline/mock development only | Safe now, but no claim that the remaining live gate can pass. Do not keep retrying it. |
| B Pause only Ubuntu timesyncd | Existing Ubuntu service; the shared clock remains exposed to other writers and retained discipline | Not recommended as a sufficient fix. The previous service-stop permission is not reused for this new experiment. |
| C Coordinate the whole WSL clock domain | All relevant distro writers, root chronyd/init authority, host integration and kernel state; potentially other workloads | Deferred. Exact owners, supported controls, impact and restoration are not established. No shutdown, restart, kernel parameter, clocksource, tick/frequency or Windows-time change is approved. |
| D Separate local Linux VM | New local image/storage/CPU/RAM/network, its own kernel/time policy and temporary diagnostic fixtures | Recommended for functional fault tests if capacity and capability permit. A different WSL distro or container does not satisfy this option. |
| E Native Linux server fixture | Explicitly selected host, read-only preflight, then separately approved temporary directories/cgroup/namespace and fixtures | Alternative to D, not authorized now. No release installation, supervisor operation, existing-service change, clock mutation or product workload is implied. |

Preference for D is an isolation/risk decision, not a measured claim that its
clocks will be healthy. A VM's own hypervisor clock integration and time service
must still be inventoried and admitted. Availability, disk capacity and resource
cost have not been checked in this step. Do not install a hypervisor, enable a
Windows feature or download/create an image as an automatic extension.

## Proposed clock policy for a selected independent test kernel

1. Name the exact host/image, kernel, boot, clocksource, time namespaces and all
   known mechanisms permitted to adjust time. Identify the operator/configuration
   authority and restoration procedure. A daemon name alone is not authority.
2. Agree one coordinated discipline policy, including any host/hypervisor
   integration. Any intentional initial synchronization belongs before issuing
   admission origins. Do not disable corrections indefinitely and assume UTC
   correctness, manually reset tick/frequency to guessed values, or silently
   replace the existing clock source in the test code.
3. During an admitted attempt, perform no administrative clock/service changes,
   suspend/resume, migration or snapshot restore. A reboot, backward read,
   expired window or context mismatch retains its first refusal. Do not refresh
   issued/now, restart the attempt or stitch a new lifetime onto old evidence.
4. Preserve the unchanged kernel reads and live validation frame. In particular,
   the runtime samples BOOTTIME and REALTIME between origin revalidations and
   rejects backward order or changed boot. RAW remains diagnostic context, not
   an alternative authority/lease clock.
5. Keep all original 300/60/10-second windows and their exclusive boundaries.
   Preserve every other source, credential, namespace, owner, manager, executable,
   FD, cgroup, enrollment and writer-retirement predicate. No new tolerance or
   performance threshold is introduced by this proposal.

The existing `diagnostic_kernel_origin.rs::sample` and
`diagnostic_kernel_admission.rs::{ordered,valid_hold}` define the live behavior;
this policy does not replace their checks with a preflight assertion. A finite
clean observation is not a guarantee that an unobserved correction cannot occur.
Any correction that invalidates an active attempt must still fail closed.

## Staged admission and separate permissions

The next bounded scope to request is a read-only local capability/capacity
inventory for D, with no VM creation, feature activation, WSL invocation or service
change. If D is unavailable or undesirable, choose E explicitly instead. Do not
switch hosts silently after a refusal.

Only after selecting the environment:

1. Approve exact provisioning resources and temporary fixture ownership. Freeze
   the image/source/toolchain and test method before any new observation. Do not
   reuse WSL-specific boot or namespace pins on another kernel.
2. Review the environment's real time/enrollment authorities. Prepare a bounded
   read-only clock/context control with exact limits, source/build receipts,
   exclusive journals and a first-outcome rule. No control is run by this plan;
   approval of it does not also approve original-owner or the full fault matrix.
3. If that control passes, approve a distinct original-owner/fault scope. Bind
   fresh real context through the existing enrollment path; do not forge pins or
   recycle an incomplete attempt. Use a process-tracked control channel with
   proven guest retirement, not the consumed debug console.
4. Establish one healthy original-delivery baseline, then execute only the
   preregistered SIGKILL/lost-ACK cells for that admitted candidate. Monitor the
   same process lifetime and retain every first failure. No retry-to-pass or
   larger deadline is permitted.
5. Retain fault results and verify all writers stopped before fixture cleanup
   and fence release. Production policy, server installation, supervisor
   maintenance and any product/qualification workload remain separate gates.

Local-VM functional results do not qualify bare-metal saturation, latency or
long-run retention. They do not substitute for the exact-candidate server
rehearsal and release qualification. Likewise, selecting a stable test kernel
does not settle the production clock policy or close implementation gaps.

## Evidence and status

The [offline review record](wsl-clock-policy-review-20261023.json) pins the prior
archives and unchanged runtime/manifest inputs used for this decision. All
options remain proposed; no execution option is selected in machine admission
state. This step adds no runtime code and no numerical performance measurement.
The first unknown operator outcome, Administrator refusal and UAC timeout stay
independent and immutable. The debug console is not rerun.

This documentation-only step passes doc-check, the local non-promotable
performance-contract check, the saved identity/capture verifiers and the
byte-only transport assessment. All nine reviewed archive indexes and every
referenced object's size/hash verify. The frozen runtime/tests/Cargo.lock/COMPAT
guard passes. No Rust package or test implementation changed; no live or
measurement test was repeated.
