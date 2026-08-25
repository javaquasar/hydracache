#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'
export LC_ALL=C

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

for tool in awk git grep jq loginctl lscpu lsblk sha256sum sort stat sudo systemctl systemd-detect-virt taskset wc; do
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
cgroup_path="$(awk -F: '$1 == "0" && $2 == "" {print $3}' /proc/self/cgroup)"
test -n "$cgroup_path"
case "$cgroup_path" in
  /*) ;;
  *) echo "cgroup v2 process path is not absolute: $cgroup_path" >&2; exit 1 ;;
esac
case "/$cgroup_path/" in
  */../*) echo "cgroup v2 process path contains parent traversal" >&2; exit 1 ;;
esac
cgroup_cursor="/sys/fs/cgroup${cgroup_path%/}"
cpu_controller_observed=false
while :; do
  if test -f "$cgroup_cursor/cpu.max"; then
    IFS=' ' read -r cpu_quota cpu_period extra <"$cgroup_cursor/cpu.max"
    test -z "${extra:-}"
    test "$cpu_period" -gt 0
    test "$cpu_quota" = "max" || {
      echo "cgroup CPU quota detected at $cgroup_cursor: $cpu_quota/$cpu_period" >&2
      exit 1
    }
    cpu_controller_observed=true
  fi
  test "$cgroup_cursor" != "/sys/fs/cgroup" || break
  cgroup_cursor="${cgroup_cursor%/*}"
  case "$cgroup_cursor" in
    /sys/fs/cgroup|/sys/fs/cgroup/*) ;;
    *) echo "cgroup v2 ancestor escaped the unified hierarchy" >&2; exit 1 ;;
  esac
done
test "$cpu_controller_observed" = true

mapfile -t host_identity_values < <(
  for identity_path in \
    /sys/class/dmi/id/product_uuid \
    /sys/class/dmi/id/board_serial \
    /sys/class/dmi/id/product_serial; do
    if sudo test -r "$identity_path"; then
      identity_value="$(sudo cat -- "$identity_path")"
      identity_value="${identity_value#"${identity_value%%[![:space:]]*}"}"
      identity_value="${identity_value%"${identity_value##*[![:space:]]}"}"
      case "${identity_value,,}" in
        ""|none|unknown|"not specified"|"to be filled by o.e.m.") ;;
        *) printf '%s\n' "$identity_value" ;;
      esac
    fi
  done | sort --unique
)
test "${#host_identity_values[@]}" -gt 0 || {
  echo "no usable root-readable DMI identity inputs" >&2
  exit 1
}
host_identity_digest="$(
  {
    printf '%s' 'hydracache-host-identity-v2'
    for identity_value in "${host_identity_values[@]}"; do
      printf '\0%s' "$identity_value"
    done
  } | sha256sum | awk '{print $1}'
)"
[[ "$host_identity_digest" =~ ^[0-9a-f]{64}$ ]]

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

sudo scripts/perf/provision-reference-isolation.sh verify

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
turbo_policy_backend="unavailable"
if test -r /sys/devices/system/cpu/cpufreq/boost; then
  case "$(cat /sys/devices/system/cpu/cpufreq/boost)" in
    1) turbo_policy="enabled"; turbo_policy_backend="cpufreq-global-boost-v1" ;;
    0) turbo_policy="disabled"; turbo_policy_backend="cpufreq-global-boost-v1" ;;
    *) echo "malformed cpufreq boost policy" >&2; exit 1 ;;
  esac
elif test -r /sys/devices/system/cpu/intel_pstate/no_turbo; then
  case "$(cat /sys/devices/system/cpu/intel_pstate/no_turbo)" in
    0) turbo_policy="enabled"; turbo_policy_backend="intel-pstate-no-turbo-v1" ;;
    1) turbo_policy="disabled"; turbo_policy_backend="intel-pstate-no-turbo-v1" ;;
    *) echo "malformed intel_pstate turbo policy" >&2; exit 1 ;;
  esac
elif test "$(cat /sys/devices/system/cpu/amd_pstate/status 2>/dev/null || true)" = active; then
  # Linux documents that amd-pstate reports a lower cpuinfo maximum when
  # boost is supported but inactive. Require the CPB capability and exact
  # equality of the driver, cpuinfo, and policy maxima for every online
  # policy before recording boost as enabled.
  lscpu | awk -F: '/^Flags:/ { print $2 }' | tr ' ' '\n' |
    grep --quiet --fixed-strings --line-regexp cpb
  amd_policy_count=0
  for policy_path in /sys/devices/system/cpu/cpufreq/policy*; do
    test -d "$policy_path"
    driver="$(cat "$policy_path/scaling_driver")"
    test "$driver" = amd-pstate-epp
    amd_max="$(cat "$policy_path/amd_pstate_max_freq")"
    cpuinfo_max="$(cat "$policy_path/cpuinfo_max_freq")"
    scaling_max="$(cat "$policy_path/scaling_max_freq")"
    [[ "$amd_max" =~ ^[1-9][0-9]*$ ]]
    test "$amd_max" = "$cpuinfo_max"
    test "$amd_max" = "$scaling_max"
    amd_policy_count=$((amd_policy_count + 1))
  done
  test "$amd_policy_count" -gt 0
  turbo_policy="enabled"
  turbo_policy_backend="amd-pstate-active-max-frequency-v1"
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

for rootful_unit in docker.service docker.socket containerd.service; do
  if systemctl is-active --quiet "$rootful_unit"; then
    echo "rootful container service must remain inactive: $rootful_unit" >&2
    exit 1
  fi
done
test "$(loginctl show-user github-runner --property=Linger --value)" = "yes"
grep --quiet '^github-runner:' /etc/subuid
grep --quiet '^github-runner:' /etc/subgid
rootless_unit="/home/github-runner/.config/systemd/user/docker.service"
sudo test -f "$rootless_unit"
test "$(sudo stat --format=%U "$rootless_unit")" = "github-runner"

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
  --arg turbo_backend "$turbo_policy_backend" \
  --arg runner_unit "$runner_unit" \
  --arg host_identity_digest "$host_identity_digest" \
  --argjson physical_cores "$physical_cores" \
  --argjson memory_bytes "$memory_bytes" \
  --argjson measurement_topology "$topology_json" \
  --argjson nvme_devices "$nvme_json" \
  '{
    schema_version: 4,
    release: "0.67.1",
    stage: "runner-provisioned",
    source_commit: $commit,
    platform: "linux-x86_64",
    os_image: $os_image,
    kernel: $kernel,
    virtualization: "none",
    host_identity_digest: $host_identity_digest,
    physical_cores: $physical_cores,
    memory_bytes: $memory_bytes,
    measurement_cpuset: "1-4",
    measurement_topology: $measurement_topology,
    cpu_isolation: {
      smt_control: "off",
      online_cpus: "0-7",
      isolated_cpus: "1-4",
      nohz_full_cpus: "1-4",
      rcu_nocbs_cpus: "1-4",
      housekeeping_cpus: "0,5-7",
      irq_affinity_policy: "housekeeping-only-v1",
      measurement_idle_policy: "latency-cap-us-v1",
      measurement_max_idle_latency_us: 1,
      housekeeping_idle_policy: "latency-cap-us-v1",
      housekeeping_max_idle_latency_us: 1
    },
    storage_transport: "nvme",
    storage_devices: $nvme_devices,
    cgroup_version: 2,
    cgroup_cpu_quota: "unlimited",
    governor: $governor,
    turbo_policy: $turbo,
    turbo_policy_backend: $turbo_backend,
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
