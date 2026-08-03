#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

# Stage 3 is an exploratory metrics matrix. It is deliberately separate from
# qualification/bootstrap and from the Stage 1/2 output roots.
repo_root="$(git rev-parse --show-toplevel)"
output_dir="${1-/dev/shm/hydracache-metric-expansion-$(date -u +%Y%m%dT%H%M%SZ)}"
benchmark="${REDIS_BENCHMARK-/usr/bin/redis-benchmark}"
redis_image="${REDIS_IMAGE-redis@sha256:3aaec283e6e593bde528077d60280ac1589887067a39273348860837c9346d7e}"
hazelcast_image="${HAZELCAST_IMAGE-}"
hazelcast_client_python="${HAZELCAST_CLIENT_PYTHON-python3}"
hazelcast_client_version="${HAZELCAST_CLIENT_VERSION-5.5.0}"
affinity="${MEASUREMENT_AFFINITY-4}"
interval="${TELEMETRY_INTERVAL_SECONDS-1}"
duration="${METRIC_DURATION_SECONDS-45}"
long_duration="${METRIC_LONG_DURATION_SECONDS-180}"
requests="${METRIC_REQUESTS-20000}"
cycles="${METRIC_CYCLES-3}"
active_target=""; active_container=""; active_pid=""

mkdir -p "$output_dir/metric-experiments" "$output_dir/metadata"
export MEASUREMENT_AFFINITY="$affinity"

require_tools() {
  test -x "$benchmark"
  test -x "$(command -v redis-cli)"
  test -x "$(command -v nc)"
  test -x "$repo_root/target/release/hydracache-server"
  test -x "$(command -v taskset)"
  test -n "$hazelcast_image" && [[ "$hazelcast_image" =~ @sha256:[0-9a-fA-F]{64}$ ]]
  "$hazelcast_client_python" -c 'import hazelcast'
  "$hazelcast_client_python" -c "import importlib.metadata as m; assert m.version('hazelcast-python-client') == '$hazelcast_client_version'"
}

wait_resp() { local port="$1"; for _ in $(seq 1 100); do printf '*1\r\n$4\r\nping\r\n' | nc -w1 127.0.0.1 "$port" 2>/dev/null | grep -q PONG && return 0; sleep .2; done; return 1; }
wait_hz() { for _ in $(seq 1 120); do "$hazelcast_client_python" -c 'import hazelcast; c=hazelcast.HazelcastClient(cluster_members=["127.0.0.1:5701"]); c.cluster_service.get_members(); c.shutdown()' >/dev/null 2>&1 && return 0; sleep 1; done; return 1; }
pin_container() {
  local container="$1" dir="$2" pid
  pid="$(docker inspect --format '{{.State.Pid}}' "$container")"
  test -n "$pid" && test "$pid" -gt 0
  taskset --cpu-list -p "$affinity" "$pid" >"$dir/affinity.txt" 2>&1
  grep -q "Cpus_allowed_list:[[:space:]]*${affinity}" "/proc/$pid/status"
}

stop_target() {
  set +e
  if [[ "$active_target" == hydra && -n "$active_pid" ]]; then kill "$active_pid" 2>/dev/null || true; wait "$active_pid" 2>/dev/null || true; fi
  [[ -n "$active_container" ]] && docker rm -f "$active_container" >/dev/null 2>&1 || true
  active_target=""; active_container=""; active_pid=""
  set -e
}

