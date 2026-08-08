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
  scripts/perf/local-orchestration/actual-memory-only-smoke.sh \
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
PYTHONPATH=scripts/perf PYTHONPYCACHEPREFIX=/work/pycache \
  python3 -m unittest scripts/perf/reference_campaign_test.py
PYTHONPYCACHEPREFIX=/work/pycache \
  python3 -m unittest scripts/perf/reference_memory_only_window_test.py

readonly memory_fixture=/work/memory-only-fixture
readonly memory_runtime=/work/memory-only-runtime
readonly memory_profile=/work/memory-only-profile.json
mkdir --parents \
  "$memory_fixture/proc/self" "$memory_fixture/proc/sys/kernel/random" \
  "$memory_fixture/cgroup/test" \
  "$memory_runtime/bin" "$memory_runtime/results"
jq '.profile_id = "ubuntu-24.04-memory-only-local-test" | .cpu_contract.measurement_cpus = "0"' \
  docs/testing/perf-host-profiles/ubuntu-24.04-memory-only-v1.json >"$memory_profile"
printf 'Filename Type Size Used Priority\n' >"$memory_fixture/proc/swaps"
printf '0::/test\n' >"$memory_fixture/proc/self/cgroup"
printf '11111111-2222-3333-4444-555555555555\n' \
  >"$memory_fixture/proc/sys/kernel/random/boot_id"
printf '259 0 nvme0n1 1 0 2 0 3 0 4 0 0 0 0 0 0 0 0\n' \
  >"$memory_fixture/proc/diskstats"
printf '       CPU0\n 24: 0 PCI-MSI 0-edge nvme0q0\n' \
  >"$memory_fixture/proc/interrupts"
printf '259:0 rbytes=0 wbytes=0 rios=0 wios=0 dbytes=0 dios=0\n' \
  >"$memory_fixture/cgroup/test/io.stat"
cp /usr/bin/true "$memory_runtime/bin/workload"
chmod 0755 "$memory_runtime/bin/workload"
for _ in 1 2 3; do "$memory_runtime/bin/workload"; done
HC_LOCAL_ORCHESTRATION_TESTING=1 \
HC_MEMORY_ONLY_PROC_ROOT="$memory_fixture/proc" \
HC_MEMORY_ONLY_CGROUP_ROOT="$memory_fixture/cgroup" \
  scripts/perf/reference-memory-only-window.py \
    --profile "$memory_profile" \
    --runtime-root "$memory_runtime" \
    --output-dir "$memory_runtime/results/accepted" \
    -- "$memory_runtime/bin/workload"
jq --exit-status '
  .stage == "reference-memory-only-window" and
  (.source_commit | test("^[0-9a-f]{40}$")) and
  (.runtime_root_digest | test("^[0-9a-f]{64}$")) and
  .passed == true and
  .major_faults == 0 and
  .qualification_evidence == false and
  .bootstrap_evidence == false and
  .ship_evidence_eligible == false
' "$memory_runtime/results/accepted/memory-only-window.json" >/dev/null

cp /usr/bin/python3 "$memory_runtime/bin/mutate-diskstats"
chmod 0755 "$memory_runtime/bin/mutate-diskstats"
expect_failure memory-only-disk-activity \
  env HC_LOCAL_ORCHESTRATION_TESTING=1 \
    HC_MEMORY_ONLY_PROC_ROOT="$memory_fixture/proc" \
    HC_MEMORY_ONLY_CGROUP_ROOT="$memory_fixture/cgroup" \
    "$work_root/scripts/perf/reference-memory-only-window.py" \
      --profile "$memory_profile" \
      --runtime-root "$memory_runtime" \
      --output-dir "$memory_runtime/results/rejected-disk" \
      -- "$memory_runtime/bin/mutate-diskstats" -c \
        'from pathlib import Path; p=Path("/work/memory-only-fixture/proc/diskstats"); p.write_text(p.read_text().replace("1 0 2", "2 0 2"))'
jq --exit-status '
  .passed == false and
  (.violations.nvme_counters | length) > 0 and
  .ship_evidence_eligible == false
' "$memory_runtime/results/rejected-disk/memory-only-window.json" >/dev/null

expect_failure memory-only-cgroup-io \
  env HC_LOCAL_ORCHESTRATION_TESTING=1 \
    HC_MEMORY_ONLY_PROC_ROOT="$memory_fixture/proc" \
    HC_MEMORY_ONLY_CGROUP_ROOT="$memory_fixture/cgroup" \
    "$work_root/scripts/perf/reference-memory-only-window.py" \
      --profile "$memory_profile" \
      --runtime-root "$memory_runtime" \
      --output-dir "$memory_runtime/results/rejected-cgroup" \
      -- "$memory_runtime/bin/mutate-diskstats" -c \
        'from pathlib import Path; p=Path("/work/memory-only-fixture/cgroup/test/io.stat"); p.write_text(p.read_text().replace("rbytes=0", "rbytes=4096"))'
jq --exit-status '
  .passed == false and
  (.violations.cgroup_io | length) > 0
' "$memory_runtime/results/rejected-cgroup/memory-only-window.json" >/dev/null

