#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

# Separate leak/soak stage.  It never writes into qualification/bootstrap
# directories and keeps checkpoint metadata beside raw one-second telemetry.
repo_root="$(git rev-parse --show-toplevel)"
output_dir="${1-/dev/shm/hydracache-memory-leak-$(date -u +%Y%m%dT%H%M%SZ)}"
benchmark="${REDIS_BENCHMARK-/usr/bin/redis-benchmark}"
redis_image="${REDIS_IMAGE-redis@sha256:3aaec283e6e593bde528077d60280ac1589887067a39273348860837c9346d7e}"
hazelcast_image="${HAZELCAST_IMAGE-}"
hazelcast_client_python="${HAZELCAST_CLIENT_PYTHON-python3}"
hazelcast_client_version="${HAZELCAST_CLIENT_VERSION-5.5.0}"
affinity="${MEASUREMENT_AFFINITY-4}"
interval="${TELEMETRY_INTERVAL_SECONDS-1}"
duration="${LEAK_DURATION_SECONDS-300}"
cycles="${LEAK_CYCLES-6}"
batch="${LEAK_BATCH_REQUESTS-10000}"
active_target=""; active_container=""; active_pid=""
mkdir -p "$output_dir/leak-experiments" "$output_dir/metadata"
export MEASUREMENT_AFFINITY="$affinity"

require_tools() {
  test -x "$benchmark"; test -x "$(command -v redis-cli)"; test -x "$repo_root/target/release/hydracache-server"; test -x "$(command -v taskset)"
  test -n "$hazelcast_image" && [[ "$hazelcast_image" =~ @sha256:[0-9a-fA-F]{64}$ ]]
  "$hazelcast_client_python" -c 'import hazelcast'
  "$hazelcast_client_python" -c "import importlib.metadata as m; assert m.version('hazelcast-python-client') == '$hazelcast_client_version'"
}

wait_resp() { local port="$1"; for _ in $(seq 1 100); do printf '*1\r\n$4\r\nping\r\n' | nc -w1 127.0.0.1 "$port" 2>/dev/null | grep -q PONG && return 0; sleep .2; done; return 1; }
wait_hz() { for _ in $(seq 1 120); do "$hazelcast_client_python" -c 'import hazelcast; c=hazelcast.HazelcastClient(cluster_members=["127.0.0.1:5701"]); c.cluster_service.get_members(); c.shutdown()' >/dev/null 2>&1 && return 0; sleep 1; done; return 1; }

stop_target() {
  set +e
  if [[ "$active_target" == hydra && -n "$active_pid" ]]; then kill "$active_pid" 2>/dev/null || true; wait "$active_pid" 2>/dev/null || true; fi
  [[ -n "$active_container" ]] && docker rm -f "$active_container" >/dev/null 2>&1 || true
  active_target=""; active_container=""; active_pid=""
  set -e
}

start_target() {
  local target="$1" dir="$2" mode="${3-default}" name="memory-leak-${target}-$$"
  case "$target" in
    hydra)
      rm -rf "$dir/hydra-data"; mkdir -p "$dir/hydra-data"
      nohup taskset --cpu-list "$affinity" env HYDRACACHE_ROLE=local HYDRACACHE_LISTEN_ADDR=127.0.0.1:0 HYDRACACHE_CLUSTER_ADDR=127.0.0.1:0 HYDRACACHE_STORAGE_DIR="$dir/hydra-data" HYDRACACHE_ADMIN_API_ENABLED=true HYDRACACHE_ADMIN_ADDR=127.0.0.1:6390 HYDRACACHE_REDIS_API_ENABLED=true HYDRACACHE_REDIS_ADDR=127.0.0.1:6380 "$repo_root/target/release/hydracache-server" >"$dir/target.log" 2>&1 &
      active_pid=$!; active_target=hydra; echo "$active_pid" >"$dir/target.pid"; wait_resp 6380;;
    redis)
      mkdir -p "$dir/redis-data"; local args=(redis-server --save "" --appendonly no)
      [[ "$mode" == rdb ]] && args=(redis-server --save '1 1' --appendonly no --dir /data --dbfilename dump.rdb)
      [[ "$mode" == aof ]] && args=(redis-server --save "" --appendonly yes --appendfsync everysec --dir /data)
      docker run -d --name "$name" --network host --cpuset-cpus "$affinity" -v "$dir/redis-data:/data" "$redis_image" "${args[@]}" >"$dir/container-id.txt" 2>"$dir/docker.log"
      active_container="$name"; active_target=redis; docker inspect "$name" >"$dir/container.inspect.json"; wait_resp 6379;;
    hazelcast)
      docker run -d --name "$name" --network host --cpuset-cpus "$affinity" "$hazelcast_image" >"$dir/container-id.txt" 2>"$dir/docker.log"
      active_container="$name"; active_target=hazelcast; docker inspect "$name" >"$dir/container.inspect.json"; wait_hz;;
    *) return 1;;
  esac
}