start_target() {
  local target="$1" dir="$2" mode="${3-default}" memory_limit="${4-}" name="metric-expansion-${target}-$$"
  case "$target" in
    hydra)
      rm -rf "$dir/hydra-data"; mkdir -p "$dir/hydra-data"
      local envs=(HYDRACACHE_ROLE=local HYDRACACHE_LISTEN_ADDR=127.0.0.1:0 HYDRACACHE_CLUSTER_ADDR=127.0.0.1:0 HYDRACACHE_STORAGE_DIR="$dir/hydra-data" HYDRACACHE_ADMIN_API_ENABLED=true HYDRACACHE_ADMIN_ADDR=127.0.0.1:6390 HYDRACACHE_REDIS_API_ENABLED=true HYDRACACHE_REDIS_ADDR=127.0.0.1:6380)
      [[ "$mode" == allocator-trim ]] && envs+=(MALLOC_TRIM_THRESHOLD_=131072 MALLOC_MMAP_THRESHOLD_=131072)
      nohup taskset --cpu-list "$affinity" env "${envs[@]}" "$repo_root/target/release/hydracache-server" >"$dir/target.log" 2>&1 &
      active_pid=$!; active_target=hydra; echo "$active_pid" >"$dir/target.pid"; wait_resp 6380;;
    redis)
      mkdir -p "$dir/redis-data"; local args=(redis-server --save "" --appendonly no)
      [[ "$mode" == rdb ]] && args=(redis-server --save '1 1' --appendonly no --dir /data --dbfilename dump.rdb)
      [[ "$mode" == aof ]] && args=(redis-server --save "" --appendonly yes --appendfsync everysec --dir /data)
      local docker_args=(run -d --name "$name" --network host --cpuset-cpus "$affinity" --user "$(id -u):$(id -g)" -v "$dir/redis-data:/data")
      [[ -n "$memory_limit" ]] && docker_args+=(--memory "$memory_limit")
      docker "${docker_args[@]}" "$redis_image" "${args[@]}" >"$dir/container-id.txt" 2>"$dir/docker.log"
      active_container="$name"; active_target=redis; docker inspect "$name" >"$dir/container.inspect.json"; docker image inspect "$redis_image" >"$dir/image.inspect.json"; if ! pin_container "$name" "$dir"; then return 1; fi; wait_resp 6379;;
    hazelcast)
      local docker_args=(run -d --name "$name" --network host --cpuset-cpus "$affinity")
      [[ -n "$memory_limit" ]] && docker_args+=(--memory "$memory_limit")
      docker "${docker_args[@]}" "$hazelcast_image" >"$dir/container-id.txt" 2>"$dir/docker.log"
      active_container="$name"; active_target=hazelcast; docker inspect "$name" >"$dir/container.inspect.json"; docker image inspect "$hazelcast_image" >"$dir/image.inspect.json"; if ! pin_container "$name" "$dir"; then return 1; fi; wait_hz;;
    *) return 1;;
  esac
}

target_pid() { [[ "$1" == hydra ]] && echo "$active_pid" || docker inspect --format '{{.State.Pid}}' "$active_container"; }
target_port() { [[ "$1" == hydra ]] && echo 6380 || echo 6379; }

run_resp_op() {
  local target="$1" operation="$2" payload="$3" clients="$4" pipeline="$5" count="$6" keyrange="$7" keylength="$8" distribution="$9" ttl_ms="${10}" output="${11}"
  [[ "$count" -gt 0 ]] || { echo '{"skipped":"zero-count"}' >>"$output"; return 0; }
  python3 "$repo_root/scripts/perf/metric-workload.py" --port "$(target_port "$target")" --operation "$operation" --payload "$payload" --clients "$clients" --pipeline "$pipeline" --requests "$count" --key-range "$keyrange" --key-length "$keylength" --distribution "$distribution" --ttl-ms "$ttl_ms" >>"$output"
}

run_hz_op() {
  local operation="$1" payload="$2" clients="$3" pipeline="$4" count="$5" keyrange="$6" keylength="$7" distribution="$8" output="$9"
  [[ "$count" -gt 0 ]] || { echo '{"skipped":"zero-count"}' >>"$output"; return 0; }
  "$hazelcast_client_python" "$repo_root/scripts/perf/hazelcast-workload.py" --host 127.0.0.1 --port 5701 --payload "$payload" --clients "$clients" --pipeline "$pipeline" --requests "$count" --key-range "$keyrange" --key-length "$keylength" --distribution "$distribution" --operation "$operation" >>"$output"
}

