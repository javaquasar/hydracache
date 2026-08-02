#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

# Ten exploratory memory investigations.  This script is deliberately separate
# from qualification/bootstrap runners: every case starts a fresh target and
# writes raw telemetry, workload logs, metadata, and a machine-readable status.
repo_root="$(git rev-parse --show-toplevel)"
output_dir="${1-/dev/shm/hydracache-memory-investigations-$(date -u +%Y%m%dT%H%M%SZ)}"
benchmark="${REDIS_BENCHMARK-/usr/bin/redis-benchmark}"
redis_image="${REDIS_IMAGE-redis@sha256:3aaec283e6e593bde528077d60280ac1589887067a39273348860837c9346d7e}"
hazelcast_image="${HAZELCAST_IMAGE-}"
hazelcast_client_python="${HAZELCAST_CLIENT_PYTHON-python3}"
hazelcast_client_version="${HAZELCAST_CLIENT_VERSION-5.5.0}"
affinity="${MEASUREMENT_AFFINITY-4}"
interval="${TELEMETRY_INTERVAL_SECONDS-1}"
requests="${MEMORY_REQUESTS-20000}"
idle_seconds="${MEMORY_IDLE_SECONDS-30}"
small_requests="${MEMORY_SMALL_REQUESTS-10000}"
branch_name="$(git branch --show-current)"
[[ -n "$branch_name" ]] || branch_name="detached@$(git rev-parse --short=12 HEAD)"
hydra_binary="$repo_root/target/release/hydracache-server"
active_target=""
active_container=""
active_pid=""

mkdir -p "$output_dir" "$output_dir/experiments" "$output_dir/metadata"
export MEASUREMENT_AFFINITY="$affinity"

require_tools() {
  test -x "$benchmark" || { echo "redis-benchmark unavailable: $benchmark" >&2; exit 2; }
  test -x "$(command -v redis-cli)" || { echo "redis-cli unavailable" >&2; exit 2; }
  test -x "$hydra_binary" || { echo "release binary unavailable: $hydra_binary" >&2; exit 2; }
  test -x "$(command -v taskset)" || { echo "taskset unavailable" >&2; exit 2; }
  test -n "$hazelcast_image" && [[ "$hazelcast_image" =~ @sha256:[0-9a-fA-F]{64}$ ]] || {
    echo 'HAZELCAST_IMAGE must be a full digest (tag@sha256:...)' >&2; exit 2;
  }
  "$hazelcast_client_python" -c 'import hazelcast' || {
    echo 'hazelcast-python-client is unavailable' >&2; exit 2;
  }
  "$hazelcast_client_python" -c "import importlib.metadata as m; assert m.version('hazelcast-python-client') == '$hazelcast_client_version'" || {
    echo "hazelcast-python-client must be exactly $hazelcast_client_version" >&2; exit 2;
  }
}

write_root_metadata() {
  {
    echo "stage=memory-investigations"
    echo "branch=$branch_name"
    echo "source_commit=$(git rev-parse HEAD)"
    echo "source_status=$(git status --porcelain=v1 --untracked-files=all | tr '\n' ';')"
    echo "host=$(hostname)"
    echo "kernel=$(uname -srmo)"
    echo "cpu_model=$(awk -F: '/model name/ {gsub(/^ /, "", $2); print $2; exit}' /proc/cpuinfo)"
    echo "logical_cpus=$(nproc)"
    echo "measurement_affinity=$affinity"
    echo "telemetry_interval_seconds=$interval"
    echo "requests_per_workload=$requests"
    echo "idle_seconds=$idle_seconds"
    echo "redis_image=$redis_image"
    echo "hazelcast_image=$hazelcast_image"
    echo "hazelcast_client_version=$hazelcast_client_version"
    echo "redis_benchmark=$benchmark"
    echo "redis_benchmark_version=$($benchmark --version 2>&1 | head -n 1)"
    if [[ -f /var/lib/hydracache-perf/runner-provisioned.json ]]; then
      echo "runner_receipt=/var/lib/hydracache-perf/runner-provisioned.json"
      echo "runner_receipt_sha256=$(sha256sum /var/lib/hydracache-perf/runner-provisioned.json | cut -d' ' -f1)"
    fi
  } >"$output_dir/reproduction-command.txt"
  # The reset server may have ordinary generated evidence directories. Remove
  # only these known generated paths, then recreate the pinned tmpfs aliases.
  for generated_evidence in target/test-evidence/0.67 target/test-evidence/0.67.1; do
    if [[ -e "$generated_evidence" && ! -L "$generated_evidence" ]]; then rm -rf -- "$generated_evidence"; fi
  done
  if ! scripts/perf/reference-evidence-tmpfs.sh verify >"$output_dir/reference-evidence-validation.txt" 2>&1; then
    scripts/perf/reference-evidence-tmpfs.sh prepare >"$output_dir/reference-evidence-preparation.txt" 2>&1
  fi
  cat "$output_dir/reference-evidence-validation.txt" "$output_dir/reference-evidence-preparation.txt" 2>/dev/null >>"$output_dir/hardware-validation.txt" || true
  scripts/perf/reference-runtime-irq-guard.sh memory-investigations-pre >>"$output_dir/hardware-validation.txt"
  docker version >"$output_dir/metadata/docker-version.txt" 2>&1 || true
}

