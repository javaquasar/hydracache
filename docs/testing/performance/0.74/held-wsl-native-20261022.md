# Separate WSL environment and first actual native FD observation

The authorized separate local environment `HydraCache074Native` now runs
Debian 13.5 with systemd 257.13-1~deb13u1. A synthetic nonproduct helper receives
the real native extra-FDs and reports correct mapping. **The complete mechanics
control still fails** on its first wrong-interface property query. That outcome
is retained; the query is corrected offline without another delivery. Original
delivery, all-writer cessation and server admission remain false.

## Environment preparation and isolation

The [plan](held-wsl-native-plan-20261022.md) fixes a new name and dedicated
storage directory `C:/Workspace/prj/jq/cashe/hydracache-074-wsl-native`. It is
not a Git worktree. The Microsoft-pinned Debian WSL image is downloaded once:
89,228,801 bytes, SHA-256
`5ec7dc68216e75d1d4d4761474e99d8461a98d316537110314b137122a879e0f`.
The archive's actual package database, not a current package-site version,
establishes the installed systemd version; dbus is 1.16.2-2.

The first offline derivation refuses absent python3-minimal before any import.
Its source, receipt and derived-tar digest remain. The minimal rootfs does not
need Python to boot systemd; [revision 2](held-wsl-native-plan-v2-20261022.md)
separates that environment prerequisite from future Python harness dependencies.
No package is installed or upgraded. The new prepared image has 10,589 copied
members, 238,008,320 bytes and SHA-256
`253025e7599908f372199e8515ba0a1574b7dc25b0b1104f16ba5587a926a2d9`.

Before first boot, offline derivation enables systemd only in distro-local
wsl.conf, removes vendor OOBE and masks nine time-agent/timedated units and
six maintenance timers. Other archive contents/metadata are preserved. Both
the original and prepared archives remain local, not committed. The VHDX is
environment storage, not a release artifact. No extraction onto Windows occurs.

The one-shot import and first boot succeed. Registry snapshots before/after
show identical existing Ubuntu/docker-desktop entries, unchanged default and
unchanged global .wslconfig digest/absence. No global WSL shutdown/update or
existing-service operation is issued. First boot verifies the masks, PID1 and
bus-reported manager PID 1/UID 0. Three read-only realtime/uptime/boot samples
remain in raw logs. They are observations, not a complete runtime clock gate.

## One actual native extra-FD delivery, not an original launch

The [synthetic control](held-native-fd-plan-20261022.md) is a fixed oneshot
shell helper, not HydraCache. It uses one random service and corresponding
private slice with restart=no, NoNewPrivileges, control-group kill and bounded
start/stop. Every busctl argv/output/exit is retained. Methods target a unique
manager owner with reported PID 1/UID 0, but use separate connections: this is
explicitly not the retained original manager channel/full validation frame.

Source 4befa365 first refuses the `--helper` payload as a busctl CLI option,
before any message delivery. Its uncertain cleanup refuses to stop an unmarked
unit. The first receipts remain. A failing offline regression precedes the
[option-terminator correction](held-native-fd-plan-v2-20261022.md), source
dde2cbde, with a new random name and separate output directory.

The corrected lane sends exactly once and receives a job-path ACK. The helper
reports LISTEN_PID 242, LISTEN_FDS 2 and LISTEN_FDNAMES role:empty. Read-only
/usr/bin/true and /dev/null arrive at slots 3/4 with matching device/inode/type;
the role hash check passes. Its reported stdout/stderr identities match the
parent's separately opened report FDs. It reports /usr/bin/bash and the exact
random service cgroup under its private system-hc074nativefd...slice. The manager
reports exited, Result success, ExecMainStatus 0 and MainPID 0; helper stderr
is empty. These observations establish only this synthetic descriptor mapping.

The lane then asks ControlGroup on org.freedesktop.systemd1.Unit and fails:
Unknown interface/property. The [v257 interface reference](https://github.com/systemd/systemd/blob/v257/man/org.freedesktop.systemd1.xml)
places ControlGroup on Service in this path, not the generic Unit interface. A second offline red
regression precedes the corrected Service query in f226d362. **No second actual
delivery is performed.** The initial outcome remains exit 1, not a promoted
green mechanics gate. Offline replay validates the helper/stream observations
from the retained data without inventing the missing manager property result.

Cleanup verifies the same unique manager and exact random Description, sends
one StopUnit only for that service and observes inactive. It does not stop an
existing service, supervisor, another attempt or a wildcard. First boot and
the two controls have distinct recorded WSL boot lifetimes; their observations
are never concatenated into a continuous original attempt or recovery proof.

## Checks, evidence and remaining boundary

Eighteen final offline controls cover archive preservation/masks/refusals,
one-shot host orchestration, identity/default/config preservation, exact nonce
and command, first outcome retention, both CLI/property regressions and receipt
schema refusal. Bash syntax, formatting, doc-check, gated-test registry, local
non-promotable performance-contract and diff checks are separate focused gates.
No Rust source changes or new Rust compilation/performance cohort are claimed.

The [immutable packet](local-runs/held-wsl-native-20261022/inventory.json)
retains environment receipts, original failures, both synthetic controls,
failing regression baselines, source versions and final check logs. Large
rootfs archives/VHDX are omitted from Git; their digests remain. Sources are
8b2ceb25 (preparation), 4befa365 (control), dde2cbde (CLI correction) and
f226d362 (offline property correction). Raw packet bytes are audited separately
from the Git checkout's text conversion.

The packet seals 205 members plus inventory.json; inventory SHA-256 is
`def7bee3105292d240f0b542dd4ab35a0a4a8da65233c5e0c9c6423e80785597`.
All 206 files match their staged raw Git blobs. Collector/check tooling is
6c30567f; immutable evidence is 1bfd42af. No sealed packet member is rewritten
to include this later prose or these commit references.

Next integrate the held descriptor mapping into the consumed owner and retained
manager channel/full frame, with durable one-shot uncertainty and actual held
executable/stream/process/cgroup binding. The active independent watchdog must
then cover all writers and external FD aliases, and completion must wait for
revocation, empty pinned root, kernel EOF/drain, collector reconciliation and
durable terminal proof under the fence. The synthetic helper cannot supply any
of those predicates. Full SIGKILL/lost-ACK, cells 2–4, reconnect, production
enrollment and server integration remain independent unfinished gates.

All original live checks, predicate order/count, 300/60/10 origins/windows,
first Causal failures and qualification remain unchanged. Qualification SHA-256
is `11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
There is no server operation, product load, clock change or release installation.