run_op() {
  local target="$1" operation="$2" payload="$3" clients="$4" pipeline="$5" count="$6" keyrange="$7" keylength="$8" distribution="$9" ttl_ms="${10}" output="${11}"
  if [[ "$target" == hazelcast ]]; then run_hz_op "$operation" "$payload" "$clients" "$pipeline" "$count" "$keyrange" "$keylength" "$distribution" "$output"; else run_resp_op "$target" "$operation" "$payload" "$clients" "$pipeline" "$count" "$keyrange" "$keylength" "$distribution" "$ttl_ms" "$output"; fi
}

run_case() {
  local exp="$1" target="$2" case_id="$3" payload="$4" clients="$5" pipeline="$6" count="$7" keyrange="$8" keylength="$9" distribution="${10}" ttl_ms="${11}" mode="${12}" memory_limit="${13}" kind="${14}" set_pct="${15}" case_duration="${16}" jvm_probe="${17-0}"
  local dir="$output_dir/metric-experiments/$exp/$target/$case_id"; mkdir -p "$dir/telemetry" "$dir/raw"
  {
    echo "experiment=$exp"; echo "target=$target"; echo "case=$case_id"; echo "payload_bytes=$payload"; echo "clients=$clients"; echo "pipeline=$pipeline"; echo "requests=$count"; echo "key_range=$keyrange"; echo "key_length=$keylength"; echo "distribution=$distribution"; echo "ttl_ms=$ttl_ms"; echo "mode=$mode"; echo "memory_limit=$memory_limit"; echo "kind=$kind"; echo "set_percent=$set_pct"; echo "affinity=$affinity"; echo "duration_seconds=$case_duration"
  } >"$dir/case-metadata.txt"
  if ! start_target "$target" "$dir" "$mode" "$memory_limit"; then echo failed >"$dir/status.txt"; printf '%s\t%s\t%s\tfailed\tstart_failed\n' "$exp" "$target" "$case_id" >>"$output_dir/case-status.tsv"; stop_target; return 0; fi
  local pid collector workload_status=0 telemetry="$dir/telemetry/telemetry.jsonl"; pid="$(target_pid "$target")"
  local collector_args=(--target "$target" --output "$telemetry" --interval "$interval" --duration "$((case_duration + 15))")
  [[ "$target" == hydra ]] && collector_args+=(--pid "$pid") || collector_args+=(--container "$active_container")
  [[ "$target" == hazelcast && "$jvm_probe" == 1 ]] && collector_args+=(--jvm-container "$active_container")
  python3 "$repo_root/scripts/perf/collect-target-telemetry.py" "${collector_args[@]}" >"$dir/collector.log" 2>&1 & collector=$!
  set +e
  case "$kind" in
    long|pressure)
      local until=$((SECONDS + case_duration)); while (( SECONDS < until )); do
        run_op "$target" set "$payload" "$clients" "$pipeline" "$count" "$keyrange" "$keylength" "$distribution" "$ttl_ms" "$dir/raw/workload.jsonl" >>"$dir/raw/workload.log" 2>&1 || workload_status=1
        run_op "$target" get "$payload" "$clients" "$pipeline" "$count" "$keyrange" "$keylength" "$distribution" 0 "$dir/raw/workload.jsonl" >>"$dir/raw/workload.log" 2>&1 || workload_status=1
      done;;
    ttl)
      for cycle in $(seq 1 "$cycles"); do
        run_op "$target" set "$payload" "$clients" "$pipeline" "$count" "$keyrange" "$keylength" "$distribution" "$ttl_ms" "$dir/raw/workload.jsonl" >>"$dir/raw/workload.log" 2>&1 || workload_status=1
        [[ "$target" == hazelcast ]] || redis-cli -h 127.0.0.1 -p "$(target_port "$target")" DBSIZE >>"$dir/raw/dbsize-$cycle-before.txt" 2>&1 || workload_status=1
        sleep 5
        [[ "$target" == hazelcast ]] || redis-cli -h 127.0.0.1 -p "$(target_port "$target")" DBSIZE >>"$dir/raw/dbsize-$cycle-after.txt" 2>&1 || workload_status=1
      done;;
    mix)
      local set_count get_count; set_count=$((count * set_pct / 100)); get_count=$((count - set_count));
      run_op "$target" set "$payload" "$clients" "$pipeline" "$set_count" "$keyrange" "$keylength" "$distribution" 0 "$dir/raw/workload.jsonl" >>"$dir/raw/workload.log" 2>&1 || workload_status=1
      run_op "$target" get "$payload" "$clients" "$pipeline" "$get_count" "$keyrange" "$keylength" "$distribution" 0 "$dir/raw/workload.jsonl" >>"$dir/raw/workload.log" 2>&1 || workload_status=1
      sleep "$case_duration";;
    one|allocator|jvm)
      run_op "$target" set "$payload" "$clients" "$pipeline" "$count" "$keyrange" "$keylength" "$distribution" "$ttl_ms" "$dir/raw/workload.jsonl" >>"$dir/raw/workload.log" 2>&1 || workload_status=1
      run_op "$target" get "$payload" "$clients" "$pipeline" "$count" "$keyrange" "$keylength" "$distribution" 0 "$dir/raw/workload.jsonl" >>"$dir/raw/workload.log" 2>&1 || workload_status=1
      sleep "$case_duration";;
  esac
  set -e
  kill -TERM "$collector" 2>/dev/null || true; wait "$collector" 2>/dev/null || true
  python3 "$repo_root/scripts/perf/summarize-telemetry.py" --input "$dir/telemetry" --output "$dir/telemetry-summary.json" || true
  [[ -n "$active_container" ]] && docker inspect "$active_container" >"$dir/container.inspect.final.json" 2>/dev/null || true
  stop_target
  if [[ "$workload_status" -eq 0 ]]; then echo complete >"$dir/status.txt"; printf '%s\t%s\t%s\tcomplete\t%s\n' "$exp" "$target" "$case_id" "$kind" >>"$output_dir/case-status.tsv"; else echo failed >"$dir/status.txt"; printf '%s\t%s\t%s\tfailed\tworkload\n' "$exp" "$target" "$case_id" >>"$output_dir/case-status.tsv"; fi
}

