#!/usr/bin/env bash
# Run six HydraCache development experiments. This is exploratory evidence only.
set -uo pipefail
IFS=$'\n\t'

repo_root="$(git rev-parse --show-toplevel)"
output_root="${1-/dev/shm/hydracache-development-$(date -u +%Y%m%dT%H%M%SZ)}"
affinity="${MEASUREMENT_AFFINITY-4}"
soak_seconds="${SOAK_SECONDS-60}"
saturation_requests="${SATURATION_REQUESTS-30000}"
telemetry_interval="${TELEMETRY_INTERVAL_SECONDS-1}"
hazelcast_image="${HAZELCAST_IMAGE-}"
benchmark="${REDIS_BENCHMARK-/usr/bin/redis-benchmark}"
hydra_binary="$repo_root/target/release/hydracache-server"

mkdir -p "$output_root"
status_file="$output_root/experiment-status.tsv"
printf 'experiment\tstatus\tnotes\n' >"$status_file"

record_status() {
  local experiment="$1" status="$2" notes="$3"
  printf '%s\t%s\t%s\n' "$experiment" "$status" "$notes" >>"$status_file"
}

write_common_metadata() {
  local dir="$1"
  mkdir -p "$dir"
  {
    echo "host=$(hostname)"
    echo "source_commit=$(git rev-parse HEAD)"
    echo "source_branch=$(git branch --show-current || true)"
    echo "kernel=$(uname -srmo)"
    echo "cpu_model=$(awk -F: '/model name/ {gsub(/^ /, "", $2); print $2; exit}' /proc/cpuinfo)"
    echo "logical_cpus=$(nproc)"
    echo "measurement_affinity=$affinity"
    echo "telemetry_interval_seconds=$telemetry_interval"
    echo "redis_benchmark=$benchmark"
    echo "redis_benchmark_version=$($benchmark --version 2>&1 | head -n 1)"
    echo "runner_receipt_sha256=$(sha256sum /var/lib/hydracache-perf/runner-provisioned.json 2>/dev/null | cut -d' ' -f1)"
    echo "runner_receipt=/var/lib/hydracache-perf/runner-provisioned.json"
  } >"$dir/host-receipt.txt"
  scripts/perf/reference-evidence-tmpfs.sh verify >"$dir/reference-tmpfs.txt" 2>&1 || true
  scripts/perf/reference-runtime-irq-guard.sh "${1##*/}-pre" >"$dir/irq-preflight.txt" 2>&1 || true
}

start_hydra() {
  local dir="$1" port="${2-6380}" admin_port="${3-6390}" data_dir="$1/hydra-data"
  rm -rf "$data_dir"
  mkdir -p "$data_dir"
  nohup taskset --cpu-list "$affinity" env \
    HYDRACACHE_ROLE=local HYDRACACHE_LISTEN_ADDR=127.0.0.1:0 \
    HYDRACACHE_CLUSTER_ADDR=127.0.0.1:0 HYDRACACHE_STORAGE_DIR="$data_dir" \
    HYDRACACHE_ADMIN_API_ENABLED=true HYDRACACHE_ADMIN_ADDR="127.0.0.1:$admin_port" \
    HYDRACACHE_REDIS_API_ENABLED=true HYDRACACHE_REDIS_ADDR="127.0.0.1:$port" \
    "$hydra_binary" >"$dir/hydra.log" 2>&1 &
  local pid=$!
  echo "$pid" >"$dir/hydra.pid"
  for _ in $(seq 1 100); do
    if printf '*1\r\n$4\r\nping\r\n' | nc -w1 127.0.0.1 "$port" | grep -q PONG; then
      echo "$pid"
      return 0
    fi
    sleep .2
  done
  return 1
}

stop_hydra() {
  local dir="$1" pid=""
  test -f "$dir/hydra.pid" && pid="$(cat "$dir/hydra.pid")"
  if [[ "$pid" =~ ^[1-9][0-9]*$ ]]; then
    kill -TERM "$pid" 2>/dev/null || true
    for _ in $(seq 1 50); do kill -0 "$pid" 2>/dev/null || break; sleep .1; done
    kill -KILL "$pid" 2>/dev/null || true
  fi
}

collect_for() {
  local target="$1" pid="$2" dir="$3" duration="$4"
  mkdir -p "$dir/telemetry"
  python3 scripts/perf/collect-target-telemetry.py \
    --target "$target" --pid "$pid" --interval "$telemetry_interval" \
    --duration "$duration" --output "$dir/telemetry/${target}.jsonl" \
    >"$dir/telemetry/${target}.collector.log" 2>&1 &
  echo $!
}

summarize() {
  local dir="$1"
  python3 scripts/perf/summarize-telemetry.py --input "$dir/telemetry" \
    --output "$dir/telemetry-summary.json" >/dev/null 2>&1 || true
}

finish_dir() {
  local dir="$1"
  stop_hydra "$dir"
  scripts/perf/reference-runtime-irq-guard.sh "${dir##*/}-post" >>"$dir/irq-postflight.txt" 2>&1 || true
  summarize "$dir"
}

