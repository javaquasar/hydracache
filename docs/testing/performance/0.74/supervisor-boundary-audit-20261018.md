# Server execution and monitoring ownership boundaries

The server preparation uses systemd as the process manager, not a second custom
scheduler. HydraCache's supervisor admits a fixed experiment and verifies its
identity, useful progress and retained outcome. An SSH or GitHub observer may
disappear without owning or restarting the measured process. Reconnection means
observing the same surviving process, never restoring execution from checkpoints.

This audit starts at source `9c12881b379619b05930e422cb215fae269ab4dc` and follows
the [P0 through P5 plan](diagnostic-server-readiness-intermediate-plan-20261012.md).
It changes no admission flag, trust selection, validation predicate or deadline.

## Responsibilities already delegated to systemd

| Responsibility | Existing integration | Boundary retained in HydraCache |
| --- | --- | --- |
| Detached process lifetime | Fixed transient services in `systemd_unit.rs` and typed diagnostic properties in `diagnostic_unit.rs` | Authenticate the original request and manager; bind actual process, executable, streams and cgroup |
| No workload restart | Both unit builders select `Restart=no` | A lost start acknowledgement must not issue another start |
| Resource containment and stop | Unit builders select `KillMode=control-group`, fixed limits and runtime bounds | Verify the exact original cgroup and cessation of every output writer, including descriptors outside it |
| Supervisor availability | The service file selects `Restart=on-failure` | Reconcile durable history without replaying a workload or manufacturing an acknowledgement |
| Status and logs | systemd unit inspection and journals | Authenticate observations and verify useful progress and checkpoint continuity |

The supervisor service's `KillMode=process` and the measured unit's
`KillMode=control-group` have different owners. Do not mechanically substitute
one for the other: active collector/custodian helpers still require explicit
supervision and a tested loss policy. No unit property is changed by this audit.

The [systemd-run manual](https://www.freedesktop.org/software/systemd/man/latest/systemd-run.html)
describes detached services owned by the manager. The
[service manual](https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html)
defines runtime bounds and notification watchdogs; the
[kill manual](https://www.freedesktop.org/software/systemd/man/latest/systemd.kill.html)
defines cgroup stopping. Notification liveness alone does not establish useful
benchmark progress, and cgroup stopping does not prove EOF if a writer alias
remains elsewhere.

## Custom responsibilities that remain necessary

Keep independently selected trust and epoch pins, kernel peer/context checks,
exclusive attempt admission, durable intent before side effects, one-shot start,
original-object validation, phase-specific progress and fixed lease enforcement.
Keep root-owned output collection, retained uncertain/failure outcomes and
independent offline evidence verification. systemd job receipt alone is neither
process-start proof nor output-completion proof.

`server.rs::dispatch_attach` already verifies campaign manifest, host receipt,
unit, process identities, cpusets and checkpoint evidence. The read-only
`monitor-long-run-campaign-074.py` validates a supplied state snapshot and writes
an observation receipt; it does not fetch live state or authorize attach. Neither
component proves an integrated reconnect on the new diagnostic execution path.

Do not introduce a scheduler, Prometheus/Grafana stack or a general remote shell
into the critical path. A replaceable metrics/notification adapter may follow,
subject to the frozen monitoring-overhead budget; it is not execution authority.
Do not remove live checks merely because systemd exposes related properties.

## Implementation sequence and acceptance boundaries

1. Correct the old private exchange's METHOD_RETURN interoperability, then join
   the owned manager channel to original admission/full validation and construct
   the uniquely addressed original message. Prove negotiated FD delivery and
   actual executable/environment/stream/process/cgroup identity before start
   admission. The old well-known-addressed message remains unsent.
2. Join autonomous active watchdog/collector supervision and all-writer cessation.
   Retain custody on timeout, unknown stop, leaked aliases or cleanup failure.
   Successful job receipt or leader death cannot release the fence.
3. Publish durable authenticated completion only after cessation, then implement
   fixed cells two through four and their legal transitions. Test every added
   publication cut with real SIGKILL and lost acknowledgements; never replay an
   interrupted attempt.
4. Exercise observer/controller loss and fresh live-status/attach on that exact
   path. Require unchanged original PID/start ticks, boot/cgroup and advancing
   checkpoint chain with no replacement execution. Test stale/corrupt status,
   concurrent attach and loss after terminal completion as separate cases.
5. Enroll independently reviewed production policy/digest/epoch, receiver and
   protected workflow pins on the actual host. Existing keys or fixture success
   cannot select these values. The October 15 read-only host inventory is a dated
   observation, not current enrollment authority.
6. Run the integrated local fault gate and full milestone verification. Freeze a
   signed exact bundle/receipt, perform read-only ownership/UID/GID/cgroup/space
   preflight, then admit a bounded nonproduct server rehearsal separately.
   Installation, supervisor maintenance, reboot and product qualification need
   their respective explicit approvals.

Original Reserved 300 s, Starting 60 s, progress/challenge 10 s windows and their
origins remain unchanged. Controller disconnection and expiry of a measured
process's execution authority are different faults. Passing this sequence does
not admit a product candidate or the six-hour/24-hour qualification stages.

## Preregistered exchange interoperability correction

The actual fixed-bus packet at
[manager-channel-20261018](local-runs/manager-channel-20261018/README.md) attributed
the stopped first producer to METHOD_RETURN flags=1. The
[D-Bus specification](https://dbus.freedesktop.org/doc/dbus-specification.html#message-protocol-header-encoding)
gives NO_REPLY_EXPECTED no reply semantics. The old dispatch exchange still
rejects this form; its fourteen existing units provide the baseline.

Allow only flags 0 or 0x01 on an already required METHOD_RETURN. Keep request
flags exactly zero and ERROR handling unchanged. Exhaust all eight combinations
of known bits; only those two return forms may yield the same job-path data.
Cross permitted forms with wrong correlation/body/FDs and closed/duplicate/early
reply windows. Preserve the original refusal-before-parse and Uncertain-before-IO
ordering, every name/serial/size/FD predicate and no-send boundary. Register exact
source restoration for this correction only, not arbitrary normalization.

Record the unchanged baseline, a failing new positive test, the correction,
current/MSRV unit cohorts, source canaries, checks/strict Clippy and portable
controls. This correction alone cannot transmit a message or establish execution,
cessation, completion, trust enrollment or server admission.

## Exchange correction result

Source `2076b5882a5ea47da4e426c7b3964f0a4c3f8b2b` passes nineteen exchange
tests on current/MSRV, affected checks/strict Clippy, 235 source contracts and
portable controls. The [retained packet](local-runs/exchange-reply-flags-20261018/README.md)
keeps the original fourteen-test baseline, new positive test's expected failure
and first exact-format source-comparison failure separate from the accepted
gate. Only METHOD_RETURN flag interoperability is closed; the consuming original
channel/message join is the next code boundary, not a newly admitted start.