require_tools
{
  echo "stage=metric-expansion"; echo "branch=$(git branch --show-current)"; echo "source_commit=$(git rev-parse HEAD)"; echo "host=$(hostname)"; echo "kernel=$(uname -srmo)"; echo "cpu_model=$(awk -F: '/model name/ {gsub(/^ /, "", $2); print $2; exit}' /proc/cpuinfo)"; echo "affinity=$affinity"; echo "interval_seconds=$interval"; echo "duration_seconds=$duration"; echo "long_duration_seconds=$long_duration"; echo "requests=$requests"; echo "cycles=$cycles"; echo "redis_image=$redis_image"; echo "hazelcast_image=$hazelcast_image"; echo "hazelcast_client_version=$hazelcast_client_version"; echo "metrics=RSS,HWM,smaps,cgroup,CPU,latency,errors,PSI,faults,IO,network,context_switches,JVM_heap"
  echo "logical_cpus=$(nproc)"; echo "runner_receipt=/var/lib/hydracache-perf/runner-provisioned.json"; if [[ -r /var/lib/hydracache-perf/runner-provisioned.json ]]; then echo "runner_receipt_sha256=$(sha256sum /var/lib/hydracache-perf/runner-provisioned.json | cut -d' ' -f1)"; else echo "runner_receipt_sha256=unavailable"; fi; echo "docker_version=$(docker --version 2>&1 | head -n 1)"; echo "redis_benchmark_version=$($benchmark --version 2>&1 | head -n 1)"; echo "source_status=$(git status --porcelain=v1 --untracked-files=all | tr '\n' ';')"
} >"$output_dir/reproduction-command.txt"
for generated_evidence in target/test-evidence/0.67 target/test-evidence/0.67.1; do [[ -e "$generated_evidence" && ! -L "$generated_evidence" ]] && rm -rf -- "$generated_evidence"; done
if ! scripts/perf/reference-evidence-tmpfs.sh verify >>"$output_dir/hardware-validation.txt" 2>&1; then rm -f -- target/test-evidence/0.67 target/test-evidence/0.67.1; rm -rf -- /dev/shm/hydracache-reference-evidence-v1; scripts/perf/reference-evidence-tmpfs.sh prepare >>"$output_dir/hardware-validation.txt" 2>&1; fi
scripts/perf/reference-runtime-irq-guard.sh metric-expansion-pre >>"$output_dir/hardware-validation.txt"
printf 'experiment\ttarget\tcase\tstatus\tdetail\n' >"$output_dir/case-status.tsv"
trap 'stop_target || true' EXIT INT TERM

