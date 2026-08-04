#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'
export LC_ALL=C

readonly source_root="/repo"
readonly work_root="/work/repo"
readonly tmpfs_root="/dev/shm/hydracache-reference-evidence-v1"
readonly source_receipt="/var/lib/hydracache-perf/runner-provisioned.json"

fail() {
  echo "local orchestration smoke: $*" >&2
  exit 1
}

expect_failure() {
  local label="$1"
  shift
  if "$@" >/tmp/expected-failure.stdout 2>/tmp/expected-failure.stderr; then
    fail "negative canary unexpectedly passed: $label"
  fi
  printf 'negative canary rejected: %s\n' "$label"
}

cleanup() {
  rm --recursive --force -- "$tmpfs_root"
  rm --recursive --force -- /var/lib/hydracache-perf
}
trap cleanup EXIT

test -d "$source_root" || fail "read-only source mount is missing"
rm --recursive --force -- /work/repo
mkdir --parents /work/repo
(cd "$source_root" && tar --exclude=.git --exclude=target --create --file=- .) |
  tar --extract --file=- --directory=/work/repo
cd "$work_root"
git init --quiet
git config user.email local-orchestration@invalid.example
git config user.name hydracache-local-orchestration
git add --all
git commit --quiet --message fixture
fixture_commit="$(git rev-parse HEAD)"
readonly fixture_commit

mapfile -t shell_files < <(find scripts/perf -maxdepth 1 -type f -name '*.sh' -print | sort)
test "${#shell_files[@]}" -gt 0
shellcheck --external-sources --severity=error "${shell_files[@]}"
shellcheck --external-sources \
  scripts/perf/local-orchestration/static-and-fault-smoke.sh \
  scripts/perf/local-orchestration/systemd-smoke.sh
actionlint \
  -ignore 'unexpected key "(background|cancel)"' \
  -ignore 'step must run script with "run" section or run action with "uses" section' \
  -ignore 'label "hydracache-perf-v1" is unknown' \
  -ignore 'SC2129:style' \
  .github/workflows/*.yml
mkdir --parents /work/pycache
mapfile -t python_files < <(find scripts/perf -maxdepth 1 -type f -name '*.py' -print | sort)
PYTHONPYCACHEPREFIX=/work/pycache python3 -m py_compile "${python_files[@]}"

mkdir --parents /work/malformed-telemetry
printf '{not-json}\n' >/work/malformed-telemetry/input.jsonl
expect_failure malformed-telemetry \
  python3 scripts/perf/summarize-telemetry.py \
    --input /work/malformed-telemetry --output /work/summary.json

rm --recursive --force -- target/test-evidence "$tmpfs_root"
scripts/perf/reference-evidence-tmpfs.sh prepare
scripts/perf/reference-evidence-tmpfs.sh verify
printf 'fixture-evidence\n' >target/test-evidence/0.67/evidence.txt
mkdir --parents target/test-evidence/0.67.materializing
printf 'interrupted-copy\n' >target/test-evidence/0.67.materializing/stale.txt
scripts/perf/reference-evidence-tmpfs.sh materialize
test -f target/test-evidence/0.67/evidence.txt
test ! -e target/test-evidence/0.67.materializing
test ! -L target/test-evidence/0.67
test ! -L target/test-evidence/0.67.1
test ! -e "$tmpfs_root"
scripts/perf/reference-evidence-tmpfs.sh materialize

rm --recursive --force -- target/test-evidence
scripts/perf/reference-evidence-tmpfs.sh prepare
printf '%s\n' wrong-commit >"$tmpfs_root/source-commit"
expect_failure tmpfs-source-commit scripts/perf/reference-evidence-tmpfs.sh verify
rm --recursive --force -- target/test-evidence "$tmpfs_root"

mkdir --parents target/test-evidence "$tmpfs_root/0.67" "$tmpfs_root/0.67.1"
printf '%s\n' "$fixture_commit" >"$tmpfs_root/source-commit"
ln --symbolic /tmp target/test-evidence/0.67
ln --symbolic "$tmpfs_root/0.67.1" target/test-evidence/0.67.1
expect_failure tmpfs-alias-target scripts/perf/reference-evidence-tmpfs.sh verify
rm --recursive --force -- target/test-evidence "$tmpfs_root"

make_receipt() {
  local commit="$1"
  jq --null-input --arg commit "$commit" '
    {
      schema_version: 4,
      release: "0.67.1",
      stage: "runner-provisioned",
      source_commit: $commit,
      platform: "linux-x86_64",
      os_image: "ubuntu-24.04",
      virtualization: "none",
      host_identity_digest: ("d" * 64),
      measurement_cpuset: "1-4",
      cpu_isolation: {
        smt_control: "off",
        online_cpus: "0-7",
        isolated_cpus: "1-4",
        nohz_full_cpus: "1-4",
        rcu_nocbs_cpus: "1-4",
        housekeeping_cpus: "0,5-7",
        irq_affinity_policy: "housekeeping-only-v1",
        measurement_idle_policy: "latency-cap-us-v1",
        measurement_max_idle_latency_us: 1,
        housekeeping_idle_policy: "latency-cap-us-v1",
        housekeeping_max_idle_latency_us: 1
      },
      storage_transport: "nvme",
      cgroup_version: 2,
      cgroup_cpu_quota: "unlimited",
      runner_name: "hydracache-perf-v1",
      runner_online: false,
      ship_evidence_eligible: false
    }
  ' >"$source_receipt"
  chmod 0444 "$source_receipt"
}

mkdir --parents /var/lib/hydracache-perf
make_receipt "$fixture_commit"
scripts/perf/import-provisioning-receipt.sh
readonly imported="target/test-evidence/0.67.1/runner-provisioned.json"
test -f "$imported"
test "$(stat --format=%a "$imported")" = 444
expect_failure provisioning-overwrite scripts/perf/import-provisioning-receipt.sh

rm --force -- "$imported"
chmod 0644 "$source_receipt"
expect_failure provisioning-mode scripts/perf/import-provisioning-receipt.sh
make_receipt "$(printf '0%.0s' {1..40})"
expect_failure provisioning-source-commit scripts/perf/import-provisioning-receipt.sh
printf '{"schema_version":4' >"$source_receipt"
chmod 0444 "$source_receipt"
expect_failure provisioning-truncated-json scripts/perf/import-provisioning-receipt.sh

printf 'static, malformed-input, tmpfs recovery, and receipt import checks: OK\n'
