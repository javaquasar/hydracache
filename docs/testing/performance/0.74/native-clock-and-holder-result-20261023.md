# Native clock control passes and the holder expires before join

The [bounded dedicated-server experiment](native-clock-and-holder-plan-20261023.md)
completed without changing clocks, services, installed release files or the
supervisor. Its 601-row read-only clock control found no sampled backward or
libc/syscall bracket anomaly. The subsequent GNU holder matrix nevertheless
failed its first healthy case at the original ten-second window. No case was
retried and the remaining four fault cases were not executed.

## Exact inputs and host boundary

The clock plan/runner were committed as `6ae3751c`, protected holder runner as
`048819cc` and offline outcome reader as `e95caf4a`. The existing GNU sampler
remains SHA-256 `e5c44ef4920caf685b0860d695ac25d78cf791e868621e5981dc4f30838ed71e`.
The existing libtest image remains
`ca5484aa951414b80aefd0148c7d8f5185a8fdf73bb197cf98a790de59b8cff1`;
all 191 inputs in its saved build proof matched before dispatch. No image was
rebuilt, and runtime/tests remain at `8fb0fb5a`.

Strict known-host, noninteractive SSH used the existing hydracache-ax42 alias
and UID/GID 1000. The host reports Linux 6.8.0-138-generic, systemd 255 and
cgroup2. Boot stayed `e81671d6-bf23-4ebf-b8fe-5dad1f45866f`. All nine fixed
campaign/diagnostic/lifecycle markers were absent at the checked points.
Supervisor remained active/running with PID 8839 and NRestarts 0; installed
binary and receipt SHA-256 stayed
`f797629f65385ebfd4d94577a43a2d1d020f71c1e841028dad7d02a51b47ad62` and
`b314953d4deedaeaade3e24468da4844b7f13d348a5d77b8e8f41c672ff44525`.
These point observations are not an exclusive host reservation.

## Phase A read only clock control

One unprivileged nice-10 execution retained all 601 rows with the unchanged
100 ms spacing and BOOTTIME/REALTIME/MONOTONIC/RAW libc/syscall brackets. Sample,
SSH and postflight exits were all zero. Boot/PID1/namespace/offset identity
matched before/after, as did input hashes. Every clock had zero observed
backward intervals in all three read positions and zero bracket violations.

Direct endpoint spans were 60,035,762,979 ns BOOTTIME and 60,036,938,444 ns RAW.
The adjacent REALTIME-minus-BOOTTIME residual range was -290 to +140 ns; reads
are not simultaneous. These are one finite control's observations, not UTC
accuracy, an absolute drift estimator or a performance improvement. No sampled
anomaly does not exclude intervening steps or later corrections. The existing
timesyncd remained active; no daemon was stopped or reconfigured.

The helper bound remained 80 seconds and SSH bound 120 seconds. Neither
replaces an original 300/60/10-second admission window. The unchanged offline
observer analysis reproduces the result from retained bytes.

## Phase B first healthy refusal

The new root-owned 0700 staging directory verified the runner, manifest, GNU
image and all nine material files after copying. The historical worker image
was material-validation input only, not an executed product. The clean
environment omitted case, cost, material and original-service overrides.
Only the exact ignored holder selector ran, with one thread and the unchanged
1200-second external safety bound.

The first healthy subprocess panicked at
`tests/support/consuming_held_live.rs:245`, in `prepare_owned_root_guardians`.
Its recorded first causal refusal was `expired` at
`diagnostic_collector_causal.rs:109`:

| Observation | Original BOOTTIME age |
| --- | ---: |
| First recorded causal check | 264,756,843 ns |
| First custody check | 9,027,489,393 ns |
| Last accepted custody check | 9,876,706,154 ns |
| First expiry | 10,015,008,749 ns |

The same issued origin is retained throughout. The refusal report has
`kernel=null`, `deadline_changed=false`, `guard_clock_added=false` and 116
causal observations. This records expiry, not a recorded kernel clock-order
refusal. It does not separately measure CPU, I/O or scheduler cost, and a clock
control preceding the test is not a continuous trace of its execution.

The child and parent both exited 101. Completed cases: none. Not executed:
revalidate, role-before, role-after and trust-after. Signed placement, consuming
join, all-writer assertions and the full SIGKILL/lost-ACK path were not reached.
The old fixture's inner `server=false` is a historical label; the outer
execution/retention receipts correctly state `server=true`.

## Postflight and retained artifacts

Root-wrapper postflight and an independent subsequent read-only SSH preflight
both passed. Input/installed hashes, supervisor and boot remained unchanged;
the host mount tables are byte-identical. The bounded checks found no remaining
owned-root/root-fixture directories or cgroup subtrees. These cleanup facts do
not promote the failed test into proof of all-production-writer retirement.

The following staging directories are retained, not deleted or reused:

- `/tmp/hc074-native-clock-20261023.Zq88Md`
- `/tmp/hc074-native-held-upload-20261023.dXIenK`
- `/tmp/hc074-native-held-root-20261023.0Au6cQ`

The [exact 122-file archive](local-runs/native-clock-holder-20261023/index.json)
has 74 objects and index SHA-256
`7856778ab511deab026ffe5576496d202b1a29a02ceb8271fbc1b169afdd70d9`.
It retains exact clock data/metadata, root-export bytes with individual hashes,
first failure and derived analyses, all SSH/staging receipts, source/build proof,
runner sources and local checks. Executable binaries are excluded; their hashes
and existing build proof remain pinned. Both analyses reproduce offline.

Ten existing observer controls, five new offline outcome controls and shell
syntax checks pass. The latter prevent a clean clock sample or successful
postflight from promoting a partial/failed matrix. Doc-check, the local
non-promotable performance contract and the frozen runtime/test/Cargo.lock/
COMPAT guard pass. The qualification manifest remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
Earlier WSL refusals, unknown outcome and UAC timeout remain unchanged.

## Next gate

Native Linux did not make this candidate healthy automatically. The first
custody observation already consumed most of the original window. The next
work is offline attribution of the existing preparation/validation graph and a
separately reviewed cost experiment, not another healthy retry or a larger
deadline. No CPU/I/O owner can yet be assigned from the age records alone.

The host's systemd 255 is also a separate future compatibility question for
the newer original-delivery/18-role FD path. That transport gate was not
exercised: the recorded refusal is expiry, not an unsupported-FD reply. No upgrade,
bootstrap/enrollment/account installation, original send, supervisor maintenance,
release installation, product candidate or expensive qualification ran.
