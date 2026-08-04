#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

source_receipt="/var/lib/hydracache-perf/runner-provisioned.json"
test -f "$source_receipt" || {
  echo "offline provisioning receipt is missing: $source_receipt" >&2
  exit 1
}
test "$(stat --format=%U "$source_receipt")" = "root"
test "$(stat --format=%a "$source_receipt")" = "444"

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
test -z "$(git status --porcelain=v1 --untracked-files=normal)" || {
  echo "provisioning receipt import requires a clean worktree" >&2
  exit 1
}
commit="$(git rev-parse HEAD)"

jq --exit-status \
  --arg commit "$commit" '
    .schema_version == 4 and
    .release == "0.67.1" and
    .stage == "runner-provisioned" and
    .source_commit == $commit and
    .platform == "linux-x86_64" and
    .os_image == "ubuntu-24.04" and
    .virtualization == "none" and
    (.host_identity_digest | type == "string" and test("^[0-9a-f]{64}$")) and
    .measurement_cpuset == "1-4" and
    .cpu_isolation.smt_control == "off" and
    .cpu_isolation.online_cpus == "0-7" and
    .cpu_isolation.isolated_cpus == "1-4" and
    .cpu_isolation.nohz_full_cpus == "1-4" and
    .cpu_isolation.rcu_nocbs_cpus == "1-4" and
    .cpu_isolation.housekeeping_cpus == "0,5-7" and
    .cpu_isolation.irq_affinity_policy == "housekeeping-only-v1" and
    .cpu_isolation.measurement_idle_policy == "latency-cap-us-v1" and
    .cpu_isolation.measurement_max_idle_latency_us == 1 and
    .cpu_isolation.housekeeping_idle_policy == "latency-cap-us-v1" and
    .cpu_isolation.housekeeping_max_idle_latency_us == 1 and
    .storage_transport == "nvme" and
    .cgroup_version == 2 and
    .cgroup_cpu_quota == "unlimited" and
    .runner_name == "hydracache-perf-v1" and
    .runner_online == false and
    .ship_evidence_eligible == false
  ' "$source_receipt" >/dev/null

output="$repo_root/target/test-evidence/0.67.1/runner-provisioned.json"
mkdir --parents "$(dirname "$output")"
test ! -e "$output" || {
  echo "refusing to overwrite provisioning receipt: $output" >&2
  exit 1
}
cp --no-preserve=mode,ownership,timestamps "$source_receipt" "$output"
chmod 0444 "$output"
printf 'offline provisioning receipt imported: %s\n' "$output"