stop_target() {
  local target="${1-${active_target}}"
  set +e
  if [[ "$target" == hydra && -n "$active_pid" ]]; then
    kill "$active_pid" 2>/dev/null || true
    wait "$active_pid" 2>/dev/null || true
  elif [[ -n "$active_container" ]]; then
    docker rm -f "$active_container" >/dev/null 2>&1 || true
  fi
  active_target=""; active_container=""; active_pid=""
  set -e
}

wait_for_resp() {
  local port="$1"
  for _ in $(seq 1 100); do
    if printf '*1\r\n$4\r\nping\r\n' | nc -w1 127.0.0.1 "$port" 2>/dev/null | grep -q PONG; then return 0; fi
    sleep .2
  done
  return 1
}

wait_for_hazelcast() {
  for _ in $(seq 1 120); do
    if "$hazelcast_client_python" -c 'import hazelcast; c=hazelcast.HazelcastClient(cluster_members=["127.0.0.1:5701"]); c.cluster_service.get_members(); c.shutdown()' >/dev/null 2>&1; then return 0; fi
    sleep 1
  done
  return 1
}

start_hydra() {
  local dir="$1" mode="${2-default}" admin=true
  [[ "$mode" == hydra-admin-off ]] && admin=false
  rm -rf "$dir/hydra-data"; mkdir -p "$dir/hydra-data"
  nohup taskset --cpu-list "$affinity" env \
    HYDRACACHE_ROLE=local HYDRACACHE_LISTEN_ADDR=127.0.0.1:0 HYDRACACHE_CLUSTER_ADDR=127.0.0.1:0 \
    HYDRACACHE_STORAGE_DIR="$dir/hydra-data" HYDRACACHE_ADMIN_API_ENABLED="$admin" \
    HYDRACACHE_ADMIN_ADDR=127.0.0.1:6390 HYDRACACHE_REDIS_API_ENABLED=true HYDRACACHE_REDIS_ADDR=127.0.0.1:6380 \
    "$hydra_binary" >"$dir/target.log" 2>&1 &
  active_pid=$!
  active_target=hydra
  echo "$active_pid" >"$dir/target.pid"
  wait_for_resp 6380
  taskset --cpu-list --pid "$active_pid" >"$dir/effective-affinity.txt"
}

start_redis() {
  local dir="$1" mode="${2-ephemeral}" name="memory-investigation-redis-$$"
  local args=(redis-server --save "" --appendonly no)
  mkdir -p "$dir/redis-data"
  case "$mode" in
    rdb) args=(redis-server --save '1 1' --appendonly no --dir /data --dbfilename dump.rdb) ;;
    aof) args=(redis-server --save "" --appendonly yes --appendfsync everysec --dir /data) ;;
  esac
  docker run -d --name "$name" --network host --cpuset-cpus "$affinity" \
    -v "$dir/redis-data:/data" "$redis_image" "${args[@]}" >"$dir/container-id.txt" 2>"$dir/docker.log"
  active_container="$name"; active_target=redis
  docker inspect "$name" >"$dir/container.inspect.json"
  wait_for_resp 6379
}

