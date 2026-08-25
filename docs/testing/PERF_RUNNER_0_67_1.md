# Release 0.67.1 dedicated performance runner runbook

This runbook provisions one short-lived Scaleway Elastic Metal host for release `0.67.1`
qualification and reference bootstrap. It is written for the provisional
`EM-B220E-NVMe` host (AMD EPYC 7232P, 8C/16T, 64 GiB RAM, local NVMe) but the checks are
provider-neutral.

For the next rental, use this runbook together with the profile-driven
[next-rental automation playbook](PERF_RUNNER_NEXT_RENTAL_PLAYBOOK.md). The new
playbook captures the lessons from the completed qualification/exploratory work
and adds allowlisted service tuning, exact environment freeze, drift detection,
and reversible pre-state receipts.
The executable, resumable sequence is documented in the
[0.67.1 campaign automation guide](PERF_RUNNER_0_67_1_CAMPAIGN_AUTOMATION.md).
Before renting another host, repeat or review the
[local Docker orchestration preflight](PERF_RUNNER_0_67_1_LOCAL_DOCKER_PREFLIGHT.md).
It verifies the containerizable context, shell, cgroup, affinity, pinned-tool,
and fail-closed surfaces, while explicitly leaving physical IRQ and bare-metal
qualification to the rented machine.
Also run the
[six-scenario orchestration hardening suite](PERF_RUNNER_0_67_1_LOCAL_ORCHESTRATION_HARDENING.md).
It exercises the complete receipt state machine, disposable systemd lifecycle,
fault injection, offline replay, static analysis, and exact cleanup/recovery.
The commands are intentionally split into local Windows PowerShell, remote root, remote
administrator, and GitHub runner-user sections. Do not paste an entire section blindly. Run one
block at a time, inspect its output, and stop on any failed assertion.

Before creating a billed host, record the named spend/deletion owners with
`scripts/perf/reference-rental-readiness.py` as described in the
[next-rental playbook](PERF_RUNNER_NEXT_RENTAL_PLAYBOOK.md). After installing
the reviewed CPU/IRQ isolation and rebooting, run
`prepare-reference-host.sh irq-layout-preflight` before entering a runner
registration token or completing runner registration. These receipts reduce
wasted rental time but remain non-evidence and do not replace W2/W3.

Separate indicative reports are defined in
[`PERF_INDICATIVE_0_67_1.md`](PERF_INDICATIVE_0_67_1.md). Their numbers may be
used for optimization hypotheses only and never satisfy this runbook.

> **Evidence boundary**
>
> Provisioning and runner registration do not approve the machine. Until W2 machine attestation
> and W3 qualification mode from the
> [0.67.1 plan](../plans/V0_67_1_DEDICATED_PERFORMANCE_REFERENCE_BOOTSTRAP_PLAN.md) are implemented
> and green, do not dispatch the current `0.67 Self-hosted Reference Evidence` job. Exploratory
> output is not release evidence and must not be quoted as a capacity, sizing, baseline, or Redis
> comparison claim.

## Measurement I/O isolation contract

The runner service is root-owned and restricted to housekeeping CPUs `0,5-7`. Build, Git,
artifact, Docker-control, and other orchestration I/O must remain inside that inherited affinity;
only the already-prewarmed measurement child may run on CPUs `1-4`.

The contract is:

- `scripts/perf/reference-evidence-tmpfs.sh prepare` creates the commit-bound evidence tree at
  `/dev/shm/hydracache-reference-evidence-v1` and links the normal evidence paths to it;
- `scripts/perf/run-reference-measurement.sh` warms exact binaries, libraries, and inputs on
  housekeeping CPUs before applying `taskset --cpu-list 1-4` to the measurement child only;
- when the measurement child attests the host, its `git`, `findmnt`, `lsblk`, and
  `systemd-detect-virt` probes are launched through `taskset --cpu-list 0,5-7`; their executables,
  libraries, `/etc/os-release`, and the root-owned provisioning receipt are prefaulted before the
  pre-measurement IRQ guard;
- `scripts/perf/reference-runtime-irq-guard.sh` rejects any active IRQ reaching CPUs `1-4`
  immediately before or after a measurement;
- the RESP wrapper pulls and prewarms the pinned Redis image on housekeeping CPU `0`;
- `scripts/perf/reference-evidence-tmpfs.sh materialize` copies completed evidence back to durable
  storage on housekeeping CPUs after the final measurement stage.

The runtime IRQ guard has no dormant-vector exception. The booted host must expose no PCI
MSI/MSI-X vectors, and every effective IRQ affinity must exclude CPUs `1-4`. Any violation fails
closed and requires a clean host recovery before another sample.

Run these helpers only through the reviewed `0.67`/`0.67.1` GitHub workflow. A direct
`taskset --cpu-list 1-4 cargo ...` command is forbidden because compiler and artifact I/O can
activate immutable managed NVMe queues on a measurement CPU.

The first qualification after introducing the tmpfs wrapper proved why this distinction is
load-bearing: the seven calibration probes themselves were stable (`0.0011` spread), but a host
probe launched by the measurement-pinned child submitted one root-filesystem read on CPU `1`.
Managed NVMe IRQ `128` (`nvme0q2`, immutable effective affinity `1`) fired once, so the post-guard
correctly rejected the run. External attestation probes now execute on housekeeping CPUs while the
same process remains measurement-pinned for fingerprint affinity and calibration. The rejected run
is diagnostic-only and does not count toward bootstrap.

