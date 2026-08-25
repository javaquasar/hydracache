#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

usage() {
  cat >&2 <<'EOF'
usage: scripts/perf/reference-host-tuning.sh <plan|apply|verify|freeze|restore> \
  [--profile PATH] [--state-dir PATH]

All actions are profile-driven and fail closed. `apply` changes only units named
in the profile. `freeze` records the exact environment that must remain unchanged
for a qualification/bootstrap sample family. `restore` uses the pre-apply state.
EOF
  exit 2
}

action="${1:-}"
case "$action" in
  plan|apply|verify|freeze|restore) shift ;;
  *) usage ;;
esac

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

for tool in awk date findmnt git grep jq lsblk lscpu sha256sum sort stat systemctl \
  systemd-detect-virt uname uniq wc; do
  command -v "$tool" >/dev/null || {
    echo "missing reference host tuning tool: $tool" >&2
    exit 1
  }
done
test -f "$profile" || {
  echo "host tuning profile does not exist: $profile" >&2
  exit 1
}

require_root() {
  test "$(id --user)" -eq 0 || {
    echo "$action requires root; run the reviewed script through sudo" >&2
    exit 1
  }
}

profile_value() {
  jq --exit-status --raw-output "$1" "$profile"
}

validate_profile() {
  jq --exit-status '
    .schema_version == 1 and
    (.profile_id | type == "string" and length > 0) and
    .operating_system.id == "ubuntu" and
    .operating_system.version_id == "24.04" and
    .operating_system.architecture == "x86_64" and
    (.operating_system.kernel_release_regex | type == "string" and length > 0) and
    .hardware.require_bare_metal == true and
    .hardware.require_cgroup_v2 == true and
    .hardware.require_unlimited_cpu_quota == true and
    .cpu_contract.measurement_cpus == "1-4" and
    .cpu_contract.housekeeping_cpus == "0,5-7" and
    .cpu_contract.expected_online_cpus == "0-7" and
    .cpu_contract.smt == "off" and
    .cpu_contract.governor == "performance" and
    .interrupt_contract.managed_irq_policy == "dormant-measurement-queues-v1" and
    .interrupt_contract.storage_io_cpus == .cpu_contract.housekeeping_cpus and
    .interrupt_contract.measurement_cpu_storage_io == "forbidden" and
    .interrupt_contract.maximum_measurement_cpu_irq_delta == 0 and
    (.service_policy.protected_units | type == "array" and length > 0) and
    (.service_policy.required_active_groups | type == "array" and length == 2) and
    (.service_policy.disable_if_present | type == "array") and
    (.service_policy.mask_if_present | type == "array") and
    (.service_policy.require_inactive_if_present | type == "array") and
    (.service_policy.report_only_candidates | type == "array") and
    (.freeze_contract.selected_sysctls | type == "array" and length > 0) and
    (.freeze_contract.invalidate_sample_family_on_change | type == "array" and length > 0)
  ' "$profile" >/dev/null || {
    echo "invalid or unsupported reference host profile: $profile" >&2
    exit 1
  }

  mapfile -t protected_units < <(jq --raw-output '.service_policy.protected_units[]' "$profile")
  mapfile -t mutable_units < <(
    jq --raw-output '
      .service_policy.disable_if_present[],
      .service_policy.mask_if_present[],
      .service_policy.require_inactive_if_present[]
    ' "$profile"
  )
  test "$(printf '%s\n' "${mutable_units[@]}" | sort | uniq -d | wc --lines)" -eq 0 || {
    echo "service policy contains duplicate mutable units" >&2
    exit 1
  }
  for unit in "${mutable_units[@]}"; do
    for protected in "${protected_units[@]}"; do
      test "$unit" != "$protected" || {
        echo "service policy attempts to mutate protected unit: $unit" >&2
        exit 1
      }
    done
  done
}

