#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'
export LC_ALL=C

action="${1:-}"
case "$action" in
  install|verify) ;;
  *)
    echo "usage: scripts/perf/provision-reference-isolation.sh <install|verify>" >&2
    exit 2
    ;;
esac

test "$(id --user)" -eq 0 || {
  echo "reference CPU isolation provisioning requires root" >&2
  exit 1
}

measurement_cpus="1-4"
housekeeping_cpus="0,5-7"
isolcpus_argument="domain,managed_irq,nohz,1-4"
runner_unit="actions.runner.javaquasar-hydracache.hydracache-perf-v1.service"
runner_dropin="/etc/systemd/system/${runner_unit}.d/20-hydracache-housekeeping.conf"
runner_user="github-runner"
runner_uid="$(id --user "$runner_user")"
docker_dropin="/etc/systemd/user/docker.service.d/20-hydracache-housekeeping.conf"
grub_dropin="/etc/default/grub.d/60-hydracache-perf-isolation.cfg"

write_root_file() {
  local path="$1"
  local mode="$2"
  local temporary
  temporary="$(mktemp)"
  cat >"$temporary"
  install --owner=root --group=root --mode="$mode" "$temporary" "$path"
  rm --force "$temporary"
}

if test "$action" = install; then
  # Before `nosmt` takes effect, prove the host-specific sibling topology that
  # this policy is about to remove from the online CPU set.
  for cpu in 1 2 3 4; do
    sibling=$((cpu + 8))
    test "$(cat "/sys/devices/system/cpu/cpu${cpu}/topology/thread_siblings_list")" = "${cpu},${sibling}"
  done

  test "$(systemctl is-active "$runner_unit" || true)" = inactive
  test "$(runuser --user "$runner_user" -- env \
    XDG_RUNTIME_DIR="/run/user/${runner_uid}" \
    DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/${runner_uid}/bus" \
    systemctl --user is-active docker.service || true)" = inactive

  install --directory --owner=root --group=root --mode=0755 /etc/default/grub.d
  write_root_file "$grub_dropin" 0644 <<EOF
GRUB_CMDLINE_LINUX_DEFAULT="\${GRUB_CMDLINE_LINUX_DEFAULT} nosmt isolcpus=${isolcpus_argument} nohz_full=${measurement_cpus} rcu_nocbs=${measurement_cpus} irqaffinity=${housekeeping_cpus}"
EOF

  install --directory --owner=root --group=root --mode=0755 "$(dirname "$runner_dropin")"
  write_root_file "$runner_dropin" 0644 <<'EOF'
[Service]
CPUAffinity=0 5 6 7
EOF

  install --directory --owner=root --group=root --mode=0755 "$(dirname "$docker_dropin")"
  write_root_file "$docker_dropin" 0644 <<'EOF'
[Service]
CPUAffinity=0 5 6 7
EOF

  systemctl daemon-reload
  runuser --user "$runner_user" -- env \
    XDG_RUNTIME_DIR="/run/user/${runner_uid}" \
    DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/${runner_uid}/bus" \
    systemctl --user daemon-reload
  update-grub
  echo "reference CPU isolation installed; reboot is required before verify"
  exit 0
fi

normalize_cpu_list() {
  local raw="${1// /,}"
  local segment first last cpu
  local old_ifs="$IFS"
  local -a segments
  IFS=',' read -r -a segments <<<"$raw"
  IFS="$old_ifs"
  for segment in "${segments[@]}"; do
    test -n "$segment"
    if [[ "$segment" =~ ^([0-9]+)-([0-9]+)$ ]]; then
      first="${BASH_REMATCH[1]}"
      last="${BASH_REMATCH[2]}"
    elif [[ "$segment" =~ ^[0-9]+$ ]]; then
      first="$segment"
      last="$segment"
    else
      return 1
    fi
    test "$first" -le "$last"
    for ((cpu = first; cpu <= last; cpu++)); do
      printf '%s\n' "$cpu"
    done
  done |
    sort --numeric-sort --unique |
    awk 'BEGIN { separator = "" } { printf "%s%s", separator, $1; separator = "," } END { print "" }'
}

has_exact_kernel_argument() {
  local expected="$1"
  local count=0
  local argument
  local -a kernel_arguments
  IFS=' ' read -r -a kernel_arguments </proc/cmdline
  for argument in "${kernel_arguments[@]}"; do
    if test "$argument" = "$expected"; then
      count=$((count + 1))
    fi
  done
  test "$count" -eq 1
}

has_exact_kernel_argument nosmt
has_exact_kernel_argument "isolcpus=${isolcpus_argument}"
has_exact_kernel_argument "nohz_full=${measurement_cpus}"
has_exact_kernel_argument "rcu_nocbs=${measurement_cpus}"
has_exact_kernel_argument "irqaffinity=${housekeeping_cpus}"