start_hazelcast() {
  local dir="$1" name="memory-investigation-hazelcast-$$"
  docker run -d --name "$name" --network host --cpuset-cpus "$affinity" "$hazelcast_image" >"$dir/container-id.txt" 2>"$dir/docker.log"
  active_container="$name"; active_target=hazelcast
  docker inspect "$name" >"$dir/container.inspect.json"
  wait_for_hazelcast
}

start_target() {
  local target="$1" dir="$2" mode="${3-default}"
  case "$target" in
    hydra) start_hydra "$dir" "$mode" ;;
    redis) start_redis "$dir" "$mode" ;;
    hazelcast) start_hazelcast "$dir" ;;
    *) echo "unknown target=$target" >&2; return 1 ;;
  esac
}

target_pid() {
  if [[ "$1" == hydra ]]; then echo "$active_pid"; else docker inspect --format '{{.State.Pid}}' "$active_container"; fi
}

run_workload() {
  local target="$1" op="$2" payload="$3" clients="$4" pipeline="$5" count="$6" keyrange="$7"
  if [[ "$target" == hydra || "$target" == redis ]]; then
    local port=6379; [[ "$target" == hydra ]] && port=6380
    taskset --cpu-list "$affinity" "$benchmark" -h 127.0.0.1 -p "$port" -n "$count" -c "$clients" -P "$pipeline" -d "$payload" -r "$keyrange" -t "$op" -q
  else
    taskset --cpu-list "$affinity" "$hazelcast_client_python" scripts/perf/hazelcast-workload.py \
      --host 127.0.0.1 --port 5701 --payload "$payload" --clients "$clients" --pipeline "$pipeline" \
      --requests "$count" --key-range "$keyrange" --operation "$op"
  fi
}

run_resp() {
  local target="$1" command="$2" port=6379
  [[ "$target" == hydra ]] && port=6380
  redis-cli -h 127.0.0.1 -p "$port" --raw $command
}

summarize_case() {
  local dir="$1"
  if compgen -G "$dir/telemetry/*.jsonl" >/dev/null; then
    python3 scripts/perf/summarize-telemetry.py --input "$dir/telemetry" --output "$dir/telemetry-summary.json" || true
  fi
}

record_status() {
  local exp="$1" target="$2" case_id="$3" status="$4" detail="${5-}"
  printf '%s\t%s\t%s\t%s\t%s\n' "$exp" "$target" "$case_id" "$status" "$detail" >>"$output_dir/case-status.tsv"
}

run_case() {
  local exp="$1" target="$2" case_id="$3" payload="$4" clients="$5" pipeline="$6" count="$7" keyrange="$8" mode="${9-default}" kind="${10-workload}"
  local dir="$output_dir/experiments/$exp/$target/$case_id"
  mkdir -p "$dir/telemetry" "$dir/raw"
  {
    echo "experiment=$exp"; echo "target=$target"; echo "case=$case_id"; echo "payload_bytes=$payload"
    echo "clients=$clients"; echo "pipeline=$pipeline"; echo "requests=$count"; echo "key_range=$keyrange"
    echo "mode=$mode"; echo "kind=$kind"; echo "affinity=$affinity"; echo "interval_seconds=$interval"
  } >"$dir/case-metadata.txt"
  if ! start_target "$target" "$dir" "$mode"; then
    echo "start_failed" >"$dir/status.txt"; record_status "$exp" "$target" "$case_id" failed start_failed; stop_target "$target"; return 0
  fi
  local pid collector workload_status=0 telemetry="$dir/telemetry/telemetry.jsonl"
  pid="$(target_pid "$target")"
  local collector_args=(--target "$target" --output "$telemetry" --interval "$interval" --duration "$((idle_seconds + 120))")
  if [[ "$target" == hydra ]]; then collector_args+=(--pid "$pid"); else collector_args+=(--container "$active_container"); fi
  python3 scripts/perf/collect-target-telemetry.py "${collector_args[@]}" >"$dir/collector.log" 2>&1 &
  collector=$!
  set +e
  if [[ "$kind" == idle ]]; then
    sleep "$idle_seconds"
  elif [[ "$kind" == workload ]]; then
    run_workload "$target" set "$payload" "$clients" "$pipeline" "$count" "$keyrange" >"$dir/raw/set.log" 2>&1
    workload_status=$?
    run_workload "$target" get "$payload" "$clients" "$pipeline" "$count" "$keyrange" >"$dir/raw/get.log" 2>&1
    [[ "$workload_status" -eq 0 ]] || true
  fi
  set -e
  kill -TERM "$collector" 2>/dev/null || true; wait "$collector" 2>/dev/null || true
  summarize_case "$dir"
  docker inspect "$active_container" >"$dir/container.inspect.final.json" 2>/dev/null || true
  stop_target "$target"
  if [[ "$workload_status" -eq 0 ]]; then echo complete >"$dir/status.txt"; record_status "$exp" "$target" "$case_id" complete "$dir"; else echo workload_failed >"$dir/status.txt"; record_status "$exp" "$target" "$case_id" failed workload_$workload_status; fi
}