pid_for() { [[ "$1" == hydra ]] && echo "$active_pid" || docker inspect --format '{{.State.Pid}}' "$active_container"; }
port_for() { [[ "$1" == hydra ]] && echo 6380 || echo 6379; }
run_workload() {
  local target="$1" op="$2" count="$3" keyrange="$4" port
  if [[ "$target" == hydra || "$target" == redis ]]; then
    port="$(port_for "$target")"; taskset --cpu-list "$affinity" "$benchmark" -h 127.0.0.1 -p "$port" -n "$count" -c 10 -P 1 -d 256 -r "$keyrange" -t "$op" -q
  else
    taskset --cpu-list "$affinity" "$hazelcast_client_python" scripts/perf/hazelcast-workload.py --host 127.0.0.1 --port 5701 --payload 256 --clients 10 --pipeline 1 --requests "$count" --key-range "$keyrange" --operation "$op"
  fi
}
record_checkpoint() { printf '%s\t%s\t%s\n' "$(date -u +%s.%N)" "$1" "$2" >>"$3/checkpoints.tsv"; }

run_soak() {
  local exp="$1" target="$2" mode="$3" pattern="$4" dir
  dir="$output_dir/leak-experiments/$exp/$target"
  mkdir -p "$dir/telemetry" "$dir/raw"; printf 'timestamp\tphase\tdetail\n' >"$dir/checkpoints.tsv"
  start_target "$target" "$dir" "$mode" || { echo failed >"$dir/status.txt"; printf '%s\t%s\t%s\tfailed\n' "$exp" "$target" "$pattern" >>"$output_dir/leak-status.tsv"; stop_target; return; }
  local pid collector=""; pid="$(pid_for "$target")"
  local args=(--target "$target" --output "$dir/telemetry/telemetry.jsonl" --pid "$pid" --interval "$interval" --duration "$duration")
  [[ "$target" == hydra ]] || args=(--target "$target" --output "$dir/telemetry/telemetry.jsonl" --container "$active_container" --interval "$interval" --duration "$duration")
  python3 scripts/perf/collect-target-telemetry.py "${args[@]}" >"$dir/collector.log" 2>&1 & collector=$!
  record_checkpoint start "$pattern" "$dir"
  set +e
  case "$pattern" in
    fixed-keyspace)
      for cycle in $(seq 1 "$cycles"); do record_checkpoint load "$cycle" "$dir"; run_workload "$target" set "$batch" 10000 >"$dir/raw/set-$cycle.log" 2>&1; run_workload "$target" get "$batch" 10000 >"$dir/raw/get-$cycle.log" 2>&1; sleep $((duration / cycles)); done;;
    expiry-reclamation)
      if [[ "$target" == hazelcast ]]; then :; else local port="$(port_for "$target")"; for cycle in $(seq 1 "$cycles"); do record_checkpoint ttl-set "$cycle" "$dir"; for i in $(seq 1 10000); do redis-cli -h 127.0.0.1 -p "$port" SET "leak-ttl-$i" x PX 1000 >/dev/null; done; sleep $((duration / cycles)); record_checkpoint ttl-after "$cycle" "$dir"; done; fi;;
    cycle-reset)
      for cycle in $(seq 1 "$cycles"); do record_checkpoint load "$cycle" "$dir"; run_workload "$target" set "$batch" "$((cycle * 10000))" >"$dir/raw/set-$cycle.log" 2>&1; sleep 2; record_checkpoint reset "$cycle" "$dir"; if [[ "$target" == hydra || "$target" == redis ]]; then redis-cli -h 127.0.0.1 -p "$(port_for "$target")" FLUSHALL >/dev/null; else "$hazelcast_client_python" -c 'import hazelcast; c=hazelcast.HazelcastClient(cluster_members=["127.0.0.1:5701"]); c.get_map("exploratory-067").clear().result(); c.shutdown()'; fi; sleep $((duration / cycles)); done;;
    idle-fragmentation)
      record_checkpoint load "$batch" "$dir"; run_workload "$target" set "$batch" 10000 >"$dir/raw/set.log" 2>&1; record_checkpoint idle-start "$duration" "$dir"; sleep "$duration";;
  esac
  set -e
  kill -TERM "$collector" 2>/dev/null || true; wait "$collector" 2>/dev/null || true
  stop_target; echo complete >"$dir/status.txt"; printf '%s\t%s\t%s\tcomplete\n' "$exp" "$target" "$pattern" >>"$output_dir/leak-status.tsv"
}