A follow-up qualification (`30560237096`) passed the same seven-probe preflight at `0.0011` spread
but exposed a narrower implementation gap: the bare-metal `systemd-detect-virt --quiet` branch
still used a direct process launch instead of the reviewed housekeeping dispatcher. Managed NVMe
IRQ `106` (`nvme0q2`, immutable effective affinity `1`) consequently recorded one interrupt. Both
the quiet and named virtualization probes now share the dispatcher, and governance rejects any
direct `Command::new("systemd-detect-virt")` path.

The next qualification (`30566810245`) passed attestation and the seven-probe preflight at `0.0016`
spread, then the post-guard observed one interrupt on managed NVMe IRQ `118` (`nvme0q2`, immutable
effective affinity `1`). The measurement wrapper warmed a nonexistent
`docs/testing/perf-profiles/0.67` subtree, while the pinned preflight actually reads
`docs/testing/perf-profiles/reference-v1.toml`; repository-root discovery also probes
`docs/plans/releases.toml`. Both exact regular-file inputs are now prefaulted on housekeeping CPUs
before the measurement child starts, and the stale subtree selector is removed.

Qualification `30572949833` then passed the v5 attestation and seven-probe preflight at `0.0012`
spread, but the prebuild gate correctly rejected controller affinity `0,5-7` because the legacy
runner observation still expected measurement affinity `1-4` on the Cargo process itself. The
0.67.1 prebuild now keeps Cargo on housekeeping CPUs and reports measurement affinity `1-4` only
when the exact `qualify`, `full-dress`, or `bootstrap` mode, current housekeeping affinity, attested housekeeping
set, and attested isolated set all agree. Any mode or cpuset drift still fails closed; the build is
never moved onto a measurement CPU.

Qualification `30580548936` passed the same v5 attestation and seven-probe preflight at
`0.1259046089844615` spread, then exposed a deterministic publication mismatch: the committed
tmpfs wrapper correctly made `target/test-evidence/0.67` a symlink, while the legacy atomic
prebuild publisher rejected every symlink. The publisher still accepts ordinary outputs only at
their canonical in-repository path; the sole exception is the exact `0.67` tmpfs target when the
mode is `qualify`, `full-dress`, or `bootstrap`, GitHub Actions and `GITHUB_SHA` match the source snapshot, the
link target and `source-commit` marker are exact, and `findmnt` independently reports `tmpfs`.
Every other path, mode, commit, marker, link, or filesystem fails closed.

Qualification `30585767852` passed attestation, the seven-probe preflight at
`0.0014360058485199713` spread, prebuild, and both bounded diagnostics. Finalization then rejected
the bundle because the attestation and prebuild storage fingerprints differed. Exact retained
payload comparison proved that every stable field matched except `storage_identity_digest`:
plain `lsblk --inverse` tree decoration caused the parser to omit the indented RAID1 leaf and hash
whichever NVMe device happened to be rendered as the final branch. The storage probe now requests
`lsblk --raw`, parses both normalized NVMe disk rows, collapses padding before hashing, and keeps the
existing order-independent privacy digest. A regression covers both RAID1 leaves and reversed
enumeration order. Raw model, serial, and WWN values remain confined to the in-process digest input
and are never serialized.

This correction does **not** change the SLOs, request schedules, repetitions, zero-error rule,
frozen `0.15` spread limit, affinity set, quota rule, or non-ship bootstrap boundary.

Bootstrap acquisition `30592637715` was rejected before measurement because its workflow imported
the offline provisioning receipt before preparing tmpfs. The import correctly materialized
`target/test-evidence/0.67.1`, while `reference-evidence-tmpfs.sh prepare` correctly requires both
evidence paths to be absent before creating exact tmpfs links. Bootstrap now follows qualification
by preparing tmpfs before importing the receipt, and a runner-contract regression fixes that order.
The rejected run is not a bootstrap sample and no frozen contract changed.

## Placeholders

Replace these values only where explicitly instructed:

| Placeholder | Meaning | Example |
| --- | --- | --- |
| `SERVER_IP` | Elastic Metal public IPv4 | `203.0.113.10` |
| `ADMIN_SOURCE_IP` | current administrator public IPv4, without a subnet | `198.51.100.25` |
| `RUNNER_VERSION` | runner version shown by GitHub's **New self-hosted runner** page | `2.x.y` |
| `RUNNER_SHA256` | Linux x64 archive SHA-256 shown on that page | 64 hexadecimal characters |

Never put the following values in this repository, an Actions artifact, a shell transcript, or a
support ticket:

- the SSH private key;
- a GitHub runner registration/removal token;
- a Scaleway API key, project id, server id, or billing identifier;
- a raw DMI UUID, disk serial number, or cloud instance identity.

## 0. Before creating the billed server

### 0.1 Create and verify the local SSH key

Run in Windows PowerShell. If both `Test-Path` calls already return `True`, do not overwrite the
key; skip `ssh-keygen`.

```powershell
Test-Path "$env:USERPROFILE\.ssh\hydracache-perf-v1"
Test-Path "$env:USERPROFILE\.ssh\hydracache-perf-v1.pub"
```

Create it only when absent:

```powershell
ssh-keygen -t ed25519 -a 100 `
  -f "$env:USERPROFILE\.ssh\hydracache-perf-v1" `
  -C "hydracache-perf-v1"
```

Verify and display only the public key:

```powershell
ssh-keygen -lf "$env:USERPROFILE\.ssh\hydracache-perf-v1.pub"
Get-Content "$env:USERPROFILE\.ssh\hydracache-perf-v1.pub"
```

