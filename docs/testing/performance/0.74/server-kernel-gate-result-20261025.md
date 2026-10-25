# Dedicated Linux kernel controls and preparation refusal

The authorized bounded server session passes three existing Rust kernel tests
containing 31 internal scenarios and survives loss of its SSH observer. The
fresh current-source full holder matrix nevertheless fails its first healthy
case at the original ten-second causal window. Server execution is now observed,
but original delivery, production recovery and release admission remain closed.

## Exact build and host

The source capsule contains 2558 raw-file hashes from committed source
`ee9692b672be093716822db13a22f5a63960f928`. Its gzip archive is 7082247 bytes,
SHA-256 `45ed5f3f2b8f465cad58a61bca21169bcb5e9df5e66b9bf1081a6793a179e63c`;
source-manifest SHA-256 is
`2a0f7c581a1e854ab71841efb71aaebee3de56d1c0357b54d55a41ae4011ea6d`.
The capsule preserves the working tree's raw source/fixture bytes, including
the independently frozen historical material lock. It is not a release bundle.

The dedicated host reports Linux `6.8.0-138-generic`, cgroup2 and systemd
`255.4-1ubuntu8.17`. Existing Rust 1.94.0 compiler/Cargo bytes are copied into
the private build directory, not installed or updated. Cargo runs as UID/GID
1000, offline/locked, default features, release profile, two jobs, incremental
disabled and nice 10. Only the supervisor library test image is built; source
hashes match before and after. Its exact SHA-256 is
`6640e1b9b31ae7c87137b335be4f2665d729ea7b4324ee0e874bdbedc346c520`,
26376320 bytes. The image is copied into protected root-owned storage before
privileged component execution. No product daemon is executed.

Two setup refusals are retained separately: the first compiler child could not
traverse the existing runner's private home; a second setup script rejected an
incorrect expected replacement count before invoking the compiler. Neither
started Cargo or a test. The third, explicit private-toolchain configuration
builds successfully without changing existing home/toolchain permissions or
shared caches. These setup corrections do not erase a failed test or constitute
a repeated holder attempt.

## Kernel and disconnect controls

Each of the following exact ignored tests executes once, serially, with one
test thread. The existing fixture timers and assertions are unchanged; external
180-second case bounds do not extend any original admission window.

| Component | Internal scenarios | Observed outcome |
| --- | ---: | --- |
| `cgroup_root_kernel_fault_matrix` | 11 | Pass, including four actual creator SIGKILL/lost-confirmation cuts, recursive nonroot writers, replacement/permission refusals and external writer blocking EOF. |
| `cgroup_root_guardian_fault_matrix` | 16 | Pass, including construction failures, disconnect, guardian kill/stop, duplicate enrollment, late population and explicit double-guardian-loss negatives. |
| `root_guardian_original_kernel_controls` | 4 | Pass for the original-root kernel adapter; original validation frame and delivery remain false. |

The guardian test deliberately proves that its two-guardian mechanism does not
cover every failure: cases 10/11 retain `expected_negative=true all_writers=false`.
An external FD alias also blocks EOF until that alias closes. Passing these
negative assertions must not be presented as all-production-writer cessation.
The four creator cuts concern kernel revocation boundaries, not a completed
Starting/completion/cells 2–4 crash matrix.

The diagnostic wrapper publishes 27 fsynced, hash-linked checkpoints. After six
observations, the controller kills only its original local SSH process. A new
strict-known-host connection finds the same live wrapper PID `315925`, start
ticks `163514632`, boot identity and uninterrupted checkpoint prefix. The
guardian component's same launcher-child PID/start ticks (`315951`/`163514654`)
appear on both sides. This is the owned timeout launcher, not separate
attestation of the Rust test process's PID/cgroup. Observed sequence 6 advances
to sequence 9 on reattachment; the
final chain remains continuous through terminal sequence 26. No server worker
is signalled or restarted by the disconnect control.

This proves the detached diagnostic wrapper/child observation, not production
controller lease recovery, same-cgroup original delivery or a qualified measured
process lifetime. The fixture never gains production execution authority.

## First current holder attempt remains red

