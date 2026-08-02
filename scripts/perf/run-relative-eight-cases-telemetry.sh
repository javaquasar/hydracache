#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'
repo_root="$(git rev-parse --show-toplevel)"
output_dir="${1-/tmp/hydracache-relative-eight-cases-telemetry}"
benchmark="${REDIS_BENCHMARK-/usr/bin/redis-benchmark}"
redis_image="${REDIS_IMAGE-redis@sha256:3aaec283e6e593bde528077d60280ac1589887067a39273348860837c9346d7e}"
hazelcast_image="${HAZELCAST_IMAGE-}"
hazelcast_client_python="${HAZELCAST_CLIENT_PYTHON-python3}"
hazelcast_client_version="${HAZELCAST_CLIENT_VERSION-5.5.0}"
affinity="${MEASUREMENT_AFFINITY-1-4}"
requests="${REQUESTS_PER_CASE-100000}"
repeats="${REPEATS-3}"
interval="${TELEMETRY_INTERVAL_SECONDS-1}"
hydra_binary="$repo_root/target/release/hydracache-server"
test "$(id --user --name)" = github-runner
test -x "$benchmark" || { echo "redis-benchmark is unavailable: $benchmark (install redis-tools or set REDIS_BENCHMARK)" >&2; exit 2; }
test -x "$hydra_binary"
test -n "$hazelcast_image" && [[ "$hazelcast_image" =~ @sha256:[0-9a-fA-F]{64}$ ]] || { echo 'HAZELCAST_IMAGE must include a full sha256 digest' >&2; exit 2; }
"$hazelcast_client_python" -c 'import hazelcast' || { echo 'hazelcast-python-client is unavailable; refusing a partial run' >&2; exit 2; }
"$hazelcast_client_python" -c "import importlib.metadata as m; assert m.version('hazelcast-python-client') == '$hazelcast_client_version'" || { echo "hazelcast-python-client must be exactly $hazelcast_client_version" >&2; exit 2; }
mkdir -p "$output_dir/raw" "$output_dir/telemetry" "$output_dir/metadata"
{
  echo "branch=$(git branch --show-current)"
  echo "source_commit=$(git rev-parse HEAD)"
  echo "command=scripts/perf/run-relative-eight-cases-telemetry.sh $output_dir"
  echo "targets=hydracache,redis,hazelcast-community"
  echo "hazelcast_image=$hazelcast_image"
  echo "hazelcast_client_version=$hazelcast_client_version"
  echo "measurement_affinity=$affinity"
  echo "requests_per_case=$requests"
  echo "repeats=$repeats"
  echo "telemetry_interval_seconds=$interval"
} >"$output_dir/reproduction-command.txt"
cleanup() { set +e; test -f "$output_dir/hydra.pid" && kill "$(cat "$output_dir/hydra.pid")" 2>/dev/null || true; docker rm -f hydracache-relative-redis hydracache-relative-hazelcast >/dev/null 2>&1 || true; }
trap cleanup EXIT
scripts/perf/reference-evidence-tmpfs.sh verify >"$output_dir/hardware-validation.txt"
scripts/perf/reference-runtime-irq-guard.sh relative-eight-telemetry-pre >>"$output_dir/hardware-validation.txt"
{ echo "host=$(hostname)"; echo "source_commit=$(git rev-parse HEAD)"; echo "source_status=$(git status --porcelain=v1 --untracked-files=all | tr '\n' ';')"; echo "kernel=$(uname -srmo)"; echo "cpu_model=$(awk -F: '/model name/ {gsub(/^ /, "", $2); print $2; exit}' /proc/cpuinfo)"; echo "logical_cpus=$(nproc)"; echo "measurement_affinity=$affinity"; echo "targets=hydracache,redis,hazelcast-community"; echo "runner_receipt_sha256=$(sha256sum /var/lib/hydracache-perf/runner-provisioned.json | cut -d' ' -f1)"; echo "runner_receipt=/var/lib/hydracache-perf/runner-provisioned.json"; echo "telemetry_interval_seconds=$interval"; echo "redis_benchmark=$benchmark"; echo "redis_benchmark_version=$($benchmark --version 2>&1 | head -n 1)"; echo "hazelcast_image=$hazelcast_image"; echo "hazelcast_client=$hazelcast_client_version"; } >>"$output_dir/hardware-validation.txt"
docker run -d --name hydracache-relative-redis --network host --cpuset-cpus "$affinity" "$redis_image" redis-server --save "" --appendonly no >"$output_dir/metadata/redis.container-id" 2>"$output_dir/metadata/docker-warnings.txt"
docker run -d --name hydracache-relative-hazelcast --network host --cpuset-cpus "$affinity" "$hazelcast_image" >"$output_dir/metadata/hazelcast.container-id" 2>>"$output_dir/metadata/docker-warnings.txt"
docker inspect hydracache-relative-redis >"$output_dir/metadata/redis.inspect.json"
docker inspect hydracache-relative-hazelcast >"$output_dir/metadata/hazelcast.inspect.json"
rm -rf "$output_dir/hydra-data"; mkdir -p "$output_dir/hydra-data"
nohup taskset --cpu-list "$affinity" env HYDRACACHE_ROLE=local HYDRACACHE_LISTEN_ADDR=127.0.0.1:0 HYDRACACHE_CLUSTER_ADDR=127.0.0.1:0 HYDRACACHE_STORAGE_DIR="$output_dir/hydra-data" HYDRACACHE_ADMIN_API_ENABLED=true HYDRACACHE_ADMIN_ADDR=127.0.0.1:6390 HYDRACACHE_REDIS_API_ENABLED=true HYDRACACHE_REDIS_ADDR=127.0.0.1:6380 "$hydra_binary" >"$output_dir/hydra.log" 2>&1 &
echo $! >"$output_dir/hydra.pid"
for _ in $(seq 1 100); do printf '*1\r\n$4\r\nping\r\n' | nc -w1 127.0.0.1 6380 | grep -q PONG && break; sleep .2; done
printf '*1\r\n$4\r\nping\r\n' | nc -w1 127.0.0.1 6380 | grep -q PONG
hazelcast_ready=false
for _ in $(seq 1 120); do
  if "$hazelcast_client_python" -c 'import hazelcast; c=hazelcast.HazelcastClient(cluster_members=["127.0.0.1:5701"]); c.cluster_service.get_members(); c.shutdown()'; then hazelcast_ready=true; break; fi
  sleep 1