Upload the complete `ssh-ed25519 ...` public line to Scaleway with the name
`hydracache-perf-v1-admin`. Never upload or display the file without `.pub`.

### 0.2 Record the local key fingerprint

Keep the fingerprint in a private operator note:

```powershell
ssh-keygen -lf "$env:USERPROFILE\.ssh\hydracache-perf-v1.pub" -E sha256
```

### 0.3 Create the Elastic Metal host

Use:

- product: **Elastic Metal**, not a virtual Compute Instance;
- server: `EM-B220E-NVMe` or another W2-qualifying true bare-metal SKU;
- billing: **Hourly**;
- image: **Ubuntu 24.04 LTS**, x86_64;
- server name: `hydracache-perf-v1`;
- disk layout: default;
- cloud-init: disabled;
- SSH key: `hydracache-perf-v1-admin`;
- no private network, extra public bandwidth, snapshot, or other paid option unless separately
  approved.

Billing continues while a powered-off Elastic Metal server remains allocated. Only deleting the
server stops server billing.

## 1. First connection as root

Wait until Scaleway reports the server as `Ready`. Replace `SERVER_IP` locally:

```powershell
$ServerIp = "SERVER_IP"
ssh -i "$env:USERPROFILE\.ssh\hydracache-perf-v1" "root@$ServerIp"
```

On first connection, compare the presented host-key fingerprint with the fingerprint shown by the
provider console if available. Otherwise treat the first connection as trust-on-first-use and
record the accepted host key in the private operator note.

Once connected, verify the installed image before changing anything:

```bash
set -euo pipefail
uname -a
uname -m
. /etc/os-release
printf 'os=%s version=%s\n' "$ID" "$VERSION_ID"
test "$(uname -m)" = "x86_64"
test "$ID" = "ubuntu"
test "$VERSION_ID" = "24.04"
```

Stop if any assertion fails.

## 2. Initial hardware triage before spending more time

These commands deliberately avoid raw UUIDs and serial numbers:

```bash
set -euo pipefail
printf '%s\n' '=== virtualization ==='
if systemd-detect-virt --quiet; then
  systemd-detect-virt
  echo 'FAIL: virtualization detected; this host is ineligible'
  exit 1
else
  echo 'none'
fi

printf '%s\n' '=== cpu ==='
lscpu
physical_cores="$(
  lscpu --parse=SOCKET,CORE |
    grep --invert-match '^#' |
    sort --unique |
    wc --lines
)"
printf 'physical_cores=%s\n' "$physical_cores"
test "$physical_cores" -ge 6

printf '%s\n' '=== measurement cpuset topology ==='
for cpu in 1 2 3 4; do
  test -d "/sys/devices/system/cpu/cpu${cpu}"
  package="$(cat "/sys/devices/system/cpu/cpu${cpu}/topology/physical_package_id")"
  core="$(cat "/sys/devices/system/cpu/cpu${cpu}/topology/core_id")"
  printf 'cpu=%s package=%s core=%s\n' "$cpu" "$package" "$core"
done
distinct_measurement_cores="$(
  for cpu in 1 2 3 4; do
    paste -d: \
      "/sys/devices/system/cpu/cpu${cpu}/topology/physical_package_id" \
      "/sys/devices/system/cpu/cpu${cpu}/topology/core_id"
  done | sort --unique | wc --lines
)"
printf 'distinct_measurement_cores=%s\n' "$distinct_measurement_cores"
test "$distinct_measurement_cores" -eq 4

printf '%s\n' '=== memory ==='
free --gibi
memory_kib="$(awk '/^MemTotal:/ {print $2}' /proc/meminfo)"
test "$memory_kib" -ge 16777216

printf '%s\n' '=== storage ==='
lsblk --nodeps --output NAME,TYPE,SIZE,ROTA,TRAN,MODEL
test -n "$(lsblk --nodeps --noheadings --output NAME,TRAN | awk '$2 == "nvme" {print $1}')"

printf '%s\n' '=== cgroup ==='
test "$(stat --file-system --format=%T /sys/fs/cgroup)" = "cgroup2fs"
cgroup_path="$(awk -F: '$1 == "0" && $2 == "" {print $3}' /proc/self/cgroup)"
test -n "$cgroup_path"
case "$cgroup_path" in /*) ;; *) exit 1 ;; esac
cgroup_cursor="/sys/fs/cgroup${cgroup_path%/}"
cpu_controller_observed=false
while :; do
  if test -f "$cgroup_cursor/cpu.max"; then
    cgroup_cpu_max="$(cat "$cgroup_cursor/cpu.max")"
    printf 'effective_cpu_max[%s]=%s\n' "$cgroup_cursor" "$cgroup_cpu_max"
    test "$(printf '%s\n' "$cgroup_cpu_max" | awk '{print $1}')" = "max"
    cpu_controller_observed=true
  fi
  test "$cgroup_cursor" != "/sys/fs/cgroup" || break
  cgroup_cursor="${cgroup_cursor%/*}"
done
test "$cpu_controller_observed" = true
```

If the host reports a hypervisor, fewer than six physical cores, fewer than four distinct cores for
CPUs `1-4`, less than 16 GiB RAM, non-NVMe storage, cgroup v1, an unavailable CPU controller, or a
CPU quota at any effective cgroup ancestor, stop. Do not register it as `hydracache-perf-v1`.

## 3. Patch once and install host dependencies

Still as `root`:

```bash
set -euo pipefail
export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get dist-upgrade --yes
apt-get install --yes \
  build-essential \
  ca-certificates \
  clang \
  cmake \
  curl \
  dmidecode \
  git \
  jq \
  libssl-dev \
  make \
  numactl \
  nvme-cli \
  pkg-config \
  sysstat \
  uidmap \
  dbus-user-session \
  slirp4netns \
  fuse-overlayfs \
  unzip \
  util-linux \
  ufw
apt-get autoremove --yes
```

Reboot after the initial upgrade:

```bash
reboot
```

Reconnect after the server returns to `Ready`:

```powershell
ssh -i "$env:USERPROFILE\.ssh\hydracache-perf-v1" "root@$ServerIp"
```

Verify that no reboot remains pending:

```bash
set -euo pipefail
test ! -e /var/run/reboot-required
```

## 4. Create the administrator account

Use an administrator account for later SSH access and reserve `github-runner` for Actions only.
As `root`:

```bash
set -euo pipefail
adduser --gecos '' hydracache-admin
usermod --append --groups sudo hydracache-admin
install --directory --mode=0700 --owner=hydracache-admin --group=hydracache-admin \
  /home/hydracache-admin/.ssh
install --mode=0600 --owner=hydracache-admin --group=hydracache-admin \
  /root/.ssh/authorized_keys \
  /home/hydracache-admin/.ssh/authorized_keys
```

`adduser` asks for an administrator sudo password. Store it in a password manager; it is not the
SSH key passphrase and must not enter the repository.

Open a second local PowerShell window and verify the new account before closing the root session:

```powershell
ssh -i "$env:USERPROFILE\.ssh\hydracache-perf-v1" `
  "hydracache-admin@$ServerIp" `
  "id; sudo -v; echo admin-access-ok"
```

Do not continue unless the command prints `admin-access-ok`.

## 5. Restrict SSH and enable the firewall

Determine the current administrator public IPv4 from a trusted local network source. Do not use the
server's own address. As `root`, replace `ADMIN_SOURCE_IP`:

```bash
set -euo pipefail
ADMIN_SOURCE_IP="ADMIN_SOURCE_IP"
test "$ADMIN_SOURCE_IP" != "ADMIN_SOURCE_IP"

ufw default deny incoming
ufw default allow outgoing
ufw allow from "${ADMIN_SOURCE_IP}/32" to any port 22 proto tcp
ufw --force enable
ufw status verbose
```

If the administrator has a changing public address, use a deliberately approved source range or
temporarily allow TCP/22 globally. Never enable the firewall without first allowing a working SSH
path.

Harden `sshd` only after the second administrator session is proven:

```bash
set -euo pipefail
install --directory --mode=0755 /etc/ssh/sshd_config.d
cat >/etc/ssh/sshd_config.d/90-hydracache-perf.conf <<'EOF'
PasswordAuthentication no
KbdInteractiveAuthentication no
PermitRootLogin no
PubkeyAuthentication yes
X11Forwarding no
AllowAgentForwarding no
AllowTcpForwarding no
EOF
sshd -t
systemctl reload ssh
```

Open one more new administrator session before closing the existing root session:

```powershell
ssh -i "$env:USERPROFILE\.ssh\hydracache-perf-v1" `
  "hydracache-admin@$ServerIp" `
  "echo hardened-ssh-ok"
```

## 6. Create the unprivileged Actions account

From this point onward, connect as `hydracache-admin`:

```powershell
ssh -i "$env:USERPROFILE\.ssh\hydracache-perf-v1" `
  "hydracache-admin@$ServerIp"
```

Create a locked, unprivileged service account:

```bash
set -euo pipefail
sudo useradd \
  --create-home \
  --home-dir /home/github-runner \
  --shell /bin/bash \
  github-runner
sudo passwd --lock github-runner
id github-runner
sudo -u github-runner test ! -w /etc
```

Do not add `github-runner` to `sudo`, `docker`, `lxd`, or another privileged group. The workflow
executes repository code under this account.
Create the non-secret local runner contract used by the audit helpers:

```bash
sudo install --directory --mode=0755 /etc/hydracache-perf
sudo tee /etc/hydracache-perf/runner-contract.json >/dev/null <<'EOF'
{
  "schema_version": 1,
  "repository": "javaquasar/hydracache",
  "runner_name": "hydracache-perf-v1",
  "labels": ["self-hosted", "linux", "x64", "hydracache-perf-v1"],
  "service_user": "github-runner"
}
EOF
sudo chmod 0644 /etc/hydracache-perf/runner-contract.json
jq --exit-status '.runner_name == "hydracache-perf-v1"' \
  /etc/hydracache-perf/runner-contract.json >/dev/null
```



### 6.1 Install pinned rootless Docker for the mandatory Redis comparison

The W4 RESP family includes the existing same-box Redis comparison, whose frozen 0.67 contract
uses Docker. Granting access to a rootful Docker socket would be equivalent to granting root and is
forbidden. Install the official rootless packages instead, following Docker's
[rootless-mode contract](https://docs.docker.com/engine/security/rootless/) and Ubuntu package
instructions. The pinned version below is part of the host fingerprint; if the repository no longer
serves it, stop and update the plan through review instead of silently selecting another version.

As `hydracache-admin`:

```bash
set -euo pipefail
DOCKER_VERSION='5:29.6.1-1~ubuntu.24.04~noble'

sudo install -m 0755 -d /etc/apt/keyrings
sudo curl --fail --location --silent --show-error \
  https://download.docker.com/linux/ubuntu/gpg \
  -o /etc/apt/keyrings/docker.asc