The same fresh image then executes the existing full five-case holder selector
once. Nine hash-pinned historical material files are copied read-only into its
protected staging directory. The historical worker image is validation input,
not an executed workload. The fixture privately projects only its own unique
cgroup subtree; host `system.slice` is not modified. There is no selected-case
or cost override, profiling, original send or retry.

The first `healthy` subprocess and parent both exit 101. Preparation refuses at
`tests/support/consuming_held_live.rs:245`, in `prepare_owned_root_guardians`,
with `Context(...Causal(Time))`. The causal record states `expired`:

- issued BOOTTIME: `1635367840503315` ns;
- observed BOOTTIME: `1635377859195265` ns;
- first refusal age: `10018691950` ns;
- first custody observation age: `8884293654` ns;
- last accepted custody observation age: `9719039055` ns.

All 118 recorded causal observations retain the same origin; original windows
remain 300/60/10 seconds. `kernel=null` means no kernel refusal was recorded,
not a continuous clock attestation. Zero cases complete. `revalidate`,
`role-before`, `role-after` and `trust-after` are not executed. The successful
kernel tests and postflight cannot supply their missing verdicts. Earlier WSL
and server refusals remain intact, and no comparative speedup follows from
different single-attempt frontier ages.

## Preserved host and immutable evidence

Before/after checks find all nine campaign/lifecycle markers absent, no owned
fixture cgroup remaining, unchanged boot
`e81671d6-bf23-4ebf-b8fe-5dad1f45866f`, and supervisor active/running with PID
8839/NRestarts 0. Root safety frames and PID1 mount-table hashes match. Installed
binary and receipt retain their original inodes, metadata and SHA-256:
`f797629f65385ebfd4d94577a43a2d1d020f71c1e841028dad7d02a51b47ad62` and
`b314953d4deedaeaade3e24468da4844b7f13d348a5d77b8e8f41c672ff44525`.
These are endpoint observations, not an exclusive host reservation.

Protected staging remains `/tmp/hc074-server-gate-root-20261025.0k0xbdah`;
private build/upload remains `/tmp/hc074-server-gate-upload-20261025.pvdj0f6c`.
The completed 67-file root export is byte-verified after SCP. No existing
service, account, enrollment, clock, release file or dependency changes.
Staging/build artifacts are retained, not broadly deleted. Executables and
the source tar are excluded from Git; their identities and manifests are kept.

The [evidence packet](local-runs/server-kernel-gate-20261025/index.json) has
235 members/136 content-addressed objects; index SHA-256 is
`5366ec91b06b7877e152853d3d7d10ed35e17f32441499ac0bcb4b159676b32f`.
It retains raw host/transport/build/test data, both setup refusals, the failed
holder prefix, exact coordinator sources and offline reconstruction. Six
falsification controls reject checkpoint breaks, replacement/dead reconnects,
host drift, altered windows and promotion of partial results.

The [supplemental checks](local-runs/server-kernel-gate-checks-20261025/index.json)
retain independent staged-byte verification, ten existing offline-reader tests,
doc-check and the local non-promotable performance contract. Together with the
six new falsification controls, 16 local Python checks pass. The supplemental
index has 18 members/15 objects, SHA-256
`0341a399609c1573c3b641b826bff81ed32b9475e6aab6cf0540a1449f56d7e3`.
No complete Rust package, strict lint milestone, repeated product measurement
or qualification pass is supplied by these checks.

## Next decisions

The immediate implementation blocker is complete preparation cost inside the
unchanged causal window. Native evidence rules out treating local observer
bookkeeping as a substitute for this gate; the next cost proposal must preserve
fresh predicates, their frequency and original origins. It requires its own
attribution and candidate, not another run of this failed image to seek a pass.

Separately, the installed manager is 255. The reviewed original FD-delivery
compatibility scope requires a compatible newer manager; this session neither
probes its setter with a unit mutation nor upgrades the host. A separate
compatible environment or manager maintenance is an explicit operator decision.
Production enrollment, authenticated original start, all-writer retirement,
durable completion/later cells, the full package and product qualification stay
open. The frozen qualification manifest is unchanged and non-admitting.