run_ttl_case() {
  local target="$1" exp=06-ttl case_id=ttl-10k dir
  dir="$output_dir/experiments/$exp/$target/$case_id"
  mkdir -p "$dir/telemetry" "$dir/raw"
  if [[ "$target" == hazelcast ]]; then echo not_applicable >"$dir/status.txt"; record_status "$exp" "$target" "$case_id" not_applicable ttl_requires_native_expiry; return; fi
  start_target "$target" "$dir" default || { record_status "$exp" "$target" "$case_id" failed start_failed; stop_target "$target"; return; }
  local port=6379; [[ "$target" == hydra ]] && port=6380
  local pid collector; pid="$(target_pid "$target")"
  python3 scripts/perf/collect-target-telemetry.py --target "$target" --output "$dir/telemetry/telemetry.jsonl" --pid "$pid" --interval "$interval" --duration 90 >"$dir/collector.log" 2>&1 & collector=$!
  set +e
  for i in $(seq 1 10000); do redis-cli -h 127.0.0.1 -p "$port" SET "ttl-$i" x PX 1000 >/dev/null; done
  redis-cli -h 127.0.0.1 -p "$port" DBSIZE >"$dir/raw/dbsize-before.txt" 2>&1
  sleep 5
  redis-cli -h 127.0.0.1 -p "$port" DBSIZE >"$dir/raw/dbsize-after.txt" 2>&1
  set -e
  kill -TERM "$collector" 2>/dev/null || true; wait "$collector" 2>/dev/null || true
  summarize_case "$dir"; stop_target "$target"; echo complete >"$dir/status.txt"; record_status "$exp" "$target" ttl-10k complete "expiry-residual-recorded"
}

run_mix_case() {
  local target="$1" exp=07-workload-mix case_id="mix-${2}set" pct="$2" dir
  dir="$output_dir/experiments/$exp/$target/$case_id"
  mkdir -p "$dir/telemetry" "$dir/raw"
  start_target "$target" "$dir" default || { record_status "$exp" "$target" "$case_id" failed start_failed; stop_target "$target"; return; }
  local pid collector; pid="$(target_pid "$target")"
  python3 scripts/perf/collect-target-telemetry.py --target "$target" --output "$dir/telemetry/telemetry.jsonl" --pid "$pid" --interval "$interval" --duration 90 >"$dir/collector.log" 2>&1 & collector=$!
  local set_count get_count
  set_count=$((requests * pct / 100))
  get_count=$((requests - set_count))
  set +e
  run_workload "$target" set 256 10 1 "$set_count" 10000 >"$dir/raw/set.log" 2>&1
  run_workload "$target" get 256 10 1 "$get_count" 10000 >"$dir/raw/get.log" 2>&1
  set -e
  kill -TERM "$collector" 2>/dev/null || true; wait "$collector" 2>/dev/null || true
  summarize_case "$dir"; stop_target "$target"; echo complete >"$dir/status.txt"; record_status "$exp" "$target" "$case_id" complete "set_percent=$pct"
}