sudo chmod a+r /etc/apt/keyrings/docker.asc
sudo tee /etc/apt/sources.list.d/docker.sources >/dev/null <<'EOF'
Types: deb
URIs: https://download.docker.com/linux/ubuntu
Suites: noble
Components: stable
Architectures: amd64
Signed-By: /etc/apt/keyrings/docker.asc
EOF
sudo apt-get update
apt-cache madison docker-ce | grep --fixed-strings "$DOCKER_VERSION"
sudo apt-get install --yes \
  docker-ce="$DOCKER_VERSION" \
  docker-ce-cli="$DOCKER_VERSION" \
  containerd.io \
  docker-buildx-plugin \
  docker-ce-rootless-extras="$DOCKER_VERSION"

sudo systemctl disable --now docker.service docker.socket containerd.service
for unit in docker.service docker.socket containerd.service; do
  test "$(systemctl is-active "$unit" || true)" = "inactive"
done
sudo rm --force /var/run/docker.sock

grep --quiet '^github-runner:' /etc/subuid
grep --quiet '^github-runner:' /etc/subgid
sudo loginctl enable-linger github-runner
runner_uid="$(id --user github-runner)"
sudo systemctl start "user@${runner_uid}.service"
test "$(systemctl is-active "user@${runner_uid}.service")" = "active"
sudo -iu github-runner env \
  XDG_RUNTIME_DIR="/run/user/${runner_uid}" \
  DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/${runner_uid}/bus" \
  dockerd-rootless-setuptool.sh install
sudo -iu github-runner env \
  XDG_RUNTIME_DIR="/run/user/${runner_uid}" \
  DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/${runner_uid}/bus" \
  systemctl --user disable --now docker.service
```

Verify once, then leave the rootless daemon stopped:

```bash
set -euo pipefail
runner_uid="$(id --user github-runner)"
sudo -iu github-runner env \
  XDG_RUNTIME_DIR="/run/user/${runner_uid}" \
  DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/${runner_uid}/bus" \
  DOCKER_HOST="unix:///run/user/${runner_uid}/docker.sock" \
  bash -c '
    systemctl --user start docker.service
    docker info --format "{{json .SecurityOptions}}" | grep --quiet rootless
    systemctl --user stop docker.service
  '
test ! -S /var/run/docker.sock
```

The committed `scripts/perf/rootless-docker.sh` helper and workflow start this rootless daemon only
around the RESP/Redis comparison and stop it with an `always()` cleanup step. The service must remain disabled and stopped during W1 offline
audit, qualification, core measurements, and control-plane measurements.
## 7. Disable background package activity for the measurement window

Apply this only to a short-lived host that will be deleted after evidence collection:

```bash
set -euo pipefail
sudo systemctl disable --now unattended-upgrades.service || true
sudo systemctl disable --now apt-daily.timer apt-daily-upgrade.timer
sudo systemctl mask apt-daily.service apt-daily-upgrade.service
systemctl is-enabled apt-daily.timer apt-daily-upgrade.timer || true
systemctl list-timers --all
```

Keep time synchronization and SSH enabled. Apart from the pinned, normally stopped rootless Docker
required by W4, do not install Kubernetes, monitoring agents, or unrelated workloads.

## 8. Set and persist the CPU policy

The reference contract requires a stable governor and turbo policy. This runbook chooses
`performance` governor with boost enabled and records that choice. If the hardware cannot expose
these controls consistently, stop and let W2 reject it.

Create a small root-owned systemd unit:

```bash
set -euo pipefail
sudo tee /usr/local/sbin/hydracache-perf-cpu-policy >/dev/null <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

for governor in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do
  test -w "$governor"
  printf 'performance' >"$governor"
done

if test -w /sys/devices/system/cpu/cpufreq/boost; then
  printf '1' >/sys/devices/system/cpu/cpufreq/boost
fi

if test -w /sys/devices/system/cpu/intel_pstate/no_turbo; then
  printf '0' >/sys/devices/system/cpu/intel_pstate/no_turbo
fi
EOF
sudo chmod 0755 /usr/local/sbin/hydracache-perf-cpu-policy

sudo tee /etc/systemd/system/hydracache-perf-cpu-policy.service >/dev/null <<'EOF'
[Unit]
Description=HydraCache reference runner CPU policy
After=multi-user.target

[Service]
Type=oneshot
ExecStart=/usr/local/sbin/hydracache-perf-cpu-policy
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable --now hydracache-perf-cpu-policy.service
```

Verify every exposed governor and record the boost policy:

```bash
set -euo pipefail
grep --no-filename . /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor |
  sort --unique
test "$(
  grep --no-filename . /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor |
    sort --unique
)" = "performance"

if test -r /sys/devices/system/cpu/cpufreq/boost; then
  printf 'boost=%s\n' "$(cat /sys/devices/system/cpu/cpufreq/boost)"
fi
if test -r /sys/devices/system/cpu/intel_pstate/no_turbo; then
  printf 'intel_no_turbo=%s\n' "$(cat /sys/devices/system/cpu/intel_pstate/no_turbo)"
