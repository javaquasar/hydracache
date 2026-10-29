#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'
export LC_ALL=C

usage() {
  cat >&2 <<'EOF'
usage: scripts/perf/reference-host-irq-layout-preflight.sh \
  --output-dir ABSOLUTE_PATH [--profile PATH]

This early, read-only rental eligibility probe inventories the effective IRQ
layout and applies the unchanged absolute runtime IRQ guard before runner
registration or service tuning. It never writes IRQ affinity and its receipt
is not qualification, bootstrap, or ship evidence.
EOF
  exit 2
}

repo_root="$(git rev-parse --show-toplevel)"
profile="$repo_root/docs/testing/perf-host-profiles/ubuntu-24.04-reference-v1.json"
output_dir=""
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
    *) usage ;;
  esac
done

case "$output_dir" in
  /*) ;;
  *) echo "IRQ layout preflight output directory must be absolute" >&2; exit 1 ;;
esac
test -f "$profile"
test ! -e "$output_dir" || {
  echo "IRQ layout preflight refuses to overwrite output: $output_dir" >&2
  exit 1
}
for tool in awk cat chmod cp date git jq mkdir sha256sum; do
  command -v "$tool" >/dev/null || {
    echo "missing IRQ layout preflight tool: $tool" >&2
    exit 1
  }
done
test -r /proc/interrupts

measurement="$(jq --exit-status --raw-output '.cpu_contract.measurement_cpus' "$profile")"
test -n "$measurement"
mkdir --parents --mode=0755 "$output_dir"
inventory="$output_dir/irq-layout.tsv"
guard_log="$output_dir/runtime-irq-guard.log"
receipt="$output_dir/irq-layout-preflight.json"
interrupts="$output_dir/interrupts.txt"
cp /proc/interrupts "$interrupts"

printf 'irq\teffective_affinity\taction\ttotal_count\n' >"$inventory"
irq_files=0
while IFS= read -r affinity_path; do
  test -f "$affinity_path" || continue
  affinity="$(cat "$affinity_path")"
  test -n "$affinity" || continue
  irq_files=$((irq_files + 1))
  irq="${affinity_path#/proc/irq/}"
  irq="${irq%%/*}"
  action="$(awk -v irq="${irq}:" '$1 == irq { print $NF; found = 1 } END { if (!found) print "unknown" }' /proc/interrupts)"
  total="$(awk -v irq="${irq}:" '$1 == irq { total = 0; for (field = 2; field <= NF && $field ~ /^[0-9]+$/; field++) total += $field; print total; found = 1 } END { if (!found) print 0 }' /proc/interrupts)"
  printf '%s\t%s\t%s\t%s\n' "$irq" "$affinity" "$action" "$total" >>"$inventory"
done < <(printf '%s\n' /proc/irq/[0-9]*/effective_affinity_list)
test "$irq_files" -gt 0

passed=false
if MEASUREMENT_AFFINITY="$measurement" \
  "$repo_root/scripts/perf/reference-runtime-irq-guard.sh" early-layout \
  >"$guard_log" 2>&1; then
  passed=true
fi

jq --null-input \
  --arg source_commit "$(git -C "$repo_root" rev-parse HEAD)" \
  --arg profile_sha256 "$(sha256sum "$profile" | awk '{print $1}')" \
  --arg measurement_cpus "$measurement" \
  --arg captured_at "$(date --utc --iso-8601=seconds)" \
  --arg inventory_sha256 "$(sha256sum "$inventory" | awk '{print $1}')" \
  --arg interrupts_sha256 "$(sha256sum "$interrupts" | awk '{print $1}')" \
  --arg guard_log_sha256 "$(sha256sum "$guard_log" | awk '{print $1}')" \
  --argjson irq_files "$irq_files" \
  --argjson passed "$passed" '
    {
      schema_version: 1,
      stage: "reference-host-early-irq-layout-preflight",
      source_commit: $source_commit,
      profile_sha256: $profile_sha256,
      measurement_cpus: $measurement_cpus,
      captured_at: $captured_at,
      irq_files: $irq_files,
      inventory_sha256: $inventory_sha256,
      interrupts_sha256: $interrupts_sha256,
      guard_log_sha256: $guard_log_sha256,
      passed: $passed,
      mutates_irq_affinity: false,
      qualification_evidence: false,
      bootstrap_evidence: false,
      ship_evidence_eligible: false
    }
  ' >"$receipt"
chmod 0444 "$receipt"

if test "$passed" = true; then
  echo "early IRQ layout preflight passed: receipt=$receipt"
else
  echo "early IRQ layout preflight rejected this rental candidate: receipt=$receipt" >&2
  cat "$guard_log" >&2
  exit 1
fi