test "$(cat /sys/devices/system/cpu/smt/control)" = off
test "$(cat /sys/devices/system/cpu/online)" = 0-7
test "$(cat /sys/devices/system/cpu/isolated)" = "$measurement_cpus"
test "$(cat /sys/devices/system/cpu/nohz_full)" = "$measurement_cpus"
for cpu in 1 2 3 4; do
  test "$(cat "/sys/devices/system/cpu/cpu${cpu}/topology/thread_siblings_list")" = "$cpu"
done
for sibling in 9 10 11 12; do
  if test -d "/sys/devices/system/cpu/cpu${sibling}"; then
    test -f "/sys/devices/system/cpu/cpu${sibling}/online"
    test "$(cat "/sys/devices/system/cpu/cpu${sibling}/online")" = 0
  fi
done

expected_housekeeping_cpus="$(normalize_cpu_list "$housekeeping_cpus")"
test "$expected_housekeeping_cpus" = 0,5,6,7
test -f "$runner_dropin"
test "$(stat --format=%U:%G:%a "$runner_dropin")" = root:root:644
test "$(normalize_cpu_list "$(systemctl show "$runner_unit" --property=CPUAffinity --value)")" = "$expected_housekeeping_cpus"
test -f "$docker_dropin"
test "$(stat --format=%U:%G:%a "$docker_dropin")" = root:root:644
docker_cpu_affinity="$(runuser --user "$runner_user" -- env \
  XDG_RUNTIME_DIR="/run/user/${runner_uid}" \
  DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/${runner_uid}/bus" \
  systemctl --user show docker.service --property=CPUAffinity --value)"
test "$(normalize_cpu_list "$docker_cpu_affinity")" = "$expected_housekeeping_cpus"

cpu_list_intersects_measurement() {
  local cpu_list="$1"
  local segment first last
  local old_ifs="$IFS"
  IFS=','
  for segment in $cpu_list; do
    first="${segment%%-*}"
    last="${segment##*-}"
    [[ "$first" =~ ^[0-9]+$ ]]
    [[ "$last" =~ ^[0-9]+$ ]]
    if ((first <= 4 && last >= 1)); then
      IFS="$old_ifs"
      return 0
    fi
  done
  IFS="$old_ifs"
  return 1
}

dormant_unmapped_nvme_irq() {
  local irq="$1"
  local action controller queue mq_index interrupt_total cpu_list_path
  local -a mq_cpu_lists
  action="$(
    awk -v irq="${irq}:" '
      $1 == irq { print $NF; found = 1 }
      END { if (!found) exit 1 }
    ' /proc/interrupts
  )"
  [[ "$action" =~ ^nvme([0-9]+)q([1-9][0-9]*)$ ]] || return 1
  controller="${BASH_REMATCH[1]}"
  queue="${BASH_REMATCH[2]}"
  mq_index=$((queue - 1))
  mq_cpu_lists=(/sys/block/nvme${controller}n*/mq/${mq_index}/cpu_list)
  test "${#mq_cpu_lists[@]}" -gt 0
  test -e "${mq_cpu_lists[0]}"
  for cpu_list_path in "${mq_cpu_lists[@]}"; do
    test -z "$(cat "$cpu_list_path")"
  done
  interrupt_total="$(
    awk -v irq="${irq}:" '
      $1 == irq {
        total = 0
        for (field = 2; field <= NF && $field ~ /^[0-9]+$/; field++) {
          total += $field
        }
        print total
        found = 1
      }
      END { if (!found) exit 1 }
    ' /proc/interrupts
  )"
  test "$interrupt_total" = 0
}

irq_files=0
dormant_nvme_irqs=0
for affinity_path in /proc/irq/[0-9]*/effective_affinity_list; do
  test -f "$affinity_path" || continue
  affinity="$(cat "$affinity_path")"
  test -n "$affinity" || continue
  irq_files=$((irq_files + 1))
  if cpu_list_intersects_measurement "$affinity"; then
    irq="${affinity_path#/proc/irq/}"
    irq="${irq%%/*}"
    [[ "$irq" =~ ^[0-9]+$ ]]
    if dormant_unmapped_nvme_irq "$irq"; then
      dormant_nvme_irqs=$((dormant_nvme_irqs + 1))
      continue
    fi
    echo "IRQ affinity reaches measurement CPUs: $affinity_path=$affinity" >&2
    exit 1
  fi
done
test "$irq_files" -gt 0

echo "reference CPU isolation verified: measurement=${measurement_cpus} housekeeping=${housekeeping_cpus} smt=off irq=housekeeping-only dormant-unmapped-nvme=${dormant_nvme_irqs}"
