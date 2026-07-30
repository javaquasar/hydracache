#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'
export LC_ALL=C

phase="${1:-}"
[[ "$phase" =~ ^[a-z0-9][a-z0-9-]*$ ]] || {
  echo "usage: scripts/perf/reference-runtime-irq-guard.sh <phase>" >&2
  exit 2
}

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

test -r /proc/interrupts
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
    action="$(
      awk -v irq="${irq}:" '
        $1 == irq { print $NF; found = 1 }
        END { if (!found) print "unknown" }
      ' /proc/interrupts
    )"
    counts="$(
      awk -v irq="${irq}:" '
        $1 == irq {
          separator = ""
          for (field = 2; field <= NF && $field ~ /^[0-9]+$/; field++) {
            printf "%s%s", separator, $field
            separator = ","
          }
          print ""
        }
      ' /proc/interrupts
    )"
    echo "runtime IRQ guard failed phase=${phase}: irq=${irq} action=${action} effective_affinity=${affinity} per_cpu_counts=${counts}" >&2
    exit 1
  fi
done
test "$irq_files" -gt 0

echo "reference runtime IRQ guard passed: phase=${phase} measurement=1-4 irq_files=${irq_files} dormant-unmapped-nvme=${dormant_nvme_irqs}"