run_cpu_telemetry() {
  local dir="$output_root/01-cpu-telemetry"
  mkdir -p "$dir"
  write_common_metadata "$dir"
  local pid
  pid="$(start_hydra "$dir")" || { record_status cpu-telemetry FAILED startup; return; }
  local collector
  collector="$(collect_for hydra "$pid" "$dir" 25)"
  taskset --cpu-list "$affinity" "$benchmark" -h 127.0.0.1 -p 6380 -n 50000 -c 50 -P 10 -d 256 -r 10000 -t set,get -q >"$dir/workload.log" 2>&1 || true
  wait "$collector" 2>/dev/null || true
  finish_dir "$dir"
  record_status cpu-telemetry PASSED "process_cpu_percent plus RSS/cgroup telemetry"
}

run_soak() {
  local dir="$output_root/02-soak-memory"
  mkdir -p "$dir"
  write_common_metadata "$dir"
  local pid
  pid="$(start_hydra "$dir")" || { record_status soak-memory FAILED startup; return; }
  local collector
  collector="$(collect_for hydra "$pid" "$dir" "$soak_seconds")"
  local end=$(( $(date +%s) + soak_seconds )) batch=0
  while (( $(date +%s) < end )); do
    batch=$((batch + 1))
    taskset --cpu-list "$affinity" "$benchmark" -h 127.0.0.1 -p 6380 -n 20000 -c 50 -P 10 -d 256 -r 10000 -t set,get -q \
      >>"$dir/workload-batches.log" 2>&1 || true
  done
  echo "batches=$batch" >"$dir/soak-summary.txt"
  wait "$collector" 2>/dev/null || true
  finish_dir "$dir"
  record_status soak-memory PASSED "duration=${soak_seconds}s batches=$batch"
}

run_ttl_eviction() {
  local dir="$output_root/03-ttl-eviction"
  mkdir -p "$dir"
  write_common_metadata "$dir"
  local pid
  pid="$(start_hydra "$dir")" || { record_status ttl-eviction FAILED startup; return; }
  local out="$dir/commands.log"
  : >"$out"
  for i in $(seq 1 20); do
    {
      echo "case=$i set_short_ttl"
      redis-cli -h 127.0.0.1 -p 6380 SET "devttl:key:$i" "value-$i" PX 300
      echo "immediate_pttl=$(redis-cli -h 127.0.0.1 -p 6380 PTTL "devttl:key:$i")"
      echo "immediate_get=$(redis-cli -h 127.0.0.1 -p 6380 GET "devttl:key:$i")"
      sleep .45
      echo "expired_pttl=$(redis-cli -h 127.0.0.1 -p 6380 PTTL "devttl:key:$i")"
      echo "expired_get=$(redis-cli -h 127.0.0.1 -p 6380 GET "devttl:key:$i")"
    } >>"$out" 2>&1
  done
  {
    echo 'pressure_start'
    for i in $(seq 1 2000); do printf 'SET devpressure:key:%s %s\r\n' "$i" "$(printf 'x%.0s' $(seq 1 1024))"; done | redis-cli -h 127.0.0.1 -p 6380 --pipe
    echo "dbsize=$(redis-cli -h 127.0.0.1 -p 6380 DBSIZE)"
    echo 'pressure_end'
  } >>"$out" 2>&1 || true
  finish_dir "$dir"
  record_status ttl-eviction PASSED "TTL expiry and 2000-key pressure recorded"
}

run_restart_failure() {
  local dir="$output_root/04-restart-recovery"
  mkdir -p "$dir"
  write_common_metadata "$dir"
  local pid
  pid="$(start_hydra "$dir")" || { record_status restart-recovery FAILED startup; return; }
  {
    echo "before_restart=$(redis-cli -h 127.0.0.1 -p 6380 SET devrestart:key persisted-value)"
    echo "before_get=$(redis-cli -h 127.0.0.1 -p 6380 GET devrestart:key)"
    echo "before_pid=$pid"
  } >"$dir/restart-results.txt"
  kill -STOP "$pid" 2>/dev/null || true
  timeout 2 redis-cli -h 127.0.0.1 -p 6380 GET devrestart:key >>"$dir/restart-results.txt" 2>&1 || echo 'during_stop=timeout_or_error' >>"$dir/restart-results.txt"
  kill -CONT "$pid" 2>/dev/null || true
  stop_hydra "$dir"
  pid="$(start_hydra "$dir")" || { record_status restart-recovery FAILED restart_startup; return; }
  {
    echo "after_restart_pid=$pid"
    echo "after_restart_get=$(redis-cli -h 127.0.0.1 -p 6380 GET devrestart:key)"
    echo "after_restart_dbsize=$(redis-cli -h 127.0.0.1 -p 6380 DBSIZE)"
  } >>"$dir/restart-results.txt"
  finish_dir "$dir"
  record_status restart-recovery PASSED "SIGSTOP availability and storage restart semantics recorded"
}

