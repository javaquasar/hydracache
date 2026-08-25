#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'
export LC_ALL=C

phase="${1:-}"
measurement="${MEASUREMENT_AFFINITY-1-4}"
[[ "$phase" =~ ^[a-z0-9][a-z0-9-]*$ ]] || {
  echo "usage: scripts/perf/reference-runtime-irq-guard.sh <phase>" >&2
  exit 2
}

cpu_list_intersects_measurement() {
  local cpu_list="$1"
  local segment first last msegment mfirst mlast
  local -a irq_segments measurement_segments
  IFS=',' read -r -a irq_segments <<<"$cpu_list"
  IFS=',' read -r -a measurement_segments <<<"$measurement"
  for segment in "${irq_segments[@]}"; do
    first="${segment%%-*}"
    last="${segment##*-}"
    [[ "$first" =~ ^[0-9]+$ ]]
    [[ "$last" =~ ^[0-9]+$ ]]
    for msegment in "${measurement_segments[@]}"; do
      mfirst="${msegment%%-*}"
      mlast="${msegment##*-}"
      [[ "$mfirst" =~ ^[0-9]+$ && "$mlast" =~ ^[0-9]+$ ]] || continue
      if ((first <= mlast && last >= mfirst)); then return 0; fi
    done
  done
  return 1
}

test -r /proc/interrupts
irq_files=0
for affinity_path in /proc/irq/[0-9]*/effective_affinity_list; do
  test -f "$affinity_path" || continue
  affinity="$(cat "$affinity_path")"
  test -n "$affinity" || continue
  irq_files=$((irq_files + 1))
  if cpu_list_intersects_measurement "$affinity"; then
    irq="${affinity_path#/proc/irq/}"
    irq="${irq%%/*}"
    [[ "$irq" =~ ^[0-9]+$ ]]
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

echo "reference runtime IRQ guard passed: phase=${phase} measurement=${measurement} irq_files=${irq_files}"
