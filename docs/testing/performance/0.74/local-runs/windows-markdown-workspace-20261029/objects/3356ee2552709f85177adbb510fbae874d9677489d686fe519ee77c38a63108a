#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

usage() {
  echo "usage: scripts/perf/check-reference-host-freeze.sh [--profile PATH] [--state-dir PATH]" >&2
  exit 2
}

repo_root="$(git rev-parse --show-toplevel)"
profile="$repo_root/docs/testing/perf-host-profiles/ubuntu-24.04-reference-v1.json"
state_dir="/var/lib/hydracache-perf/host-tuning-v1"
while test "$#" -gt 0; do
  case "$1" in
    --profile)
      test "$#" -ge 2 || usage
      profile="$2"
      shift 2
      ;;
    --state-dir)
      test "$#" -ge 2 || usage
      state_dir="$2"
      shift 2
      ;;
    *) usage ;;
  esac
done

test "$(id --user)" -eq 0 || {
  echo "frozen host check requires root" >&2
  exit 1
}
for tool in dpkg-query git grep jq sha256sum sort stat sysctl systemctl uname; do
  command -v "$tool" >/dev/null || {
    echo "missing frozen host check tool: $tool" >&2
    exit 1
  }
done

read_frozen_kernel_tunable() {
  local key="$1"
  local backend locator value kernel_config
  if value="$(sysctl --values "$key" 2>/dev/null)"; then
    backend="sysctl"
    locator="$key"
  else
    case "$key" in
      kernel.sched_migration_cost_ns)
        backend="debugfs"
        locator="/sys/kernel/debug/sched/migration_cost_ns"
        kernel_config="/boot/config-$(uname -r)"
        test -f "$kernel_config"
        grep --quiet --fixed-strings --line-regexp 'CONFIG_SCHED_DEBUG=y' "$kernel_config"
        test "$(stat --file-system --format=%T /sys/kernel/debug)" = debugfs
        test -f "$locator"
        test "$(stat --format=%U:%G "$locator")" = root:root
        value="$(cat "$locator")"
        [[ "$value" =~ ^[0-9]+$ ]]
        ;;
      *)
        echo "required frozen kernel tunable is unavailable: $key" >&2
        exit 1
        ;;
    esac
  fi
  case "$value" in
    *$'\t'*|*$'\n'*)
      echo "frozen kernel tunable has an unsafe value: $key" >&2
      exit 1
      ;;
  esac
  printf '%s\t%s\t%s\t%s\n' "$key" "$backend" "$locator" "$value"
}
receipt="$state_dir/freeze/host-freeze.json"
test -f "$profile" || {
  echo "missing host profile: $profile" >&2
  exit 1
}
test -f "$receipt" || {
  echo "missing frozen host receipt: $receipt" >&2
  exit 1
}
test -z "$(git -C "$repo_root" status --porcelain=v1 --untracked-files=normal)" || {
  echo "frozen host check requires a clean worktree" >&2
  exit 1
}

"$repo_root/scripts/perf/reference-host-tuning.sh" verify \
  --profile "$profile" \
  --state-dir "$state_dir"

compare_value() {
  field="$1"
  actual="$2"
  expected="$(jq --exit-status --raw-output ".$field" "$receipt")"
  test "$actual" = "$expected" || {
    echo "frozen host drift detected for $field: expected=$expected actual=$actual" >&2
    exit 1
  }
}

compare_file_digest() {
  field="$1"
  path="$2"
  compare_value "$field" "$(sha256sum "$path" | awk '{print $1}')"
}

compare_value source_commit "$(git -C "$repo_root" rev-parse HEAD)"
compare_value profile_sha256 "$(sha256sum "$profile" | awk '{print $1}')"
compare_value kernel_release "$(uname -r)"
compare_value kernel_command_line "$(cat /proc/cmdline)"
compare_file_digest tuning_plan_sha256 "$state_dir/plan.json"
compare_file_digest tuning_applied_sha256 "$state_dir/applied.json"
compare_file_digest provisioning_receipt_sha256 \
  "$repo_root/target/test-evidence/0.67.1/runner-provisioned.json"

comparison_dir="$(mktemp --directory "$state_dir/freeze-check.XXXXXX")"
cleanup() {
  rm --force \
    "$comparison_dir/packages.tsv" \
    "$comparison_dir/systemd-unit-files.tsv" \
    "$comparison_dir/systemd-active-state.tsv" \
    "$comparison_dir/sysctls.tsv"
  rmdir "$comparison_dir"
}
trap cleanup EXIT

LC_ALL=C dpkg-query --show --showformat='${binary:Package}\t${Version}\n' |
  sort >"$comparison_dir/packages.tsv"
LC_ALL=C systemctl list-unit-files --all --no-legend --no-pager |
  awk '$2 != "transient"' |
  sort >"$comparison_dir/systemd-unit-files.tsv"
LC_ALL=C systemctl list-units --all --type=service --type=timer --no-legend --no-pager --plain --full |
  sort >"$comparison_dir/systemd-active-state.tsv"
: >"$comparison_dir/sysctls.tsv"
while IFS= read -r key; do
  read_frozen_kernel_tunable "$key" >>"$comparison_dir/sysctls.tsv"
done < <(jq --raw-output '.freeze_contract.selected_sysctls[]' "$profile")

compare_file_digest package_manifest_sha256 "$comparison_dir/packages.tsv"
compare_file_digest systemd_unit_files_sha256 "$comparison_dir/systemd-unit-files.tsv"
compare_file_digest systemd_active_state_sha256 "$comparison_dir/systemd-active-state.tsv"
compare_file_digest sysctl_manifest_sha256 "$comparison_dir/sysctls.tsv"
echo "frozen host contract verified"