validate_host_compatibility() {
  test "$(uname -s)" = "Linux" || {
    echo "reference host must run Linux" >&2
    exit 1
  }
  test "$(uname -m)" = "$(profile_value '.operating_system.architecture')" || {
    echo "reference host architecture does not match the profile" >&2
    exit 1
  }
  # shellcheck disable=SC1091
  . /etc/os-release
  test "$ID" = "$(profile_value '.operating_system.id')" || {
    echo "reference host OS id does not match the profile" >&2
    exit 1
  }
  test "$VERSION_ID" = "$(profile_value '.operating_system.version_id')" || {
    echo "reference host OS version does not match the profile" >&2
    exit 1
  }
  kernel_regex="$(profile_value '.operating_system.kernel_release_regex')"
  [[ "$(uname -r)" =~ $kernel_regex ]] || {
    echo "kernel release $(uname -r) does not match profile regex $kernel_regex" >&2
    exit 1
  }
  if systemd-detect-virt --quiet; then
    echo "reference profile requires bare metal; detected $(systemd-detect-virt)" >&2
    exit 1
  fi
  test "$(stat --file-system --format=%T /sys/fs/cgroup)" = "cgroup2fs" || {
    echo "reference profile requires cgroup v2" >&2
    exit 1
  }

  physical_cores="$(
    lscpu --parse=SOCKET,CORE |
      grep --invert-match '^#' |
      sort --unique |
      wc --lines
  )"
  minimum_physical_cores="$(profile_value '.hardware.minimum_physical_cores')"
  test "$physical_cores" -ge "$minimum_physical_cores" || {
    echo "reference host has $physical_cores physical cores; need $minimum_physical_cores" >&2
    exit 1
  }
  memory_bytes="$(awk '/^MemTotal:/ { printf "%.0f\n", $2 * 1024 }' /proc/meminfo)"
  minimum_memory_bytes="$(profile_value '.hardware.minimum_memory_bytes')"
  test "$memory_bytes" -ge "$minimum_memory_bytes" || {
    echo "reference host has $memory_bytes bytes of RAM; need $minimum_memory_bytes" >&2
    exit 1
  }
  lsblk --nodeps --noheadings --output TRAN | grep --quiet '^nvme$' || {
    echo "reference profile requires an NVMe block device" >&2
    exit 1
  }

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
  test "$cpu_controller_observed" = true || {
    echo "no cgroup v2 CPU controller was observed" >&2
    exit 1
  }
}

unit_load_state() {
  systemctl show "$1" --property=LoadState --value 2>/dev/null || true
}

unit_known() {
  load_state="$(unit_load_state "$1")"
  test -n "$load_state" && test "$load_state" != "not-found"
}

unit_state_json() {
  unit="$1"
  policy="$2"
  planned_action="$3"
  load_state="$(unit_load_state "$unit")"
  if test -z "$load_state" || test "$load_state" = "not-found"; then
    known=false
    active_state="not-found"
    unit_file_state="not-found"
    planned_action="absent-noop"
  else
    known=true
    active_state="$(systemctl is-active "$unit" 2>/dev/null || true)"
    unit_file_state="$(systemctl is-enabled "$unit" 2>/dev/null || true)"
  fi
  jq --null-input --compact-output \
    --arg unit "$unit" \
    --arg policy "$policy" \
    --arg planned_action "$planned_action" \
    --arg load_state "$load_state" \
    --arg active_state "$active_state" \
    --arg unit_file_state "$unit_file_state" \
    --argjson known "$known" \
    '{unit: $unit, policy: $policy, planned_action: $planned_action,
      known: $known, load_state: $load_state, active_state: $active_state,
      unit_file_state: $unit_file_state}'
}

assert_runner_and_rootless_docker_offline() {
  mapfile -t runner_units < <(
    systemctl list-unit-files --type=service --no-legend \
      'actions.runner.javaquasar-hydracache.*.service' 2>/dev/null |
      awk '{print $1}'
  )
  for unit in "${runner_units[@]}"; do
    systemctl is-active --quiet "$unit" && {
      echo "runner service must be offline before host tuning/freeze: $unit" >&2
      exit 1
    }
  done
  if id github-runner >/dev/null 2>&1; then
    runner_uid="$(id --user github-runner)"
    test ! -S "/run/user/${runner_uid}/docker.sock" || {
      echo "rootless Docker socket must be absent before host tuning/freeze" >&2
      exit 1
    }
    if command -v pgrep >/dev/null && pgrep --uid "$runner_uid" --exact dockerd >/dev/null; then
      echo "rootless dockerd must be stopped before host tuning/freeze" >&2
      exit 1
    fi
  fi
}

verify_required_active_groups() {
  group_count="$(jq '.service_policy.required_active_groups | length' "$profile")"
  for ((index = 0; index < group_count; index += 1)); do
    group_id="$(jq --raw-output ".service_policy.required_active_groups[$index].id" "$profile")"
    active=false
    while IFS= read -r unit; do
      if unit_known "$unit" && systemctl is-active --quiet "$unit"; then
        active=true
        break
      fi
    done < <(jq --raw-output ".service_policy.required_active_groups[$index].one_of[]" "$profile")
    test "$active" = true || {
      echo "no active protected unit satisfies required group: $group_id" >&2
      exit 1
    }
  done
}

