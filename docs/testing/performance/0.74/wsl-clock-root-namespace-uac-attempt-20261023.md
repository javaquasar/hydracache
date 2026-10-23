# Root metadata retained from a timed out UAC attempt

The single [approved UAC attempt](wsl-clock-root-namespace-uac-plan-20261023.md)
at `d6446f53919d0dfed54f7dc760741c12221e03c5` passed Administrator preflight
and reached the root debug shell. The original 30-second client limit expired,
so the collector retained its synthetic timeout code 124 and Python exited 1.
The redirected stdout file nevertheless contains 18,864 bytes, including both
standalone metadata markers and all six fixed process records. Useful metadata
does not turn this timed out exchange into a passing collector run.

## First result and transport boundary

One standard UAC request returned elevated PowerShell PID `35288`, exit 1.
Its launcher recorded Python PID `46504`, exit 1, and the exact failure
`First root lookup exit 124; preserve all files, no retry`. The outer request
controller exited 0 because its request/wait operation completed; that is not
the payload exit code. No second request, collector invocation or fallback ran.

The debug stdout is terminal-shaped, with prompts, echoed commands, CR line
endings, escape sequences and NUL bytes. Its stderr is empty. The offline
decoder keeps those raw bytes intact, normalizes line endings only in memory,
requires unique standalone markers and the exact ordered PID inventory, and
checks every field result plus matching before/after PID, comm and start ticks.
Changing CPU counters does not invalidate that identity tuple. It finds the end
marker and a logout message; neither supplies a successful Windows client exit
receipt or independent proof that every guest-side process retired.

The first offline audit incorrectly assumed stdout was empty because the
TimeoutExpired exception had no captured output attributes. The call redirected
stdout to a file, not a PIPE: those exception attributes do not describe the
file's contents. The failed assertion and exact first audit source are retained.
The corrected offline audit reads the original file; it does not repeat the live
command or change the collector's timeout/marker checks.

Both 15-second inventory calls exited 0 and decoded to exactly `Ubuntu` and
`docker-desktop`. The collector's frozen-source postflight passed. Inventory
equality proves only equal distro names, not guest-process cleanup, clock
stability or absence of unrelated activity.

## Current root process identities

The observed boot is `cf222c07-5476-4f70-9a32-455e99204d0a`, observer PID
namespace `4026531836` and time namespace `4026531834`. The current records are:

| Root PID or TID | Executable | Start ticks | TGID | NSpid chain |
| --- | --- | ---: | ---: | --- |
| 1 | `/init` with comm `mini_init` | 6 | 1 | 1 |
| 210 | `/usr/sbin/chronyd` | 47 | 210 | 210 |
| 1157 | `/usr/lib/systemd/systemd-timesyncd` | 11340 | 1157 | 1157 → 193 → 176 |
| 1126 | `/init` | 11319 | 912 | 1126 → 180 → 135 → 94 |
| 1112 | `/init` | 11312 | 912 | 1112 → 171 → 126 → 85 |
| 920 | `/init` | 11153 | 912 | 920 → 112 → 67 → 26 |

Chronyd has UID 999, executable SHA-256
`bf39d010dfc6177cd864d96ddd0cb32eb796704e8c68fe8bd65088d20d2e64b8`,
cgroup `0::/`, and the root observer's PID/time/mount namespaces. This identifies
the currently visible root task rather than the unrelated Ubuntu PID 210.
The root cgroup does not identify a systemd service to stop.

Timesyncd has UID 996, executable SHA-256
`5e87427803daf1dbe974cc62fba335c96ff24e37892f67d9095d6703d2c32009`,
cgroup `/system.slice/systemd-timesyncd.service`, PID namespace `4026532241`
and mount namespace `4026532373`. Its identity fields and suffixes of the NSpid
chain match both [previously saved current views](wsl-clock-writer-identity-20261023.md).
These are sequential observations, not a held-pidfd lifetime proof.

The three init records are threads of TGID 912, not three independent process
leaders. They share executable SHA-256
`ed145ae8e128aeff4141dad30deef9222567cad9f45ecad7e128f05cfe64addc`,
cgroup `0::/`, PID namespace `4026532309`, mount namespace `4026532308` and
NStgid `912 104 59 18`. All selected records share time namespace `4026531834`.
The earlier trace observed setter calls at the same numeric kernel IDs, but did
not capture their contemporaneous start ticks/executable identities. This later
lookup therefore does not retroactively authenticate those historical writers
or establish their launch/configuration authority.

## Exact evidence and controls

Two separate archives retain preparation and the actual outcome:

| Archive | Logical files | Objects | Index SHA-256 |
| --- | ---: | ---: | --- |
| [Offline preparation](local-runs/wsl-clock-root-namespace-uac-controls-20261023/index.json) | 65 | 54 | `2c05470a638d890cdd2930247186143f041a0cded57c8cab6b89bb334316b115` |
| [Single UAC attempt](local-runs/wsl-clock-root-namespace-uac-attempt-20261023/index.json) | 103 | 79 | `792b54905fe6a1ebdeddc7f9eaf7963c2bb3b93c488610fe6737eb41f9e2a0d0` |

Preparation retains the first short/long TEMP-path fixture failure, its exact
source, the test-only filesystem-identity correction and 36 passing offline
controls. The attempt archive retains both request and child journals, exact
stdin, source pins, the earlier identity input, the first offline audit failure,
corrected analysis and eight passing byte-only outcome controls. The latter
reject missing/duplicate/echo-only markers, failed fields, PID/boot mismatch,
changed prior evidence and relabelling the timeout as success. No fake launcher
or live collector was rerun after consent.

Doc-check, the local non-promotable performance contract, the two earlier
archive verifiers and the frozen runtime/test/Cargo.lock/COMPAT diff guard pass.
The qualification manifest remains SHA-256
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
The original collector, logging core and both launcher versions remain
unchanged. So do all live predicates, the 300/60/10-second admission windows,
15-second inventory and 30-second debug-client limits.

The [first operator outcome](root-namespace-first-unconfirmed-20261023.json)
remains unknown with SHA-256
`70a405fcf3a65394b83d3e6a500b3a4ad454ce649436c80af2a098e25ba3006c`.
The [separate known Administrator refusal](wsl-clock-root-namespace-logged-attempt-20261023.md)
remains pinned independently. This timeout explains neither earlier outcome.

## Next decision

The UAC request and child output lanes are consumed. Do not clear, reuse or
redispatch them. The next safe work is offline attribution of the retained
identities and transport protocol, followed by an explicitly scoped decision
about any further diagnostic. Guest-side retirement remains unconfirmed; do
not infer it from client timeout, logout text or distro inventory.

No clock/service change, new tracing, stopped-distro start, original-owner run,
product workload, release installation or server operation was performed by this
attempt. Existing background clock writers were not controlled. Root metadata
narrows the diagnosis; it does not admit clock policy, original delivery,
full SIGKILL/lost-ACK testing or server execution.
