#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'
export LC_ALL=C

usage() {
  cat >&2 <<'EOF'
usage: scripts/perf/reference-host-irq-burn-in.sh \
  --output-dir ABSOLUTE_PATH \
  [--profile PATH] [--duration-seconds 900] [--read-mebibytes 256] \
  [--network-target IPV4]

This is a destructive-to-time but read-only-to-storage host admission probe. It
runs outside measured evidence, deliberately stimulates NVMe and network queues
from every reviewed measurement CPU, then requires zero IRQ activity or mapping
change on those CPUs through the following idle window. It never writes to a
block device and it never relaxes the release IRQ guards.
EOF
  exit 2
}

repo_root="$(git rev-parse --show-toplevel)"
profile="$repo_root/docs/testing/perf-host-profiles/ubuntu-24.04-reference-v1.json"
output_dir=""
duration_seconds=900
read_mebibytes=256
network_target=""
while test "$#" -gt 0; do
  case "$1" in
    --output-dir)
      test "$#" -ge 2 || usage
      output_dir="$2"
      shift 2
      ;;
    --profile)
      test "$#" -ge 2 || usage
      profile="$2"
      shift 2
      ;;
    --duration-seconds)
      test "$#" -ge 2 || usage
      duration_seconds="$2"
      shift 2
      ;;
    --read-mebibytes)
      test "$#" -ge 2 || usage
      read_mebibytes="$2"
      shift 2
      ;;
    --network-target)
      test "$#" -ge 2 || usage
      network_target="$2"
      shift 2
      ;;
    *) usage ;;
  esac
done

