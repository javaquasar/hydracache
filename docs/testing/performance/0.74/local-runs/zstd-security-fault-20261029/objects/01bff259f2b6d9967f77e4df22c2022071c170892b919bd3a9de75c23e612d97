#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

# Exploratory only. This script deliberately does not write target/test-evidence
# or alter any qualification receipt. It compares the same RESP workload against
# HydraCache and Redis on one selected reference host.

repo_root="$(git rev-parse --show-toplevel)"
output_dir="${1:-/tmp/hydracache-relative-eight-cases}"
benchmark="${REDIS_BENCHMARK:-/opt/actions-runner/_work/_temp/hydracache-perf-tools/redis-7.2.5/src/redis-benchmark}"
redis_image="${REDIS_IMAGE:-redis@sha256:3aaec283e6e593bde528077d60280ac1589887067a39273348860837c9346d7e}"
hydra_binary="$repo_root/target/release/hydracache-server"
requests="${REQUESTS_PER_CASE:-100000}"
repeats="${REPEATS:-3}"

test "$(id --user --name)" = github-runner
test -x "$benchmark"
test -x "$hydra_binary"
test "$(( $(nproc) >= 4 ))" = 1

mkdir -p "$output_dir/raw"
report="$output_dir/relative-eight-cases.txt"
hardware="$output_dir/hardware-validation.txt"

cleanup() {
  set +e
  if test -f "$output_dir/hydra.pid"; then
    kill "$(cat "$output_dir/hydra.pid")" 2>/dev/null || true
    wait "$(cat "$output_dir/hydra.pid")" 2>/dev/null || true
  fi
  docker rm -f hydracache-relative-redis >/dev/null 2>&1 || true
}
trap cleanup EXIT

scripts/perf/reference-evidence-tmpfs.sh verify >"$hardware"
scripts/perf/reference-runtime-irq-guard.sh relative-eight-pre >>"$hardware"
{
  echo "host=$(hostname)"
  echo "source_commit=$(git rev-parse HEAD)"
  echo "source_status=$(git status --porcelain=v1 --untracked-files=all | tr '\n' ';')"
  echo "kernel=$(uname -srmo)"
  echo "cpu_model=$(awk -F: '/model name/ {gsub(/^ /, "", $2); print $2; exit}' /proc/cpuinfo)"
  echo "logical_cpus=$(nproc)"
  echo "measurement_affinity=1-4"
  echo "runner_receipt_sha256=$(sha256sum /var/lib/hydracache-perf/runner-provisioned.json | cut -d' ' -f1)"
  echo "runner_receipt=/var/lib/hydracache-perf/runner-provisioned.json"
  echo "hardware_validation=tmpfs-verify; reference-runtime-irq-guard; runner-provisioned-receipt-sha256"
} >>"$hardware"

docker rm -f hydracache-relative-redis >/dev/null 2>&1 || true
docker_warning="$output_dir/docker-cpuset-warning.txt"
docker run -d --name hydracache-relative-redis --network host --cpuset-cpus 1-4 \
  "$redis_image" redis-server --save "" --appendonly no \
  >"$output_dir/redis.container-id" 2>"$docker_warning"

rm -rf "$output_dir/hydra-data"
mkdir -p "$output_dir/hydra-data"
nohup taskset --cpu-list 1-4 env \
  HYDRACACHE_ROLE=local \
  HYDRACACHE_LISTEN_ADDR=127.0.0.1:0 \
  HYDRACACHE_CLUSTER_ADDR=127.0.0.1:0 \
  HYDRACACHE_STORAGE_DIR="$output_dir/hydra-data" \
  HYDRACACHE_ADMIN_API_ENABLED=true \
  HYDRACACHE_ADMIN_ADDR=127.0.0.1:6390 \
  HYDRACACHE_REDIS_API_ENABLED=true \
  HYDRACACHE_REDIS_ADDR=127.0.0.1:6380 \
  "$hydra_binary" >"$output_dir/hydra.log" 2>&1 &
echo $! >"$output_dir/hydra.pid"
for _ in $(seq 1 100); do
  if printf '*1\r\n$4\r\nping\r\n' | nc -w1 127.0.0.1 6380 | grep -q PONG; then
    break
  fi
  sleep .2
done
grep -q PONG < <(printf '*1\r\n$4\r\nping\r\n' | nc -w1 127.0.0.1 6380)

{
  echo "methodology=matched-loopback-resp; same-host; fixed-hydra-then-redis-order; exploratory-only"
  if grep -qi cpuset "$docker_warning"; then
    echo "docker_cpuset_warning=$(tr '\n' ' ' <"$docker_warning")"
  else
    echo "docker_cpuset_warning=none"
  fi
  echo "redis_image=$redis_image"
  echo "benchmark=$benchmark"
  echo "payloads=64,256,1024"
  echo "clients=1,10,50,100"
  echo "pipelines=1,10"
  echo "operations=SET,GET"
  echo "key_range=10000"
  echo "requests_per_case=$requests"
  echo "repeats=$repeats"
  echo "case_order=p64-c10-p1,p64-c10-p10,p256-c10-p1,p256-c10-p10,p1024-c50-p1,p1024-c50-p10,p256-c1-p1,p256-c100-p1"
} >"$report"

cases=(
  "p64-c10-p1 64 10 1"
  "p64-c10-p10 64 10 10"
  "p256-c10-p1 256 10 1"
  "p256-c10-p10 256 10 10"
  "p1024-c50-p1 1024 50 1"
  "p1024-c50-p10 1024 50 10"
  "p256-c1-p1 256 1 1"
  "p256-c100-p1 256 100 1"
)

for repeat in $(seq 1 "$repeats"); do
  for spec in "${cases[@]}"; do
    IFS=' ' read -r id payload clients pipeline <<<"$spec"
    raw="$output_dir/raw/repeat-${repeat}-${id}.txt"
    {
      echo "=== repeat=$repeat case=$id payload=$payload clients=$clients pipeline=$pipeline ==="
      echo "-- system=hydra port=6380 --"
      taskset --cpu-list 1-4 "$benchmark" -h 127.0.0.1 -p 6380 \
        -n "$requests" -c "$clients" -P "$pipeline" -d "$payload" -r 10000 -t set,get -q
      echo "-- system=redis port=6379 --"
      taskset --cpu-list 1-4 "$benchmark" -h 127.0.0.1 -p 6379 \
        -n "$requests" -c "$clients" -P "$pipeline" -d "$payload" -r 10000 -t set,get -q
    } 2>&1 | tee "$raw" >>"$report"
  done
done

scripts/perf/reference-runtime-irq-guard.sh relative-eight-post >>"$hardware"
echo "report=$report"
echo "hardware=$hardware"
