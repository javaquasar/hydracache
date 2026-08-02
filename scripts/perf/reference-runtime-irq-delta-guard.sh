#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'
export LC_ALL=C

phase="${1:-}"
baseline_file="${2:-}"
measurement="${MEASUREMENT_AFFINITY-1-4}"
[[ "$phase" =~ ^[a-z0-9][a-z0-9-]*$ ]] || { echo "usage: $0 <baseline|post-phase> <baseline-file>" >&2; exit 2; }
test -n "$baseline_file"

cpu_list_intersects_measurement() {
  local cpu_list="$1" segment first last msegment mfirst mlast
  local -a irq_segments measurement_segments
  IFS=',' read -r -a irq_segments <<<"$cpu_list"
  IFS=',' read -r -a measurement_segments <<<"$measurement"
  for segment in "${irq_segments[@]}"; do
    first="${segment%%-*}"
    last="${segment##*-}"
    [[ "$first" =~ ^[0-9]+$ && "$last" =~ ^[0-9]+$ ]] || continue
    for msegment in "${measurement_segments[@]}"; do
      mfirst="${msegment%%-*}"
      mlast="${msegment##*-}"
      [[ "$mfirst" =~ ^[0-9]+$ && "$mlast" =~ ^[0-9]+$ ]] || continue
      if ((first <= mlast && last >= mfirst)); then return 0; fi
    done
  done
  return 1
}

irq_action() {
  local irq="$1"
  awk -v irq="${irq}:" '$1 == irq { print $NF; found = 1 } END { if (!found) exit 1 }' /proc/interrupts
}

irq_count() {
  local irq="$1"
  awk -v irq="${irq}:" '$1 == irq { total = 0; for (field = 2; field <= NF && $field ~ /^[0-9]+$/; field++) total += $field; print total; found = 1 } END { if (!found) exit 1 }' /proc/interrupts
}

irq_affinity() { cat "/proc/irq/$1/effective_affinity_list"; }

if [[ "$phase" == baseline ]]; then
  : >"$baseline_file"
  irq_files=0
  while IFS= read -r affinity_path; do
    test -f "$affinity_path" || continue
    affinity="$(cat "$affinity_path")"
    test -n "$affinity" || continue
    irq_files=$((irq_files + 1))
    cpu_list_intersects_measurement "$affinity" || continue
    irq="${affinity_path#/proc/irq/}"
    irq="${irq%%/*}"
    action="$(irq_action "$irq")"
    [[ "$action" =~ ^nvme[0-9]+q[0-9]+$ || "$action" =~ ^ahci\[[^]]+\]$ ]] || {
      echo "runtime IRQ delta guard refused unknown IRQ baseline: phase=$phase irq=$irq action=$action effective_affinity=$affinity" >&2
      exit 1
    }
    printf '%s\t%s\t%s\t%s\n' "$irq" "$affinity" "$action" "$(irq_count "$irq")" >>"$baseline_file"
  done < <(printf '%s\n' /proc/irq/[0-9]*/effective_affinity_list)
  test "$irq_files" -gt 0
  echo "reference runtime IRQ delta baseline captured: phase=$phase measurement=$measurement file=$baseline_file"
  exit 0
fi

test -s "$baseline_file"
declare -A baseline_counts baseline_affinity baseline_actions
while IFS=$'\t' read -r irq affinity action count; do
  baseline_counts["$irq"]="$count"
  baseline_affinity["$irq"]="$affinity"
  baseline_actions["$irq"]="$action"
done <"$baseline_file"

for irq in "${!baseline_counts[@]}"; do
  current_affinity="$(irq_affinity "$irq")"
  test "$current_affinity" = "${baseline_affinity[$irq]}" || {
    echo "runtime IRQ delta guard failed affinity changed: phase=$phase irq=$irq baseline=${baseline_affinity[$irq]} current=$current_affinity" >&2
    exit 1
  }
  current_count="$(irq_count "$irq")"
  test "$current_count" = "${baseline_counts[$irq]}" || {
    echo "runtime IRQ delta guard failed new IRQ activity: phase=$phase irq=$irq action=${baseline_actions[$irq]} baseline_count=${baseline_counts[$irq]} current_count=$current_count" >&2
    exit 1
  }
done

while IFS= read -r affinity_path; do
  test -f "$affinity_path" || continue
  affinity="$(cat "$affinity_path")"
  cpu_list_intersects_measurement "$affinity" || continue
  irq="${affinity_path#/proc/irq/}"
  irq="${irq%%/*}"
  [[ -n "${baseline_counts[$irq]+x}" ]] || {
    echo "runtime IRQ delta guard failed new IRQ mapping: phase=$phase irq=$irq effective_affinity=$affinity" >&2
    exit 1
  }
done < <(printf '%s\n' /proc/irq/[0-9]*/effective_affinity_list)

echo "reference runtime IRQ delta guard passed: phase=$phase measurement=$measurement monitored=${#baseline_counts[@]}"
