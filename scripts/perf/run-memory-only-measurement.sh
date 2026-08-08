#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'
export LC_ALL=C

usage() {
  cat >&2 <<'EOF'
usage: scripts/perf/run-memory-only-measurement.sh \
  --run-id ID [--mode local|client-surface|all] \
  [--profile PATH] [--runtime-root ABSOLUTE_PATH] \
  [--materialized-root ABSOLUTE_PATH]

Stages the exact HydraCache loadgen/server binaries in tmpfs, performs non-evidence
warm-up, runs guarded memory-only smoke windows, and materializes immutable
non-ship results only after every selected window passes.
EOF
  exit 2
}

repo_root="$(git rev-parse --show-toplevel)"
profile="$repo_root/docs/testing/perf-host-profiles/ubuntu-24.04-memory-only-v1.json"
runtime_root="/dev/shm/hydracache-memory-only-v1"
materialized_parent="$repo_root/target/test-evidence/0.67.1/memory-only"
run_id=""
mode="all"
while test "$#" -gt 0; do
  case "$1" in
    --run-id) test "$#" -ge 2 || usage; run_id="$2"; shift 2 ;;
    --mode) test "$#" -ge 2 || usage; mode="$2"; shift 2 ;;
    --profile) test "$#" -ge 2 || usage; profile="$2"; shift 2 ;;
    --runtime-root) test "$#" -ge 2 || usage; runtime_root="$2"; shift 2 ;;
    --materialized-root) test "$#" -ge 2 || usage; materialized_parent="$2"; shift 2 ;;
    *) usage ;;
  esac
done