verify_service_policy() {
  while IFS= read -r unit; do
    unit_known "$unit" || continue
    systemctl is-active --quiet "$unit" && {
      echo "noise-control unit remains active: $unit" >&2
      exit 1
    }
    unit_file_state="$(systemctl is-enabled "$unit" 2>/dev/null || true)"
    case "$unit_file_state" in
      disabled|masked|masked-runtime) ;;
      *) echo "noise-control unit is not disabled: $unit ($unit_file_state)" >&2; exit 1 ;;
    esac
  done < <(jq --raw-output '.service_policy.disable_if_present[]' "$profile")

  while IFS= read -r unit; do
    unit_known "$unit" || continue
    systemctl is-active --quiet "$unit" && {
      echo "masked noise-control unit remains active: $unit" >&2
      exit 1
    }
    unit_file_state="$(systemctl is-enabled "$unit" 2>/dev/null || true)"
    case "$unit_file_state" in
      masked|masked-runtime) ;;
      *) echo "noise-control unit is not masked: $unit ($unit_file_state)" >&2; exit 1 ;;
    esac
  done < <(jq --raw-output '.service_policy.mask_if_present[]' "$profile")

  while IFS= read -r unit; do
    unit_known "$unit" || continue
    systemctl is-active --quiet "$unit" && {
      echo "required-inactive unit remains active: $unit" >&2
      exit 1
    }
  done < <(jq --raw-output '.service_policy.require_inactive_if_present[]' "$profile")
  verify_required_active_groups
}

write_plan() {
  require_root
  mkdir -p "$state_dir"
  chmod 0700 "$state_dir"
  actions_ndjson="$(mktemp "$state_dir/actions.XXXXXX.ndjson")"
  trap 'rm -f "${actions_ndjson:-}"' RETURN
  while IFS= read -r unit; do
    unit_state_json "$unit" disable_if_present disable-now >>"$actions_ndjson"
  done < <(jq --raw-output '.service_policy.disable_if_present[]' "$profile")
  while IFS= read -r unit; do
    unit_state_json "$unit" mask_if_present mask-now >>"$actions_ndjson"
  done < <(jq --raw-output '.service_policy.mask_if_present[]' "$profile")
  while IFS= read -r unit; do
    unit_state_json "$unit" require_inactive_if_present stop-disable-now >>"$actions_ndjson"
  done < <(jq --raw-output '.service_policy.require_inactive_if_present[]' "$profile")
  while IFS= read -r unit; do
    unit_state_json "$unit" report_only_candidates report-only >>"$actions_ndjson"
  done < <(jq --raw-output '.service_policy.report_only_candidates[]' "$profile")

  profile_sha256="$(sha256sum "$profile" | awk '{print $1}')"
  generated_at="$(date --utc +%Y-%m-%dT%H:%M:%SZ)"
  jq --null-input \
    --argjson schema_version 1 \
    --arg generated_at "$generated_at" \
    --arg profile_id "$(profile_value '.profile_id')" \
    --arg profile_sha256 "$profile_sha256" \
    --arg os_release "$(. /etc/os-release; printf '%s-%s' "$ID" "$VERSION_ID")" \
    --arg kernel_release "$(uname -r)" \
    --arg source_commit "$(git -C "$repo_root" rev-parse HEAD)" \
    --arg measurement_cpus "$(profile_value '.cpu_contract.measurement_cpus')" \
    --arg housekeeping_cpus "$(profile_value '.cpu_contract.housekeeping_cpus')" \
    --slurpfile actions "$actions_ndjson" \
    '{schema_version: $schema_version, generated_at: $generated_at,
      profile_id: $profile_id, profile_sha256: $profile_sha256,
      os_release: $os_release, kernel_release: $kernel_release,
      source_commit: $source_commit,
      cpu_contract: {measurement_cpus: $measurement_cpus,
        housekeeping_cpus: $housekeeping_cpus}, actions: $actions}' \
    >"$state_dir/plan.json"
  chmod 0444 "$state_dir/plan.json"
  rm -f "$actions_ndjson"
  trap - RETURN
  printf '%s\n' "$state_dir/plan.json"
}