run_restart_case() {
  local target="$1" exp=09-restart case_id=restart-durability dir
  dir="$output_dir/experiments/$exp/$target/$case_id"
  mkdir -p "$dir/telemetry" "$dir/raw"
  if [[ "$target" == hazelcast ]]; then echo not_applicable >"$dir/status.txt"; record_status "$exp" "$target" "$case_id" not_applicable restart_semantics_not_comparable; return; fi
  start_target "$target" "$dir" default || { record_status "$exp" "$target" "$case_id" failed start_failed; stop_target "$target"; return; }
  local port=6379; [[ "$target" == hydra ]] && port=6380
  set +e
  for i in $(seq 1 100); do redis-cli -h 127.0.0.1 -p "$port" SET "restart-$i" value >/dev/null; done
  redis-cli -h 127.0.0.1 -p "$port" DBSIZE >"$dir/raw/dbsize-before.txt" 2>&1
  stop_target "$target"
  start_target "$target" "$dir" default
  redis-cli -h 127.0.0.1 -p "$port" DBSIZE >"$dir/raw/dbsize-after.txt" 2>&1
  set -e
  stop_target "$target"; echo complete >"$dir/status.txt"; record_status "$exp" "$target" "$case_id" complete "restart-observation-recorded"
}

run_all() {
  # 01 cold-start / idle footprint
  for target in hydra redis hazelcast; do run_case 01-cold-start "$target" cold-idle 256 1 1 1 100 default idle; done
  # 02 keyspace scaling
  for target in hydra redis hazelcast; do for keys in 1000 10000 50000; do run_case 02-keyspace "$target" "keys-$keys" 256 10 1 "$small_requests" "$keys" default workload; done; done
  # 03 fixed versus random key range
  for target in hydra redis hazelcast; do run_case 03-fixed-vs-random "$target" fixed-keyrange 256 10 1 "$small_requests" 100 default workload; run_case 03-fixed-vs-random "$target" random-keyrange 256 10 1 "$small_requests" 50000 default workload; done
  # 04 persistence/storage modes
  run_case 04-persistence hydra storage-on 256 10 1 "$small_requests" 10000 default workload
  for mode in ephemeral rdb aof; do run_case 04-persistence redis "storage-$mode" 256 10 1 "$small_requests" 10000 "$mode" workload; done
  run_case 04-persistence hazelcast default 256 10 1 "$small_requests" 10000 default workload
  # 05 feature ablation
  run_case 05-feature-ablation hydra admin-on 256 10 1 "$small_requests" 10000 default workload
  run_case 05-feature-ablation hydra admin-off 256 10 1 "$small_requests" 10000 hydra-admin-off workload
  for target in redis hazelcast; do run_case 05-feature-ablation "$target" baseline 256 10 1 "$small_requests" 10000 default workload; done
  # 06 expiry/residual memory
  for target in hydra redis hazelcast; do run_ttl_case "$target"; done
  # 07 SET/GET mix
  for target in hydra redis hazelcast; do for pct in 100 90 50 10; do run_mix_case "$target" "$pct"; done; done
  # 08 concurrency scaling
  for target in hydra redis hazelcast; do for clients in 1 10 50 100; do run_case 08-concurrency "$target" "clients-$clients" 256 "$clients" 10 "$small_requests" 10000 default workload; done; done
  # 09 restart/durability observation
  for target in hydra redis hazelcast; do run_restart_case "$target"; done
  # 10 payload scaling
  for target in hydra redis hazelcast; do for payload in 64 256 1024 4096; do run_case 10-payload "$target" "payload-$payload" "$payload" 10 1 "$small_requests" 10000 default workload; done; done
}

require_tools
write_root_metadata
printf 'experiment\ttarget\tcase\tstatus\tdetail\n' >"$output_dir/case-status.tsv"
trap 'stop_target || true' EXIT INT TERM
run_all
scripts/perf/reference-runtime-irq-guard.sh memory-investigations-post >>"$output_dir/hardware-validation.txt" || true
python3 scripts/perf/render-memory-investigation-report.py --input "$output_dir" --output "$output_dir/report.md" --source-root "$repo_root"
echo "output=$output_dir"