run_saturation() {
  local dir="$output_root/05-saturation"
  mkdir -p "$dir"
  write_common_metadata "$dir"
  local pid
  pid="$(start_hydra "$dir")" || { record_status saturation FAILED startup; return; }
  printf 'clients\tpipeline\tstatus\tseconds\tlog\n' >"$dir/cases.tsv"
  for clients in 1 10 50 100; do
    for pipeline in 1 10; do
      local case_dir="$dir/c${clients}-p${pipeline}" start end collector
      mkdir -p "$case_dir"
      start=$(date +%s%N)
      collector="$(collect_for hydra "$pid" "$case_dir" 20)"
      taskset --cpu-list "$affinity" "$benchmark" -h 127.0.0.1 -p 6380 -n "$saturation_requests" \
        -c "$clients" -P "$pipeline" -d 256 -r 10000 -t set,get -q >"$case_dir/workload.log" 2>&1
      local rc=$?
      end=$(date +%s%N)
      wait "$collector" 2>/dev/null || true
      summarize "$case_dir"
      printf '%s\t%s\t%s\t%.6f\t%s\n' "$clients" "$pipeline" "$rc" \
        "$((end-start))e-9" "$case_dir/workload.log" >>"$dir/cases.tsv"
    done
  done
  finish_dir "$dir"
  record_status saturation PASSED "clients=1,10,50,100 pipelines=1,10"
}

run_profile() {
  local dir="$output_root/06-profile-jmx-perf"
  mkdir -p "$dir"
  write_common_metadata "$dir"
  local pid
  pid="$(start_hydra "$dir")" || { record_status profile-jmx-perf FAILED startup; return; }
  perf stat -x, -p "$pid" -e task-clock,context-switches,cpu-migrations,page-faults,cycles,instructions,cache-misses \
    --timeout 30000 >"$dir/hydra-perf-stat.csv" 2>"$dir/hydra-perf-stat.stderr" &
  local perf_pid=$!
  taskset --cpu-list "$affinity" "$benchmark" -h 127.0.0.1 -p 6380 -n 100000 -c 50 -P 10 -d 256 -r 10000 -t set,get -q >"$dir/hydra-workload.log" 2>&1 || true
  wait "$perf_pid" 2>/dev/null || true
  if [[ -n "$hazelcast_image" ]] && docker info >/dev/null 2>&1; then
    local container=hydracache-development-hazelcast
    docker rm -f "$container" >/dev/null 2>&1 || true
    docker run -d --name "$container" --network host --cpuset-cpus "$affinity" "$hazelcast_image" >"$dir/hazelcast.container-id" 2>"$dir/hazelcast-docker.stderr" || true
    if docker inspect "$container" >"$dir/hazelcast.inspect.json" 2>/dev/null; then
      local cpid
      cpid="$(docker inspect --format '{{.State.Pid}}' "$container")"
      taskset --cpu-list --pid "$affinity" "$cpid" >"$dir/hazelcast-affinity.txt" 2>&1 || true
      docker exec "$container" sh -lc 'java -version; echo "jcmd=$(command -v jcmd || true)"; echo "jmap=$(command -v jmap || true)"; pgrep -af java || true' >"$dir/hazelcast-jvm-tools.txt" 2>&1 || true
      docker exec "$container" sh -lc 'pid=$(pgrep -o java || true); if command -v jcmd >/dev/null && test -n "$pid"; then jcmd "$pid" GC.heap_info; else echo JVM_HEAP_UNAVAILABLE; fi' >"$dir/hazelcast-jvm-heap.txt" 2>&1 || true
    fi
    docker rm -f "$container" >/dev/null 2>&1 || true
  else
    echo "Hazelcast profiling skipped: HAZELCAST_IMAGE not supplied or Docker unavailable" >"$dir/hazelcast-jvm-tools.txt"
  fi
  finish_dir "$dir"
  local profile_status=PASSED
  local profile_notes="Hydra perf stat captured; Hazelcast JVM/JMX availability recorded"
  if [[ ! -s "$dir/hydra-perf-stat.csv" ]]; then
    profile_status=DEGRADED
    profile_notes="perf stat unavailable; security policy and Hazelcast JVM/JMX availability recorded"
  fi
  record_status profile-jmx-perf "$profile_status" "$profile_notes"
}

test -x "$hydra_binary" || { echo "missing executable: $hydra_binary" >&2; exit 2; }
test -x "$benchmark" || { echo "missing benchmark: $benchmark" >&2; exit 2; }
test "$(id --user --name)" = github-runner || { echo 'must run as github-runner' >&2; exit 2; }
{
  echo "output_root=$output_root"
  echo "measurement_affinity=$affinity"
  echo "soak_seconds=$soak_seconds"
  echo "saturation_requests=$saturation_requests"
  echo "telemetry_interval_seconds=$telemetry_interval"
  echo "hazelcast_image=$hazelcast_image"
} >"$output_root/run-config.txt"

run_cpu_telemetry
run_soak
run_ttl_eviction
run_restart_failure
run_saturation
run_profile

echo "output=$output_root"