fi
```

W2 confirmed one 8-core/16-thread package with measurement CPUs `1-4` paired with SMT siblings
`9-12`. Three independent bootstrap acquisitions then failed the unchanged 15% spread gate in
different workload families while preflight remained stable. Before any further qualification or
bootstrap run, install the reviewed v5 isolation policy from a clean trusted-`main` checkout while
the Actions runner and rootless Docker are offline:

```bash
set -euo pipefail
scripts/perf/runner-service.sh offline
sudo scripts/perf/provision-reference-isolation.sh install
# If the helper reports that a reboot is required, run:
# sudo reboot
```

If the helper requested a reboot, reconnect afterward. Then verify the exact policy before generating
the root-owned receipt:

```bash
set -euo pipefail
sudo scripts/perf/provision-reference-isolation.sh verify
```

The committed policy is intentionally host-specific and fail-closed:

- SMT is disabled, leaving logical CPUs `0-7` online; depending on the kernel, siblings `9-12`
  are absent from topology sysfs or remain enumerated with `online=0`;
- measurement CPUs `1-4` are isolated with `isolcpus`, `nohz_full`, and `rcu_nocbs`;
- every online CPU `0-7`, including measurement CPUs `1-4` and housekeeping CPUs `0,5-7`, may use
  only idle states with exit latency at most `1` microsecond; the root-owned
  `hydracache-perf-idle-policy.service` disables every deeper state before the runner starts;
- CPUs `0,5-7` are the only housekeeping set for the Actions service and rootless Docker daemon;
- active IRQ work must not reach CPUs `1-4`; the reviewed boot policy uses
  `pci=nomsi`, requires the platform's legacy INTx fallback, and rejects every
  remaining MSI/MSI-X vector so `irqaffinity=0,5-7` can keep device IRQ work on
  housekeeping CPUs; installation checks that every NVMe controller advertises
  a routed INTx pin before changing the boot policy;
- the Redis container is explicitly pinned to CPUs `1-4`, while Docker control work stays on the
  housekeeping set.

The helper writes a GRUB drop-in, root-owned system/user service drop-ins, and a root-owned
oneshot idle-policy service, then runs `update-grub`.
A first install requires an operator-controlled reboot; an upgrade on an already exact isolated
kernel enables and restarts the root-owned oneshot so the updated idle policy is applied immediately
even when that unit was already active. It never reboots the host itself. The verify action
rejects missing or duplicate kernel arguments, online SMT siblings, unexpected systemd affinity,
or any observed IRQ that reaches the measurement set.

The SMT/IRQ isolation correction changed the reference fingerprint from v3 to v4. The next v4
bootstrap passed preflight (`0.002829`) and core/redis-benchmark stability but rejected all six
low-duty RESP scheduled-send p99 points at spreads `0.238662`-`0.481121` (plus the bounded probe at
`0.602819`). The run had zero errors, timeouts, and rejections. A live audit then proved C2 remained
enabled with 400 microseconds exit latency on housekeeping CPUs `0,5-7`, which own timer/IRQ/control
work. The current v5 policy applies and independently probes the same 1 microsecond cap on both CPU
roles. Every v3/v4 qualification or bootstrap artifact remains diagnostic history only; none may be
combined with v5 samples. Re-run the offline audit, qualification, and bootstrap acquisition from
the exact post-merge commit.

## 9. Prepare the Actions runner download

In GitHub, open:

`javaquasar/hydracache` â†’ **Settings** â†’ **Actions** â†’ **Runners** â†’
**New self-hosted runner** â†’ **Linux** â†’ **x64**

The repository-level runner is restricted to `javaquasar/hydracache`. If an organization runner
group is available, additionally restrict it to this repository and the CI workflow. Copy the
current runner version, Linux x64 download URL, and SHA-256 from GitHub's generated instructions.
Do not copy the registration token into this document.

As `hydracache-admin`, replace `RUNNER_VERSION` and `RUNNER_SHA256`:

```bash
set -euo pipefail
RUNNER_VERSION="RUNNER_VERSION"
RUNNER_SHA256="RUNNER_SHA256"
test "$RUNNER_VERSION" != "RUNNER_VERSION"
printf '%s' "$RUNNER_SHA256" | grep --quiet --extended-regexp '^[0-9a-f]{64}$'

sudo install --directory --mode=0755 --owner=github-runner --group=github-runner \
  /opt/actions-runner
sudo -u github-runner curl \
  --proto '=https' \
  --tlsv1.2 \
  --fail \
  --location \
  --show-error \
  --output "/opt/actions-runner/actions-runner-linux-x64-${RUNNER_VERSION}.tar.gz" \
  "https://github.com/actions/runner/releases/download/v${RUNNER_VERSION}/actions-runner-linux-x64-${RUNNER_VERSION}.tar.gz"

printf '%s  %s\n' \
  "$RUNNER_SHA256" \
  "/opt/actions-runner/actions-runner-linux-x64-${RUNNER_VERSION}.tar.gz" |
  sha256sum --check --strict -

sudo -u github-runner tar \
  --extract \
  --gzip \
  --file "/opt/actions-runner/actions-runner-linux-x64-${RUNNER_VERSION}.tar.gz" \
  --directory /opt/actions-runner
sudo /opt/actions-runner/bin/installdependencies.sh
```

If GitHub's generated URL differs, use the exact generated URL. Never skip the SHA-256 check.

## 10. Register the repository runner

Generate a fresh registration token on the same GitHub page immediately before this step. It is
short-lived.

Read it without echoing or placing it literally in shell history:

```bash
set -euo pipefail
read -r -s -p 'GitHub runner registration token: ' RUNNER_TOKEN
printf '\n'
sudo -u github-runner \
  /opt/actions-runner/config.sh \
  --unattended \
  --url https://github.com/javaquasar/hydracache \
  --token "$RUNNER_TOKEN" \
  --name hydracache-perf-v1 \
  --labels hydracache-perf-v1 \
  --work _work
