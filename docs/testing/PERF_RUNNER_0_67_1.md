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

Keep time synchronization and SSH enabled. Do not install Docker, Kubernetes, monitoring agents, or
unrelated workloads on this host.

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
From a clean HydraCache checkout, verify the registered service and deliberately leave it offline:

```bash
scripts/perf/verify-runner-service.sh --expected-label hydracache-perf-v1
scripts/perf/runner-service.sh offline
scripts/perf/audit-reference-host.sh --mode provisioned
```

The audit writes `target/test-evidence/0.67.1/runner-provisioned.json`. It intentionally records
`ship_evidence_eligible=false`; provisioning is not performance evidence.



## 11. Provisioned-state audit

Run as `hydracache-admin`. This is a manual W1 audit until
`scripts/perf/audit-reference-host.sh` lands:

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

Before every authorized run:

1. Confirm the selected GitHub ref is clean trusted `main`.
2. Confirm no apt activity, maintenance, or other workload is running.
3. Run the committed W2 host audit and W3 qualification command.
4. Start the runner service.
5. Manually dispatch only the explicit 0.67.1 qualification/bootstrap/reference mode.
6. Monitor the job and archive its complete artifact even on failure.
7. Stop the service immediately after the job reaches a terminal state.

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