run_restart_soak() {
  local exp=04-restart-soak target="$1" mode="$2" pattern=restart-soak dir
  dir="$output_dir/leak-experiments/$exp/$target"
  mkdir -p "$dir/telemetry" "$dir/raw"; printf 'timestamp\tphase\tdetail\n' >"$dir/checkpoints.tsv"
  local per_cycle=$((duration / cycles)); [[ "$per_cycle" -lt 20 ]] && per_cycle=20
  for cycle in $(seq 1 "$cycles"); do
    start_target "$target" "$dir" "$mode" || { stop_target; echo failed >"$dir/status.txt"; printf '%s\t%s\t%s\tfailed\n' "$exp" "$target" "$pattern" >>"$output_dir/leak-status.tsv"; return; }
    local pid collector; pid="$(pid_for "$target")"
    local args=(--target "$target" --output "$dir/telemetry/restart-$cycle.jsonl" --pid "$pid" --interval "$interval" --duration "$per_cycle")
    [[ "$target" == hydra ]] || args=(--target "$target" --output "$dir/telemetry/restart-$cycle.jsonl" --container "$active_container" --interval "$interval" --duration "$per_cycle")
    python3 scripts/perf/collect-target-telemetry.py "${args[@]}" >"$dir/collector-$cycle.log" 2>&1 & collector=$!
    record_checkpoint cycle-start "$cycle" "$dir"
    set +e; run_workload "$target" set "$batch" "$((cycle * 10000))" >"$dir/raw/set-$cycle.log" 2>&1; set -e
    sleep "$per_cycle"
    kill -TERM "$collector" 2>/dev/null || true; wait "$collector" 2>/dev/null || true
    record_checkpoint cycle-stop "$cycle" "$dir"; stop_target
  done
  echo complete >"$dir/status.txt"; printf '%s\t%s\t%s\tcomplete\n' "$exp" "$target" "$pattern" >>"$output_dir/leak-status.tsv"
}

require_tools
{
  echo "stage=memory-leak"; echo "branch=$(git branch --show-current)"; echo "source_commit=$(git rev-parse HEAD)"; echo "host=$(hostname)"; echo "affinity=$affinity"; echo "interval_seconds=$interval"; echo "duration_seconds=$duration"; echo "cycles=$cycles"; echo "batch_requests=$batch"; echo "redis_image=$redis_image"; echo "hazelcast_image=$hazelcast_image"; echo "hazelcast_client_version=$hazelcast_client_version"
} >"$output_dir/reproduction-command.txt"
for generated_evidence in target/test-evidence/0.67 target/test-evidence/0.67.1; do
  if [[ -e "$generated_evidence" && ! -L "$generated_evidence" ]]; then rm -rf -- "$generated_evidence"; fi
done
if ! scripts/perf/reference-evidence-tmpfs.sh verify >>"$output_dir/hardware-validation.txt" 2>&1; then
  rm -f -- target/test-evidence/0.67 target/test-evidence/0.67.1
  rm -rf -- /dev/shm/hydracache-reference-evidence-v1
  scripts/perf/reference-evidence-tmpfs.sh prepare >>"$output_dir/hardware-validation.txt" 2>&1
fi
scripts/perf/reference-runtime-irq-guard.sh memory-leak-pre >>"$output_dir/hardware-validation.txt"
printf 'experiment\ttarget\tpattern\tstatus\n' >"$output_dir/leak-status.tsv"
trap 'stop_target || true' EXIT INT TERM
for target in hydra redis hazelcast; do run_soak 01-fixed-keyspace "$target" default fixed-keyspace; done
for target in hydra redis hazelcast; do run_soak 02-expiry-reclamation "$target" default expiry-reclamation; done
for target in hydra redis hazelcast; do run_soak 03-cycle-reset "$target" default cycle-reset; done
for target in hydra redis; do run_restart_soak "$target" default; done
for target in hydra redis hazelcast; do run_soak 05-idle-fragmentation "$target" default idle-fragmentation; done
scripts/perf/reference-runtime-irq-guard.sh memory-leak-post >>"$output_dir/hardware-validation.txt" || true
python3 scripts/perf/render-memory-leak-report.py --input "$output_dir" --output "$output_dir/report.md"
echo "output=$output_dir"
