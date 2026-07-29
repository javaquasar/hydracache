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

has_exact_kernel_argument() {
  local expected="$1"
  local count=0
  local argument
  for argument in $(cat /proc/cmdline); do
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

test -f "$runner_dropin"
test "$(stat --format=%U:%G:%a "$runner_dropin")" = root:root:644
test "$(systemctl show "$runner_unit" --property=CPUAffinity --value)" = "0 5 6 7"
test -f "$docker_dropin"
test "$(stat --format=%U:%G:%a "$docker_dropin")" = root:root:644
test "$(runuser --user "$runner_user" -- env \
  XDG_RUNTIME_DIR="/run/user/${runner_uid}" \
  DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/${runner_uid}/bus" \
  systemctl --user show docker.service --property=CPUAffinity --value)" = "0 5 6 7"

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

irq_files=0
for affinity_path in /proc/irq/[0-9]*/effective_affinity_list; do
  test -f "$affinity_path" || continue
  affinity="$(cat "$affinity_path")"
  test -n "$affinity"
  irq_files=$((irq_files + 1))
  if cpu_list_intersects_measurement "$affinity"; then
    echo "IRQ affinity reaches measurement CPUs: $affinity_path=$affinity" >&2
    exit 1
  fi
done
test "$irq_files" -gt 0

echo "reference CPU isolation verified: measurement=${measurement_cpus} housekeeping=${housekeeping_cpus} smt=off irq=housekeeping-only"