[[ "$run_id" =~ ^[a-z0-9][a-z0-9-]{0,62}$ ]] || {
  echo "memory-only run id must match [a-z0-9][a-z0-9-]{0,62}" >&2
  exit 1
}
case "$mode" in local|client-surface|all) ;; *) usage ;; esac
case "$runtime_root" in /*) ;; *) echo "runtime root must be absolute" >&2; exit 1 ;; esac
case "$materialized_parent" in /*) ;; *) echo "materialized root must be absolute" >&2; exit 1 ;; esac

testing=false
if test "${HC_LOCAL_ORCHESTRATION_TESTING:-}" = 1 && test -f /.dockerenv; then
  testing=true
fi
if test "$testing" = false; then
  test "$(id --user --name)" = github-runner || {
    echo "memory-only measurements must run as github-runner" >&2
    exit 1
  }
  test "$({ awk -F: '$1 == "Cpus_allowed_list" { gsub(/[[:space:]]/, "", $2); print $2 }' /proc/self/status; })" = "0,5-7" || {
    echo "memory-only orchestration must remain on housekeeping CPUs 0,5-7" >&2
    exit 1
  }
fi

for tool in cp find findmnt git jq mkdir mv readlink rm sha256sum taskset; do
  command -v "$tool" >/dev/null || {
    echo "missing memory-only orchestration tool: $tool" >&2
    exit 1
  }
done
test -f "$profile"
test "$(jq --exit-status --raw-output '.profile_id' "$profile")" = \
  "$([ "$testing" = true ] && printf '%s' ubuntu-24.04-memory-only-local-test || printf '%s' ubuntu-24.04-memory-only-v1)"
test "$(jq --exit-status --raw-output '.measurement_window_contract.mode' "$profile")" = memory-only-v1

loadgen_source="$repo_root/target/release/hydracache-loadgen"
server_source="$repo_root/target/release/hydracache-server"
if test "$testing" = true; then
  loadgen_source="${HC_MEMORY_ONLY_LOADGEN_SOURCE:-$loadgen_source}"
  server_source="${HC_MEMORY_ONLY_SERVER_SOURCE:-$server_source}"
fi
loadgen_source="$(readlink --canonicalize "$loadgen_source")"
server_source="$(readlink --canonicalize "$server_source")"
test -x "$loadgen_source"
test -x "$server_source"

destination="$materialized_parent/$run_id"
staging_destination="${destination}.materializing"
test ! -e "$runtime_root" || {
  echo "refusing to overwrite memory-only runtime root: $runtime_root" >&2
  exit 1
}
test ! -e "$destination" && test ! -e "$staging_destination" || {
  echo "refusing to overwrite materialized memory-only run: $destination" >&2
  exit 1
}
mkdir --parents "$runtime_root/bin" "$runtime_root/run" "$runtime_root/warmup" \
  "$runtime_root/results" "$materialized_parent"
if test "$testing" = false; then
  test "$(findmnt --noheadings --output FSTYPE --target "$materialized_parent" | xargs)" != tmpfs || {
    echo "materialized memory-only root must be on durable non-tmpfs storage" >&2
    exit 1
  }
fi
cp --preserve=mode,timestamps -- "$loadgen_source" "$runtime_root/bin/hydracache-loadgen"
cp --preserve=mode,timestamps -- "$server_source" "$runtime_root/bin/hydracache-server"
chmod 0555 "$runtime_root/bin/hydracache-loadgen" "$runtime_root/bin/hydracache-server"

measurement_cpus="$(jq --exit-status --raw-output '.cpu_contract.measurement_cpus' "$profile")"
source_commit="$(git -C "$repo_root" rev-parse HEAD)"
profile_sha256="$(sha256sum "$profile" | awk '{print $1}')"
loadgen_sha256="$(sha256sum "$runtime_root/bin/hydracache-loadgen" | awk '{print $1}')"
server_sha256="$(sha256sum "$runtime_root/bin/hydracache-server" | awk '{print $1}')"

declare -a modes=()
case "$mode" in
  local) modes=(local) ;;
  client-surface) modes=(client-surface) ;;
  all) modes=(local client-surface) ;;
esac

for selected_mode in "${modes[@]}"; do
  warmup_report="$runtime_root/warmup/${selected_mode}.json"
  taskset --cpu-list "$measurement_cpus" \
    "$runtime_root/bin/hydracache-loadgen" tier "$selected_mode" \
      --profile smoke-v1 --report "$warmup_report"
  test -s "$warmup_report"
done

for selected_mode in "${modes[@]}"; do
  window_dir="$runtime_root/results/$selected_mode"
  report="$window_dir/report.json"
  guard_env=()
  if test "$testing" = true; then
    guard_env=(
      HC_LOCAL_ORCHESTRATION_TESTING=1
      "HC_MEMORY_ONLY_PROC_ROOT=${HC_MEMORY_ONLY_PROC_ROOT:?}"
      "HC_MEMORY_ONLY_CGROUP_ROOT=${HC_MEMORY_ONLY_CGROUP_ROOT:?}"
    )
  fi
  env "${guard_env[@]}" \
    "$repo_root/scripts/perf/reference-memory-only-window.py" \
      --profile "$profile" \
      --runtime-root "$runtime_root" \
      --working-directory "$runtime_root/run" \
      --output-dir "$window_dir" \
      -- "$runtime_root/bin/hydracache-loadgen" tier "$selected_mode" \
        --profile smoke-v1 --report "$report"
  jq --exit-status '.passed == true and .ship_evidence_eligible == false' \
    "$window_dir/memory-only-window.json" >/dev/null
  test -s "$report"
done

windows_json="$({
  for selected_mode in "${modes[@]}"; do
    receipt="$runtime_root/results/$selected_mode/memory-only-window.json"
    report="$runtime_root/results/$selected_mode/report.json"
    jq --null-input \
      --arg mode "$selected_mode" \
      --arg receipt_sha256 "$(sha256sum "$receipt" | awk '{print $1}')" \
      --arg report_sha256 "$(sha256sum "$report" | awk '{print $1}')" '
        {mode: $mode, receipt_sha256: $receipt_sha256, report_sha256: $report_sha256}
      '
  done
} | jq --slurp '.')"
jq --null-input \
  --arg run_id "$run_id" \
  --arg source_commit "$source_commit" \
  --arg profile_sha256 "$profile_sha256" \
  --arg loadgen_sha256 "$loadgen_sha256" \
  --arg server_sha256 "$server_sha256" \
  --arg measurement_cpus "$measurement_cpus" \
  --argjson windows "$windows_json" '
    {
      schema_version: 1,
      stage: "reference-memory-only-run",
      run_id: $run_id,
      source_commit: $source_commit,
      profile_sha256: $profile_sha256,
      measurement_cpus: $measurement_cpus,
      binaries: {
        "hydracache-loadgen": $loadgen_sha256,
        "hydracache-server": $server_sha256
      },
      windows: $windows,
      passed: true,
      qualification_evidence: false,
      bootstrap_evidence: false,
      ship_evidence_eligible: false
    }
  ' >"$runtime_root/results/memory-only-run.json"

mkdir --parents "$staging_destination"
cp --archive -- "$runtime_root/results/." "$staging_destination/"
find "$staging_destination" -type f -exec chmod 0444 {} +
find "$staging_destination" -type d -exec chmod 0555 {} +
mv -- "$staging_destination" "$destination"
rm --recursive --force -- "$runtime_root"
echo "MEMORY_ONLY_RUN_PASSED=true results=$destination"
