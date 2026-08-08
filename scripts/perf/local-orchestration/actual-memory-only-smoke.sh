#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'
export LC_ALL=C

readonly source_root=/repo
readonly work_root=/work/repo
readonly fixture=/work/memory-only-actual-fixture
readonly runtime=/work/memory-only-actual-runtime
readonly materialized=/work/memory-only-actual-results
readonly profile=/work/memory-only-actual-profile.json

fail() {
  echo "actual memory-only smoke: $*" >&2
  exit 1
}

test -x /cargo-target/debug/hydracache-loadgen || fail "loadgen fixture is missing"
test -x /cargo-target/debug/hydracache-server || fail "server fixture is missing"
rm --recursive --force -- /work/repo "$fixture" "$runtime" "$materialized"
mkdir --parents /work/repo "$fixture/proc/self" \
  "$fixture/proc/sys/kernel/random" "$fixture/cgroup/test"
(cd "$source_root" && tar --exclude=.git --exclude=target --create --file=- .) |
  tar --extract --file=- --directory=/work/repo
cd "$work_root"
git init --quiet
git config user.email local-orchestration@invalid.example
git config user.name hydracache-local-orchestration
git add --all
git commit --quiet --message fixture

jq '.profile_id = "ubuntu-24.04-memory-only-local-test" | .cpu_contract.measurement_cpus = "0"' \
  docs/testing/perf-host-profiles/ubuntu-24.04-memory-only-v1.json >"$profile"
printf 'Filename Type Size Used Priority\n' >"$fixture/proc/swaps"
printf '0::/test\n' >"$fixture/proc/self/cgroup"
printf 'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee\n' \
  >"$fixture/proc/sys/kernel/random/boot_id"
printf '259 0 nvme0n1 1 0 2 0 3 0 4 0 0 0 0 0 0 0 0\n' \
  >"$fixture/proc/diskstats"
printf '       CPU0\n 24: 0 PCI-MSI 0-edge nvme0q0\n' \
  >"$fixture/proc/interrupts"
printf '259:0 rbytes=0 wbytes=0 rios=0 wios=0 dbytes=0 dios=0\n' \
  >"$fixture/cgroup/test/io.stat"

HC_LOCAL_ORCHESTRATION_TESTING=1 \
HC_MEMORY_ONLY_PROC_ROOT="$fixture/proc" \
HC_MEMORY_ONLY_CGROUP_ROOT="$fixture/cgroup" \
HC_MEMORY_ONLY_LOADGEN_SOURCE=/cargo-target/debug/hydracache-loadgen \
HC_MEMORY_ONLY_SERVER_SOURCE=/cargo-target/debug/hydracache-server \
  scripts/perf/run-memory-only-measurement.sh \
    --run-id actual-loadgen-smoke \
    --mode all \
    --profile "$profile" \
    --runtime-root "$runtime" \
    --materialized-root "$materialized"

readonly result="$materialized/actual-loadgen-smoke"
test ! -e "$runtime"
jq --exit-status '
  .stage == "reference-memory-only-run" and
  .passed == true and
  .qualification_evidence == false and
  .bootstrap_evidence == false and
  .ship_evidence_eligible == false and
  ([.windows[].mode] | sort) == ["client-surface", "local"] and
  (.binaries["hydracache-loadgen"] | test("^[0-9a-f]{64}$")) and
  (.binaries["hydracache-server"] | test("^[0-9a-f]{64}$"))
' "$result/memory-only-run.json" >/dev/null
for selected_mode in local client-surface; do
  jq --exit-status '.passed == true and .major_faults == 0' \
    "$result/$selected_mode/memory-only-window.json" >/dev/null
  test -s "$result/$selected_mode/report.json"
done
if HC_LOCAL_ORCHESTRATION_TESTING=1 \
  HC_MEMORY_ONLY_PROC_ROOT="$fixture/proc" \
  HC_MEMORY_ONLY_CGROUP_ROOT="$fixture/cgroup" \
  HC_MEMORY_ONLY_LOADGEN_SOURCE=/cargo-target/debug/hydracache-loadgen \
  HC_MEMORY_ONLY_SERVER_SOURCE=/cargo-target/debug/hydracache-server \
    scripts/perf/run-memory-only-measurement.sh \
      --run-id actual-loadgen-smoke \
      --mode local \
      --profile "$profile" \
      --runtime-root "$runtime" \
      --materialized-root "$materialized" >/dev/null 2>&1; then
  fail "immutable materialized run was overwritten"
fi
echo "actual HydraCache/loadgen memory-only smoke: OK"