unset RUNNER_TOKEN
```

Default labels must remain enabled. GitHub should show:

```text
self-hosted
linux
x64
hydracache-perf-v1
```

Install the service under the unprivileged account, but keep it stopped until an authorized manual
run is ready:

```bash
set -euo pipefail
cd /opt/actions-runner
sudo ./svc.sh install github-runner
sudo ./svc.sh start
sudo ./svc.sh status
sudo ./svc.sh stop
```

Confirm in GitHub that the runner exists and is `Offline`. An offline runner does not stop Scaleway
billing; it only prevents Actions jobs from being accepted.
From a clean trusted-`main` HydraCache checkout, verify the registered service and deliberately
leave it offline. The offline audit must be made for the exact commit that will be dispatched:

```bash
sudo -v
scripts/perf/verify-runner-service.sh --expected-label hydracache-perf-v1
scripts/perf/runner-service.sh offline
scripts/perf/audit-reference-host.sh --mode provisioned
sudo install -d -o root -g root -m 0755 /var/lib/hydracache-perf
sudo install -o root -g root -m 0444 \
  target/test-evidence/0.67.1/runner-provisioned.json \
  /var/lib/hydracache-perf/runner-provisioned.json
```

The audit intentionally records `runner_online=false` and `ship_evidence_eligible=false`.
Qualification imports this root-owned receipt and rejects it if its commit differs from the
workflow checkout. Any new `main` commit therefore requires a fresh offline audit before dispatch.
The DMI UUID and serial inputs remain root-readable only. The audit writes only their
domain-separated SHA-256 digest as `host_identity_digest`; it never prints or copies the raw
identifiers. The unprivileged Actions runner consumes that protected, commit-bound digest when
building the reference host fingerprint.



## 11. Provisioned-state audit

Run as `hydracache-admin` while the Actions service is offline. The committed W1 helper performs
the authoritative audit; the commands below are additional human-readable checks:

```bash
set -euo pipefail
sudo -v

test "$(uname -m)" = "x86_64"
. /etc/os-release
test "$ID" = "ubuntu"
test "$VERSION_ID" = "24.04"

if systemd-detect-virt --quiet; then
  echo 'FAIL: virtualization detected'
  systemd-detect-virt
  exit 1
fi