apply_policy() {
  require_root
  test ! -e "$state_dir/applied.json" || {
    echo "state directory already contains applied.json; use a fresh state directory" >&2
    exit 1
  }
  assert_runner_and_rootless_docker_offline
  plan_path="$(write_plan)"
  while IFS= read -r unit; do
    unit_known "$unit" || continue
    systemctl disable --now "$unit"
  done < <(jq --raw-output '.service_policy.disable_if_present[]' "$profile")
  while IFS= read -r unit; do
    unit_known "$unit" || continue
    systemctl mask --now "$unit"
  done < <(jq --raw-output '.service_policy.mask_if_present[]' "$profile")
  while IFS= read -r unit; do
    unit_known "$unit" || continue
    systemctl disable --now "$unit"
  done < <(jq --raw-output '.service_policy.require_inactive_if_present[]' "$profile")
  systemctl daemon-reload
  verify_service_policy
  jq --null-input \
    --arg applied_at "$(date --utc +%Y-%m-%dT%H:%M:%SZ)" \
    --arg plan_sha256 "$(sha256sum "$plan_path" | awk '{print $1}')" \
    '{schema_version: 1, applied_at: $applied_at, plan_sha256: $plan_sha256,
      exact_allowlist_only: true}' >"$state_dir/applied.json"
  chmod 0444 "$state_dir/applied.json"
  printf '%s\n' "$state_dir/applied.json"
}

verify_full_contract() {
  require_root
  assert_runner_and_rootless_docker_offline
  verify_service_policy
  "$repo_root/scripts/perf/provision-reference-isolation.sh" verify
}

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

freeze_host() {
  require_root
  test -z "$(git -C "$repo_root" status --porcelain=v1 --untracked-files=normal)" || {
    echo "host freeze requires a clean worktree" >&2
    exit 1
  }
  verify_full_contract
  for prerequisite in "$state_dir/plan.json" "$state_dir/applied.json"; do
    test -f "$prerequisite" || {
      echo "host freeze requires profile-driven apply evidence: $prerequisite" >&2
      exit 1
    }
  done
  expected_plan_sha256="$(jq --raw-output '.plan_sha256' "$state_dir/applied.json")"
  actual_plan_sha256="$(sha256sum "$state_dir/plan.json" | awk '{print $1}')"
  test "$actual_plan_sha256" = "$expected_plan_sha256" || {
    echo "applied host tuning plan digest does not match plan.json" >&2
    exit 1
  }
  "$repo_root/scripts/perf/audit-reference-host.sh" --mode provisioned
  mkdir -p "$state_dir/freeze"
  chmod 0700 "$state_dir/freeze"
  LC_ALL=C dpkg-query --show --showformat='${binary:Package}\t${Version}\n' |
    sort >"$state_dir/freeze/packages.tsv"
  LC_ALL=C systemctl list-unit-files --all --no-legend --no-pager |
    awk '$2 != "transient"' |
    sort >"$state_dir/freeze/systemd-unit-files.tsv"
  LC_ALL=C systemctl list-units --all --type=service --type=timer --no-legend --no-pager |
    sort >"$state_dir/freeze/systemd-active-state.tsv"
  : >"$state_dir/freeze/sysctls.tsv"
  while IFS= read -r key; do
    command -v sysctl >/dev/null || {
      echo "missing reference host freeze tool: sysctl" >&2
      exit 1
    }
    read_frozen_kernel_tunable "$key" >>"$state_dir/freeze/sysctls.tsv"
  done < <(jq --raw-output '.freeze_contract.selected_sysctls[]' "$profile")
  lscpu --json >"$state_dir/freeze/lscpu.json"
  lsblk --json --bytes --output NAME,KNAME,PATH,TYPE,TRAN,SIZE,ROTA,MOUNTPOINTS \
    >"$state_dir/freeze/lsblk.json"
  cp -- /etc/os-release "$state_dir/freeze/os-release"
  printf '%s\n' "$(cat /proc/cmdline)" >"$state_dir/freeze/kernel-command-line.txt"
  apt-mark showhold | LC_ALL=C sort >"$state_dir/freeze/apt-holds.txt"

  profile_sha256="$(sha256sum "$profile" | awk '{print $1}')"
  package_sha256="$(sha256sum "$state_dir/freeze/packages.tsv" | awk '{print $1}')"
  unit_files_sha256="$(sha256sum "$state_dir/freeze/systemd-unit-files.tsv" | awk '{print $1}')"
  active_state_sha256="$(sha256sum "$state_dir/freeze/systemd-active-state.tsv" | awk '{print $1}')"
  sysctl_sha256="$(sha256sum "$state_dir/freeze/sysctls.tsv" | awk '{print $1}')"
  provisioning_receipt="$repo_root/target/test-evidence/0.67.1/runner-provisioned.json"
  test -f "$provisioning_receipt"
  provisioning_receipt_sha256="$(sha256sum "$provisioning_receipt" | awk '{print $1}')"
  jq --null-input \
    --arg frozen_at "$(date --utc +%Y-%m-%dT%H:%M:%SZ)" \
    --arg profile_id "$(profile_value '.profile_id')" \
    --arg profile_sha256 "$profile_sha256" \
    --arg source_commit "$(git -C "$repo_root" rev-parse HEAD)" \
    --arg os_id "$(. /etc/os-release; printf '%s' "$ID")" \
    --arg os_version_id "$(. /etc/os-release; printf '%s' "$VERSION_ID")" \
    --arg kernel_release "$(uname -r)" \
    --arg kernel_command_line "$(cat /proc/cmdline)" \
    --arg package_manifest_sha256 "$package_sha256" \
    --arg systemd_unit_files_sha256 "$unit_files_sha256" \
    --arg systemd_active_state_sha256 "$active_state_sha256" \
    --arg sysctl_manifest_sha256 "$sysctl_sha256" \
    --arg provisioning_receipt_sha256 "$provisioning_receipt_sha256" \
    --argjson invalidate_on_change "$(jq '.freeze_contract.invalidate_sample_family_on_change' "$profile")" \
    --arg tuning_plan_sha256 "$actual_plan_sha256" \
    --arg tuning_applied_sha256 "$(sha256sum "$state_dir/applied.json" | awk '{print $1}')" \
    '{schema_version: 1, frozen_at: $frozen_at, profile_id: $profile_id,
      profile_sha256: $profile_sha256, source_commit: $source_commit,
      operating_system: {id: $os_id, version_id: $os_version_id},
      kernel_release: $kernel_release, kernel_command_line: $kernel_command_line,
      package_manifest_sha256: $package_manifest_sha256,
      systemd_unit_files_sha256: $systemd_unit_files_sha256,
      systemd_active_state_sha256: $systemd_active_state_sha256,
      sysctl_manifest_sha256: $sysctl_manifest_sha256,
      provisioning_receipt_sha256: $provisioning_receipt_sha256,
      tuning_plan_sha256: $tuning_plan_sha256,
      tuning_applied_sha256: $tuning_applied_sha256,
      sample_family_frozen: true, invalidate_sample_family_on_change: $invalidate_on_change}' \
    >"$state_dir/freeze/host-freeze.json"
  chmod -R a-w "$state_dir/freeze"
  printf '%s\n' "$state_dir/freeze/host-freeze.json"
}

