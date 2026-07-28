#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

usage() {
  echo "usage: scripts/perf/audit-reference-host.sh --mode provisioned" >&2
}

mode=""
while (($# > 0)); do
  case "$1" in
    --mode)
      shift
      (($# > 0)) || {
        usage
        exit 2
      }
      mode="$1"
      ;;
    *)
      usage
      exit 2
      ;;
  esac
  shift
done

test "$mode" = "provisioned" || {
  usage
  exit 2
}

for tool in awk git grep jq lscpu lsblk sort stat systemctl systemd-detect-virt taskset wc; do
  command -v "$tool" >/dev/null || {
    echo "missing required host-audit tool: $tool" >&2
    exit 1
  }
done

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
test -z "$(git status --porcelain=v1 --untracked-files=normal)" || {
  echo "host audit requires a clean worktree" >&2
  exit 1
}

test "$(uname -s)" = "Linux"
test "$(uname -m)" = "x86_64"
. /etc/os-release
test "$ID" = "ubuntu"
test "$VERSION_ID" = "24.04"

if systemd-detect-virt --quiet; then
  echo "virtualization detected: $(systemd-detect-virt)" >&2
  exit 1
fi

test "$(stat --file-system --format=%T /sys/fs/cgroup)" = "cgroup2fs"
read -r cpu_quota cpu_period extra </sys/fs/cgroup/cpu.max
test -z "${extra:-}"
test "$cpu_quota" = "max"
test "$cpu_period" -gt 0

physical_cores="$(
  lscpu --parse=SOCKET,CORE |
    grep --invert-match '^#' |
    sort --unique |
    wc --lines
)"
test "$physical_cores" -ge 6

measurement_topology="$(
  for cpu in 1 2 3 4; do
    test -d "/sys/devices/system/cpu/cpu${cpu}"
    package="$(cat "/sys/devices/system/cpu/cpu${cpu}/topology/physical_package_id")"
    core="$(cat "/sys/devices/system/cpu/cpu${cpu}/topology/core_id")"
    printf '%s:%s\n' "$package" "$core"
  done
)"
distinct_measurement_cores="$(printf '%s\n' "$measurement_topology" | sort --unique | wc --lines)"
test "$distinct_measurement_cores" -eq 4

taskset --cpu-list 1-4 sh -eu -c '
  affinity="$(awk "/^Cpus_allowed_list:/ {print \$2}" /proc/self/status)"
  test "$affinity" = "1-4"
'

memory_bytes="$(
  awk '/^MemTotal:/ {
    value = $2 * 1024
    printf "%.0f\n", value
  }' /proc/meminfo
)"
test "$memory_bytes" -ge 17179869184

nvme_devices="$(
  lsblk --nodeps --noheadings --output NAME,TRAN |
    awk '$2 == "nvme" {print $1}'
)"
test -n "$nvme_devices"

governors="$(
  grep --no-filename . /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor |
    sort --unique
)"
test "$governors" = "performance"

turbo_policy="unavailable"
if test -r /sys/devices/system/cpu/cpufreq/boost; then
  case "$(cat /sys/devices/system/cpu/cpufreq/boost)" in
    1) turbo_policy="enabled" ;;
    0) turbo_policy="disabled" ;;
    *) echo "malformed cpufreq boost policy" >&2; exit 1 ;;
  esac
elif test -r /sys/devices/system/cpu/intel_pstate/no_turbo; then
  case "$(cat /sys/devices/system/cpu/intel_pstate/no_turbo)" in
    0) turbo_policy="enabled" ;;
    1) turbo_policy="disabled" ;;
    *) echo "malformed intel_pstate turbo policy" >&2; exit 1 ;;
  esac
fi
test "$turbo_policy" != "unavailable"

for unit in apt-daily.timer apt-daily-upgrade.timer unattended-upgrades.service; do
  if systemctl is-active --quiet "$unit"; then
    echo "background package activity is active: $unit" >&2
    exit 1
  fi
done

id github-runner >/dev/null
runner_groups="$(id --groups --name github-runner)"
for forbidden_group in sudo docker lxd; do
  if printf '%s\n' "$runner_groups" | tr ' ' '\n' | grep --quiet --word-regexp "$forbidden_group"; then
    echo "github-runner belongs to forbidden group: $forbidden_group" >&2
    exit 1
  fi
done

contract_path="/etc/hydracache-perf/runner-contract.json"
test -r "$contract_path"
jq --exit-status '
  .schema_version == 1 and
  .repository == "javaquasar/hydracache" and
  .runner_name == "hydracache-perf-v1" and
  .labels == ["self-hosted", "linux", "x64", "hydracache-perf-v1"] and
  .service_user == "github-runner"
' "$contract_path" >/dev/null

mapfile -t runner_units < <(
  systemctl list-unit-files --type=service --no-legend 'actions.runner.javaquasar-hydracache.*.service' |
    awk '{print $1}'
)
test "${#runner_units[@]}" -eq 1
runner_unit="${runner_units[0]}"
if systemctl is-active --quiet "$runner_unit"; then
  echo "runner service must be offline during provisioned-state audit: $runner_unit" >&2
  exit 1
fi

output="$repo_root/target/test-evidence/0.67.1/runner-provisioned.json"
mkdir --parents "$(dirname "$output")"
temporary="$(mktemp "${output}.tmp.XXXXXX")"
trap 'rm -f "$temporary"' EXIT

topology_json="$(
  printf '%s\n' "$measurement_topology" |
    jq --raw-input --slurp 'split("\n") | map(select(length > 0))'
)"
nvme_json="$(
  printf '%s\n' "$nvme_devices" |
    jq --raw-input --slurp 'split("\n") | map(select(length > 0))'
)"
commit="$(git rev-parse HEAD)"
kernel="$(uname -r)"

jq --null-input \
  --arg commit "$commit" \
  --arg kernel "$kernel" \
  --arg os_image "$ID-$VERSION_ID" \
  --arg governor "$governors" \
  --arg turbo "$turbo_policy" \
  --arg runner_unit "$runner_unit" \
  --argjson physical_cores "$physical_cores" \
  --argjson memory_bytes "$memory_bytes" \
  --argjson measurement_topology "$topology_json" \
  --argjson nvme_devices "$nvme_json" \
  '{
    schema_version: 1,
    release: "0.67.1",
    stage: "runner-provisioned",
    source_commit: $commit,
    platform: "linux-x86_64",
    os_image: $os_image,
    kernel: $kernel,
    virtualization: "none",
    physical_cores: $physical_cores,
    memory_bytes: $memory_bytes,
    measurement_cpuset: "1-4",
    measurement_topology: $measurement_topology,
    storage_transport: "nvme",
    storage_devices: $nvme_devices,
    cgroup_version: 2,
    cgroup_cpu_quota: "unlimited",
    governor: $governor,
    turbo_policy: $turbo,
    runner_name: "hydracache-perf-v1",
    runner_service: $runner_unit,
    runner_online: false,
    ship_evidence_eligible: false
  }' >"$temporary"

jq --exit-status '.ship_evidence_eligible == false' "$temporary" >/dev/null
chmod 0644 "$temporary"
mv "$temporary" "$output"
trap - EXIT
printf 'runner provisioning audit passed: %s\n' "$output"