# 01 long retention: continuous bounded SET/GET.
for target in hydra redis hazelcast; do run_case 01-long-soak "$target" baseline 256 10 10 "$requests" 10000 16 uniform 0 default "" long 0 "$long_duration" 0; done
# 02 TTL variants; Hazelcast native expiry is intentionally not substituted.
printf '02-ttl\thazelcast\tnative-expiry\tnot_applicable\tHazelcast case has no Redis-compatible TTL control\n' >>"$output_dir/case-status.tsv"
for target in hydra redis; do for ttl in 100 1000 10000 60000; do run_case 02-ttl "$target" "ttl-${ttl}ms" 256 10 1 1000 10000 16 uniform "$ttl" default "" ttl 0 "$duration" 0; done; done
# 03 payload and key-length amplification.
for target in hydra redis hazelcast; do for payload in 64 1024 4096; do for keylength in 8 32; do run_case 03-payload-key "$target" "payload-${payload}-key-${keylength}" "$payload" 10 1 "$requests" 10000 "$keylength" uniform 0 default "" one 0 "$duration" 0; done; done; done
# 04 clients/pipeline scaling.
for target in hydra redis hazelcast; do for pair in 1:1 10:1 10:10 50:10 100:10; do clients="${pair%%:*}"; pipeline="${pair##*:}"; run_case 04-clients-pipeline "$target" "clients-${clients}-pipeline-${pipeline}" 256 "$clients" "$pipeline" "$requests" 10000 16 uniform 0 default "" one 0 "$duration" 0; done; done
# 05 SET/GET mix.
for target in hydra redis hazelcast; do for pct in 100 90 50 10; do run_case 05-workload-mix "$target" "set-${pct}" 256 10 10 "$requests" 10000 16 uniform 0 default "" mix "$pct" "$duration" 0; done; done
# 06 uniform/hot/Zipf-like skew.
for target in hydra redis hazelcast; do for distribution in uniform hot zipf; do run_case 06-key-distribution "$target" "$distribution" 256 10 10 "$requests" 10000 16 "$distribution" 0 default "" one 0 "$duration" 0; done; done
# 07 persistence controls.
run_case 07-persistence hydra storage 256 10 1 "$requests" 10000 16 uniform 0 default "" one 0 "$duration" 0
for mode in ephemeral rdb aof; do run_case 07-persistence redis "$mode" 256 10 1 "$requests" 10000 16 uniform 0 "$mode" "" one 0 "$duration" 0; done
run_case 07-persistence hazelcast baseline 256 10 1 "$requests" 10000 16 uniform 0 default "" one 0 "$duration" 0
# 08 allocator environment A/B (recorded explicitly; no default is changed).
for mode in default allocator-trim; do run_case 08-allocator hydra "$mode" 256 10 1 "$requests" 10000 16 uniform 0 "$mode" "" allocator 0 "$duration" 0; done
# 09 cgroup pressure controls for container targets.
for target in redis hazelcast; do for limit in 256m 512m; do run_case 09-memory-pressure "$target" "limit-${limit}" 256 10 10 "$requests" 10000 16 uniform 0 default "$limit" pressure 0 "$duration" 0; done; done
# 10 Hazelcast JVM probe. If jcmd is unavailable, heap fields remain explicit N/A.
run_case 10-hazelcast-jvm hazelcast jvm-probe 256 10 10 "$requests" 10000 16 uniform 0 default "" jvm 0 90 1

scripts/perf/reference-runtime-irq-guard.sh metric-expansion-post >>"$output_dir/hardware-validation.txt" || true
python3 "$repo_root/scripts/perf/render-metric-expansion-report.py" --input "$output_dir" --output "$output_dir/report.md" --analysis "$output_dir/analysis.md"
echo "output=$output_dir"