test "$(id --user)" -eq 0 || {
  echo "reference IRQ burn-in requires root" >&2
  exit 1
}
case "$output_dir" in
  /*) ;;
  *) echo "IRQ burn-in output directory must be absolute" >&2; exit 1 ;;
esac
[[ "$duration_seconds" =~ ^[0-9]+$ ]] &&
  test "$duration_seconds" -ge 600 && test "$duration_seconds" -le 3600 || {
  echo "IRQ burn-in duration must be between 600 and 3600 seconds" >&2
  exit 1
}
[[ "$read_mebibytes" =~ ^[0-9]+$ ]] &&
  test "$read_mebibytes" -ge 64 && test "$read_mebibytes" -le 1024 || {
  echo "IRQ burn-in read size must be between 64 and 1024 MiB per CPU/device" >&2
  exit 1
}
test -f "$profile"
test ! -e "$output_dir" || {
  echo "IRQ burn-in refuses to overwrite output: $output_dir" >&2
  exit 1
}

for tool in awk cat cp date dd git id ip jq lsblk ping sha256sum stat systemctl taskset tee; do
  command -v "$tool" >/dev/null || {
    echo "missing IRQ burn-in tool: $tool" >&2
    exit 1
  }
done

measurement="$(jq --exit-status --raw-output '.cpu_contract.measurement_cpus' "$profile")"
test "$measurement" = "1-4" || {
  echo "IRQ burn-in only supports the reviewed 1-4 measurement contract" >&2
  exit 1
}

if test -z "$network_target"; then
  network_target="$(ip -4 route show default | awk 'NR == 1 { print $3 }')"
fi
[[ "$network_target" =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]] || {
  echo "IRQ burn-in requires an explicit IPv4 target or IPv4 default gateway" >&2
  exit 1
}
IFS='.' read -r -a network_octets <<<"$network_target"
test "${#network_octets[@]}" -eq 4
for octet in "${network_octets[@]}"; do
  test "$octet" -ge 0 && test "$octet" -le 255 || {
    echo "invalid IPv4 network target: $network_target" >&2
    exit 1
  }
done

mapfile -t runner_units < <(
  systemctl list-unit-files --type=service --no-legend \
    'actions.runner.javaquasar-hydracache.*.service' | awk '{print $1}'
)
test "${#runner_units[@]}" -eq 1
test "$(systemctl is-active "${runner_units[0]}" || true)" = inactive || {
  echo "runner must be offline during IRQ burn-in" >&2
  exit 1
}
test ! -S /var/run/docker.sock || {
  echo "rootful Docker socket must remain absent during IRQ burn-in" >&2
  exit 1
}
runner_uid="$(id --user github-runner)"
test ! -S "/run/user/$runner_uid/docker.sock" || {
  echo "rootless Docker socket must remain absent during IRQ burn-in" >&2
  exit 1
}

mapfile -t nvme_devices < <(
  lsblk --nodeps --noheadings --paths --output PATH,TYPE,TRAN |
    awk '$2 == "disk" && $3 == "nvme" { print $1 }'
)
test "${#nvme_devices[@]}" -gt 0 || {
  echo "IRQ burn-in found no NVMe namespace" >&2
  exit 1
}

mkdir --parents --mode=0755 "$output_dir"
log="$output_dir/burn-in.log"
baseline="$output_dir/irq-baseline.tsv"
interrupts_before="$output_dir/interrupts-before.txt"
interrupts_after="$output_dir/interrupts-after.txt"
receipt="$output_dir/irq-burn-in.json"
started_at="$(date --utc --iso-8601=seconds)"
source_commit="$(git -C "$repo_root" rev-parse HEAD)"
profile_sha256="$(sha256sum "$profile" | awk '{print $1}')"
passed=false
failure_step=initialization
finished_at=""

exec > >(tee --append "$log") 2>&1

file_digest_or_null() {
  path="$1"
  if test -f "$path"; then
    sha256sum "$path" | awk '{print $1}'
  else
    printf 'null\n'
  fi
}

write_receipt() {
  exit_status="$?"
  finished_at="$(date --utc --iso-8601=seconds)"
  baseline_sha256="$(file_digest_or_null "$baseline")"
  before_sha256="$(file_digest_or_null "$interrupts_before")"
  after_sha256="$(file_digest_or_null "$interrupts_after")"
  devices_json="$(printf '%s\n' "${nvme_devices[@]}" | jq --raw-input --slurp 'split("\n") | map(select(length > 0))')"
  jq --null-input \
    --arg started_at "$started_at" \
    --arg finished_at "$finished_at" \
    --arg source_commit "$source_commit" \
    --arg profile_sha256 "$profile_sha256" \
    --arg measurement_cpus "$measurement" \
    --arg network_target "$network_target" \
    --arg failure_step "$failure_step" \
    --arg baseline_sha256 "$baseline_sha256" \
    --arg interrupts_before_sha256 "$before_sha256" \
    --arg interrupts_after_sha256 "$after_sha256" \
    --argjson duration_seconds "$duration_seconds" \
    --argjson read_mebibytes "$read_mebibytes" \
    --argjson nvme_devices "$devices_json" \
    --argjson passed "$passed" '
      {
        schema_version: 1,
        stage: "reference-host-irq-burn-in",
        source_commit: $source_commit,
        profile_sha256: $profile_sha256,
        measurement_cpus: $measurement_cpus,
        duration_seconds: $duration_seconds,
        read_mebibytes_per_cpu_device: $read_mebibytes,
        network_target: $network_target,
        nvme_devices: $nvme_devices,
        started_at: $started_at,
        finished_at: $finished_at,
        irq_baseline_sha256: (if $baseline_sha256 == "null" then null else $baseline_sha256 end),
        interrupts_before_sha256: (if $interrupts_before_sha256 == "null" then null else $interrupts_before_sha256 end),
        interrupts_after_sha256: (if $interrupts_after_sha256 == "null" then null else $interrupts_after_sha256 end),
        passed: $passed,
        failure_step: (if $passed then null else $failure_step end),
        qualification_evidence: false,
        bootstrap_evidence: false,
        ship_evidence_eligible: false
      }
    ' >"$receipt"
  chmod 0444 "$receipt"
  if test "$passed" = true; then
    echo "reference IRQ burn-in passed: receipt=$receipt"
  else
    echo "reference IRQ burn-in rejected at $failure_step: receipt=$receipt" >&2
  fi
  return "$exit_status"
}
trap write_receipt EXIT

failure_step=preflight-absolute-irq-guard
"$repo_root/scripts/perf/reference-runtime-irq-guard.sh" burn-in-preflight
cp /proc/interrupts "$interrupts_before"

failure_step=baseline
MEASUREMENT_AFFINITY="$measurement" \
  "$repo_root/scripts/perf/reference-runtime-irq-delta-guard.sh" baseline "$baseline"

failure_step=nvme-stimulus
mapfile -t measurement_cpus < <(seq 1 4)
declare -a stimulus_pids=()
for device in "${nvme_devices[@]}"; do
  cpu_index=0
  for cpu in "${measurement_cpus[@]}"; do
    taskset --cpu-list "$cpu" \
      dd if="$device" of=/dev/null bs=1M count="$read_mebibytes" \
        skip="$((cpu_index * read_mebibytes))" iflag=direct,fullblock status=none &
    stimulus_pids+=("$!")
    cpu_index=$((cpu_index + 1))
  done
done
for stimulus_pid in "${stimulus_pids[@]}"; do
  wait "$stimulus_pid"
done

failure_step=network-stimulus
stimulus_pids=()
for cpu in "${measurement_cpus[@]}"; do
  taskset --cpu-list "$cpu" \
    ping -4 --numeric --quiet --count 32 --interval 0.02 --wait 10 "$network_target" &
  stimulus_pids+=("$!")
done
for stimulus_pid in "${stimulus_pids[@]}"; do
  wait "$stimulus_pid"
done

failure_step=post-stimulus-irq-delta
MEASUREMENT_AFFINITY="$measurement" \
  "$repo_root/scripts/perf/reference-runtime-irq-delta-guard.sh" post-stimulus "$baseline"

failure_step=idle-window
elapsed=0
while test "$elapsed" -lt "$duration_seconds"; do
  remaining=$((duration_seconds - elapsed))
  chunk=60
  test "$remaining" -ge "$chunk" || chunk="$remaining"
  sleep "$chunk"
  elapsed=$((elapsed + chunk))
  echo "reference IRQ burn-in idle progress: ${elapsed}/${duration_seconds}s"
done

failure_step=post-idle-absolute-irq-guard
"$repo_root/scripts/perf/reference-runtime-irq-guard.sh" burn-in-post-idle
failure_step=post-idle-irq-delta
MEASUREMENT_AFFINITY="$measurement" \
  "$repo_root/scripts/perf/reference-runtime-irq-delta-guard.sh" post-idle "$baseline"
cp /proc/interrupts "$interrupts_after"

failure_step=complete
passed=true