done
test "$hazelcast_ready" = true
scripts/perf/reference-runtime-irq-delta-guard.sh baseline "$output_dir/irq-baseline.tsv" >>"$output_dir/hardware-validation.txt"
echo "irq_guard_mode=preflight-plus-baseline-delta" >>"$output_dir/hardware-validation.txt"
run_target() {
  local target="$1" case_id="$2" payload="$3" clients="$4" pipeline="$5" op="$6" repeat="$7" stem="repeat-$7--$2--$1--$6" raw="$output_dir/raw/repeat-$7--$2--$1--$6.log" telemetry="$output_dir/telemetry/repeat-$7--$2--$1--$6.jsonl" pid="" container=""
  if [[ "$target" == hydra ]]; then pid="$(cat "$output_dir/hydra.pid")"; else container="hydracache-relative-$target"; fi
  local args=(--target "$target" --output "$telemetry" --interval "$interval"); if [[ -n "$pid" ]]; then args+=(--pid "$pid"); else args+=(--container "$container"); fi
  python3 scripts/perf/collect-target-telemetry.py "${args[@]}" >"$raw.telemetry.log" 2>&1 & local collector=$!
  set +e
  { echo "target=$target case=$case_id operation=$op repeat=$repeat"; if [[ "$target" == hydra || "$target" == redis ]]; then local port=6379; [[ "$target" == hydra ]] && port=6380; taskset --cpu-list "$affinity" "$benchmark" -h 127.0.0.1 -p "$port" -n "$requests" -c "$clients" -P "$pipeline" -d "$payload" -r 10000 -t "$op" -q; else taskset --cpu-list "$affinity" "$hazelcast_client_python" scripts/perf/hazelcast-workload.py --host 127.0.0.1 --port 5701 --payload "$payload" --clients "$clients" --pipeline "$pipeline" --requests "$requests" --operation "$op"; fi; } >"$raw" 2>&1
  local workload_status=$?
  set -e
  kill -TERM "$collector" 2>/dev/null || true; wait "$collector" 2>/dev/null || true
  return "$workload_status"
}
cases=('p64-c10-p1 64 10 1' 'p64-c10-p10 64 10 10' 'p256-c10-p1 256 10 1' 'p256-c10-p10 256 10 10' 'p1024-c50-p1 1024 50 1' 'p1024-c50-p10 1024 50 10' 'p256-c1-p1 256 1 1' 'p256-c100-p1 256 100 1')
for repeat in $(seq 1 "$repeats"); do for spec in "${cases[@]}"; do IFS=' ' read -r case_id payload clients pipeline <<<"$spec"; for op in set get; do for target in hydra redis hazelcast; do run_target "$target" "$case_id" "$payload" "$clients" "$pipeline" "$op" "$repeat"; done; done; done; done
python3 scripts/perf/summarize-telemetry.py --input "$output_dir/telemetry" --output "$output_dir/telemetry-summary.json"
scripts/perf/reference-runtime-irq-delta-guard.sh post-relative-eight-telemetry "$output_dir/irq-baseline.tsv" >>"$output_dir/hardware-validation.txt"
python3 scripts/perf/render-exploratory-report.py --input "$output_dir" --output "$output_dir/report.md" --source-root "$repo_root"
echo "output=$output_dir"