test "$(stat --file-system --format=%T /sys/fs/cgroup)" = "cgroup2fs"
cgroup_path="$(awk -F: '$1 == "0" && $2 == "" {print $3}' /proc/self/cgroup)"
test -n "$cgroup_path"
case "$cgroup_path" in /*) ;; *) exit 1 ;; esac
cgroup_cursor="/sys/fs/cgroup${cgroup_path%/}"
cpu_controller_observed=false
while :; do
  if test -f "$cgroup_cursor/cpu.max"; then
    cgroup_cpu_max="$(cat "$cgroup_cursor/cpu.max")"
    test "$(printf '%s\n' "$cgroup_cpu_max" | awk '{print $1}')" = "max"
    cpu_controller_observed=true
  fi
  test "$cgroup_cursor" != "/sys/fs/cgroup" || break
  cgroup_cursor="${cgroup_cursor%/*}"
done
test "$cpu_controller_observed" = true

physical_cores="$(
  lscpu --parse=SOCKET,CORE |
    grep --invert-match '^#' |
    sort --unique |
    wc --lines
)"
test "$physical_cores" -ge 6

distinct_measurement_cores="$(
  for cpu in 1 2 3 4; do
    paste -d: \
      "/sys/devices/system/cpu/cpu${cpu}/topology/physical_package_id" \
      "/sys/devices/system/cpu/cpu${cpu}/topology/core_id"
  done | sort --unique | wc --lines
)"
test "$distinct_measurement_cores" -eq 4

test "$(
  grep --no-filename . /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor |
    sort --unique
)" = "performance"

test -n "$(lsblk --nodeps --noheadings --output NAME,TRAN | awk '$2 == "nvme" {print $1}')"
test "$(id --user github-runner)" -gt 0
if id --groups --name github-runner | grep --quiet --word-regexp sudo; then
  echo 'FAIL: github-runner is privileged'
  exit 1
fi

sudo systemctl is-enabled actions.runner.javaquasar-hydracache.hydracache-perf-v1.service
sudo systemctl is-active actions.runner.javaquasar-hydracache.hydracache-perf-v1.service &&
  {
    echo 'FAIL: runner must remain offline before authorized dispatch'
    exit 1
  }

echo 'provisioned-state-audit=pass'
```

The exact systemd service name may include an escaped repository or runner name. If the expected
name is absent, find it without changing state:

```bash
systemctl list-unit-files 'actions.runner.*'
```

Do not turn a mismatch into a bypass; update the audit with the observed exact unit name.

## 12. Qualification and reference execution

This section is intentionally blocked until W2 and W3 are merged to trusted `main`.

Before every authorized qualification run:

1. Update the administrative checkout to the exact clean trusted `main` commit to dispatch.
2. Confirm no apt activity, maintenance, or other workload is running.
3. Keep the runner offline, run `audit-reference-host.sh`, and install the root-owned receipt as
   shown above.
4. Start the runner service.
5. Dispatch `CI` with `candidate_release=0.67.1` and `performance_0671_mode=qualify`; leave
   `run_reference_performance=false` and `gated_gate_id` empty.
6. The job imports W1, runs W2 attestation/preflight, exact prebuild, bounded smoke diagnostics,
   and W3 finalization. It cannot activate an anchor or emit bootstrap/ship-eligible evidence.
7. Monitor the job and archive its complete artifact even on failure.
8. Stop the service immediately after the job reaches a terminal state.

Service lifecycle:

```bash
cd /opt/actions-runner
sudo ./svc.sh start
sudo ./svc.sh status
```

After the GitHub job completes:

```bash
cd /opt/actions-runner
sudo ./svc.sh stop
sudo ./svc.sh status
```

Do **not** dispatch the currently shipped job with `candidate_release=0.67` as a substitute for
0.67.1 qualification. Qualification does not count among the five W4 bootstrap samples, and a
bootstrap sample is still `ship_evidence_eligible=false` until reviewed activation.

After qualification is green, keep the exact same host and commit. Before bootstrap, dispatch two
independent complete runs with `candidate_release=0.67.1` and
`performance_0671_mode=full-dress`. Leave `full_dress_predecessor_run_id` empty on the first run.
After downloading and accepting its immutable receipt, set that first run id on the second run.
Both execute the same core, rootless-Docker/RESP, and control-plane families as bootstrap, but both
remain qualification-only and non-promotable. The second run publishes
`performance-0671-full-dress-admission` only when source, host, immutable runner-provisioning
receipt, prebuild, scenario, and run identity checks agree.

Then dispatch exactly five serialized runs with `candidate_release=0.67.1`,
`performance_0671_mode=bootstrap`, and `full_dress_admission_run_id` equal to the second full-dress
run id. Set `bootstrap_sample_index=1..5`. Sample 1 must leave
`bootstrap_predecessor_run_id` empty; each later sample must name the immediately prior accepted
sample's run id. Do not pre-queue a successor and do not use an automatic retry as a successor:
the prior immutable receipt must exist before the next dispatch. Do not select the old
`run_reference_performance` input. Each successful run uploads a distinct `bootstrap-sample.json`;
a failed or unstable run is retained for diagnosis but does not count and cannot authorize the
next index.

Download the five successful artifacts before stopping the server, copy only their original
`bootstrap-sample.json` files under unique names, and validate the set locally:

```bash
mkdir -p target/bootstrap-samples
# Copy downloaded receipts as target/bootstrap-samples/<github-run-id>.json.
cargo xtask perf-bootstrap --release 0.67.1 --profile reference-v1 \
  --phase sample-set --samples-dir target/bootstrap-samples
```

Do not delete the host until `bootstrap-sample-set.json` is produced successfully and all five
artifact archives are independently retained. This check requires exact indices `1..=5` and
verifies every predecessor receipt digest. It rejects mixed fingerprints, source commits,
admissions, contracts, scenario sets, duplicate run ids, broken/reordered chains, failed runs, and
any sample marked as ship evidence.

## 13. Emergency stop

If unexpected repository code is selected, the host changes, or a job behaves incorrectly:

```bash
cd /opt/actions-runner
sudo ./svc.sh stop
```

Then cancel the workflow in GitHub. Preserve logs and artifacts; do not edit a failed result into a
passing sample.

Powering off the Elastic Metal host does not stop billing. Delete it only after deciding that the
host must not be retained for the five-run sample family.

## 14. Remove the runner and delete the host

Do this only after:

- all required GitHub artifacts were uploaded and their digests recorded;
- no additional W4 run is needed from this physical host;
- the runner has no active or queued job;
- the evidence review does not require another observation from the same host.

Generate a fresh runner removal token from the repository runner settings. Then:

```bash
set -euo pipefail
cd /opt/actions-runner
sudo ./svc.sh stop || true
sudo ./svc.sh uninstall

read -r -s -p 'GitHub runner removal token: ' REMOVE_TOKEN
printf '\n'
sudo -u github-runner ./config.sh remove --token "$REMOVE_TOKEN"
unset REMOVE_TOKEN
```

Verify in GitHub that `hydracache-perf-v1` no longer appears under repository runners.

In Scaleway:

1. Delete the Elastic Metal server; powering it off is insufficient.
2. Confirm the server disappears from the Elastic Metal inventory.
3. Check that no paid Flexible IPv4, Private Network, snapshot, or other attached resource remains.
4. Check Cost Manager for the final metered interval.
5. Remove the old SSH host key locally only after the server has been deleted:

```powershell
ssh-keygen -R $ServerIp
```

Keep the local administrator SSH key for a later host only if that reuse is intentional. Never
commit it.

## Operator completion checklist

- [ ] Local Ed25519 private key exists only on the administrator workstation.
- [ ] True Elastic Metal host uses hourly billing and Ubuntu 24.04 x86_64.
- [ ] Initial triage finds no virtualization and proves CPU, RAM, NVMe, and cgroup requirements.
- [ ] Initial package upgrade and reboot are complete.
- [ ] `hydracache-admin` works with key-only SSH and sudo.
- [ ] Root SSH and password authentication are disabled.
- [ ] SSH ingress is restricted; other ingress is denied.
- [ ] `github-runner` is unprivileged.
- [ ] Background package timers are disabled for the measurement window.
- [ ] Governor and turbo policy are fixed and recorded.
- [ ] Runner archive checksum is verified.
- [ ] Runner has exactly the required custom label plus GitHub default labels.
- [ ] Runner service remains offline outside authorized manual runs.
- [ ] No 0.67.1 job is dispatched before W2/W3 are merged.
- [ ] Every run artifact is preserved, including failures.
- [ ] Runner is removed from GitHub before host deletion.
- [ ] Elastic Metal host and separately billed attached resources are deleted.