expect_failure memory-only-nvme-irq \
  env HC_LOCAL_ORCHESTRATION_TESTING=1 \
    HC_MEMORY_ONLY_PROC_ROOT="$memory_fixture/proc" \
    HC_MEMORY_ONLY_CGROUP_ROOT="$memory_fixture/cgroup" \
    "$work_root/scripts/perf/reference-memory-only-window.py" \
      --profile "$memory_profile" \
      --runtime-root "$memory_runtime" \
      --output-dir "$memory_runtime/results/rejected-irq" \
      -- "$memory_runtime/bin/mutate-diskstats" -c \
        'from pathlib import Path; p=Path("/work/memory-only-fixture/proc/interrupts"); p.write_text(p.read_text().replace("24: 0", "24: 1"))'
jq --exit-status '
  .passed == false and
  (.violations.measurement_cpu_nvme_irqs | length) > 0
' "$memory_runtime/results/rejected-irq/memory-only-window.json" >/dev/null

expect_failure memory-only-runtime-drift \
  env HC_LOCAL_ORCHESTRATION_TESTING=1 \
    HC_MEMORY_ONLY_PROC_ROOT="$memory_fixture/proc" \
    HC_MEMORY_ONLY_CGROUP_ROOT="$memory_fixture/cgroup" \
    "$work_root/scripts/perf/reference-memory-only-window.py" \
      --profile "$memory_profile" \
      --runtime-root "$memory_runtime" \
      --output-dir "$memory_runtime/results/rejected-runtime" \
      -- "$memory_runtime/bin/mutate-diskstats" -c \
        'from pathlib import Path; Path("/work/memory-only-runtime/unexpected.txt").write_text("drift")'
jq --exit-status '
  .passed == false and
  (.violations.runtime_tree | length) > 0
' "$memory_runtime/results/rejected-runtime/memory-only-window.json" >/dev/null

expect_failure memory-only-executable-escape \
  env HC_LOCAL_ORCHESTRATION_TESTING=1 \
    HC_MEMORY_ONLY_PROC_ROOT="$memory_fixture/proc" \
    HC_MEMORY_ONLY_CGROUP_ROOT="$memory_fixture/cgroup" \
    "$work_root/scripts/perf/reference-memory-only-window.py" \
      --profile "$memory_profile" \
      --runtime-root "$memory_runtime" \
      --output-dir "$memory_runtime/results/rejected-escape" \
      -- /usr/bin/true

mkdir --parents /work/malformed-telemetry
printf '{not-json}\n' >/work/malformed-telemetry/input.jsonl
expect_failure malformed-telemetry \
  python3 scripts/perf/summarize-telemetry.py \
    --input /work/malformed-telemetry --output /work/summary.json
expect_failure irq-burn-in-short-window \
  scripts/perf/reference-host-irq-burn-in.sh \
    --output-dir /work/invalid-short-burn-in --duration-seconds 599

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

readonly campaign_source="/var/lib/hydracache-perf/reference-campaign-v1"
readonly campaign_bundle="$campaign_source/reference-campaign-host-admission.tar.gz"
readonly campaign_receipt="$campaign_source/reference-campaign-admission.json"
mkdir --parents /work/import-shims "$campaign_source"
cat >/work/import-shims/id <<'EOF'
#!/usr/bin/env bash
if test "$*" = "--user --name"; then
  echo github-runner
  exit 0
fi
exec /usr/bin/id "$@"
EOF
chmod 0755 /work/import-shims/id
printf 'immutable host admission fixture\n' >"$campaign_bundle"
campaign_bundle_sha256="$(sha256sum "$campaign_bundle" | awk '{print $1}')"
profile_sha256="$(sha256sum docs/testing/perf-host-profiles/ubuntu-24.04-reference-v1.json | awk '{print $1}')"
jq --null-input \
  --arg campaign_id hc0671-local-fixture \
  --arg source_commit "$fixture_commit" \
  --arg profile_sha256 "$profile_sha256" \
  --arg bundle_sha256 "$campaign_bundle_sha256" '
    {
      schema_version: 1,
      release: "0.67.1",
      stage: "reference-campaign-host-admission",
      campaign_id: $campaign_id,
      source_commit: $source_commit,
      profile_sha256: $profile_sha256,
      host_state_archive_sha256: ("1" * 64),
      irq_burn_in_receipt_sha256: ("2" * 64),
      irq_baseline_sha256: ("3" * 64),
      host_admission_bundle_sha256: $bundle_sha256,
      host_frozen: true,
      irq_burn_in_passed: true,
      passed: true,
      qualification_evidence: false,
      bootstrap_evidence: false,
      ship_evidence_eligible: false
    }
  ' >"$campaign_receipt"
chmod 0444 "$campaign_bundle" "$campaign_receipt"
chmod 0555 "$campaign_source"
PATH="/work/import-shims:$PATH" scripts/perf/import-reference-campaign-admission.sh
test -f target/test-evidence/0.67.1/reference-campaign/reference-campaign-admission.json
test -f target/test-evidence/0.67.1/reference-campaign/reference-campaign-host-admission.tar.gz
expect_failure campaign-admission-overwrite \
  env PATH="/work/import-shims:$PATH" scripts/perf/import-reference-campaign-admission.sh

printf 'static, malformed-input, tmpfs recovery, and receipt import checks: OK\n'