restore_policy() {
  require_root
  plan_path="$state_dir/plan.json"
  test -f "$plan_path" && test -f "$state_dir/applied.json" || {
    echo "restore requires plan.json and applied.json from the same state directory" >&2
    exit 1
  }
  test ! -e "$state_dir/restored.json" || {
    echo "state directory already contains restored.json" >&2
    exit 1
  }
  while IFS= read -r entry; do
    unit="$(jq --raw-output '.unit' <<<"$entry")"
    previous_file_state="$(jq --raw-output '.unit_file_state' <<<"$entry")"
    previous_active_state="$(jq --raw-output '.active_state' <<<"$entry")"
    case "$previous_file_state" in
      masked|masked-runtime) systemctl mask "$unit" ;;
      enabled|enabled-runtime|linked|linked-runtime)
        systemctl unmask "$unit" || true
        systemctl enable "$unit"
        ;;
      disabled)
        systemctl unmask "$unit" || true
        systemctl disable "$unit"
        ;;
      static|indirect|generated|transient|alias)
        systemctl unmask "$unit" || true
        ;;
      *) echo "unsupported pre-apply unit state for $unit: $previous_file_state" >&2; exit 1 ;;
    esac
    case "$previous_active_state" in
      active|activating|reloading) systemctl start "$unit" ;;
      inactive|failed|deactivating|unknown) systemctl stop "$unit" || true ;;
      *) echo "unsupported pre-apply active state for $unit: $previous_active_state" >&2; exit 1 ;;
    esac
  done < <(jq --compact-output '.actions[] | select(.known and .policy != "report_only_candidates")' "$plan_path")
  systemctl daemon-reload
  jq --null-input \
    --arg restored_at "$(date --utc +%Y-%m-%dT%H:%M:%SZ)" \
    --arg plan_sha256 "$(sha256sum "$plan_path" | awk '{print $1}')" \
    '{schema_version: 1, restored_at: $restored_at,
      plan_sha256: $plan_sha256, restored_from_recorded_pre_state: true}' \
    >"$state_dir/restored.json"
  chmod 0444 "$state_dir/restored.json"
  printf '%s\n' "$state_dir/restored.json"
}

validate_profile
validate_host_compatibility
case "$action" in
  plan) write_plan ;;
  apply) apply_policy ;;
  verify) verify_full_contract ;;
  freeze) freeze_host ;;
  restore) restore_policy ;;
esac
