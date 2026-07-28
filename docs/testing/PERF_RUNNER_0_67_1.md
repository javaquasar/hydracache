# Release 0.67.1 dedicated performance runner runbook

This runbook provisions one short-lived Scaleway Elastic Metal host for release `0.67.1`
qualification and reference bootstrap. It is written for the provisional
`EM-B220E-NVMe` host (AMD EPYC 7232P, 8C/16T, 64 GiB RAM, local NVMe) but the checks are
provider-neutral.

The commands are intentionally split into local Windows PowerShell, remote root, remote
administrator, and GitHub runner-user sections. Do not paste an entire section blindly. Run one
block at a time, inspect its output, and stop on any failed assertion.

> **Evidence boundary**
>
> Provisioning and runner registration do not approve the machine. Until W2 machine attestation
> and W3 qualification mode from the
> [0.67.1 plan](../plans/V0_67_1_DEDICATED_PERFORMANCE_REFERENCE_BOOTSTRAP_PLAN.md) are implemented
> and green, do not dispatch the current `0.67 Self-hosted Reference Evidence` job. Exploratory
> output is not release evidence and must not be quoted as a capacity, sizing, baseline, or Redis
> comparison claim.

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
cat /sys/fs/cgroup/cpu.max
test "$(awk '{print $1}' /sys/fs/cgroup/cpu.max)" = "max"
```

If the host reports a hypervisor, fewer than six physical cores, fewer than four distinct cores for
CPUs `1-4`, less than 16 GiB RAM, non-NVMe storage, cgroup v1, or a CPU quota, stop. Do not register
it as `hydracache-perf-v1`.

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

grep --quiet '^github-runner:' /etc/subuid
grep --quiet '^github-runner:' /etc/subgid
sudo loginctl enable-linger github-runner
runner_uid="$(id --user github-runner)"
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

Do not add kernel isolation parameters yet. W2 must first attest the real CPU numbering and SMT
topology; an incorrect `isolcpus`, `nohz_full`, IRQ affinity, or SMT setting can make the host less
stable or unreachable.

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



## 11. Provisioned-state audit

Run as `hydracache-admin` while the Actions service is offline. The committed W1 helper performs
the authoritative audit; the commands below are additional human-readable checks:

```bash
set -euo pipefail

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
test "$(awk '{print $1}' /sys/fs/cgroup/cpu.max)" = "max"

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

After qualification is green, keep the exact same host and commit. Dispatch five serialized runs
with `candidate_release=0.67.1` and `performance_0671_mode=bootstrap`. Do not select the old
`run_reference_performance` input. Each successful run uploads a distinct
`bootstrap-sample.json`; a failed or unstable run is retained for diagnosis but does not count.

Download the five successful artifacts before stopping the server, copy only their original
`bootstrap-sample.json` files under unique names, and validate the set locally:

```bash
mkdir -p target/bootstrap-samples
# Copy downloaded receipts as target/bootstrap-samples/<github-run-id>.json.
cargo xtask perf-bootstrap --release 0.67.1 --profile reference-v1 \
  --phase sample-set --samples-dir target/bootstrap-samples
```

Do not delete the host until `bootstrap-sample-set.json` is produced successfully and all five
artifact archives are independently retained. This check rejects mixed fingerprints, contracts,
scenario sets, duplicate run ids, failed runs, and any sample marked as ship evidence.

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
