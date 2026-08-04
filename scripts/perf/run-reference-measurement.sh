#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'
export LC_ALL=C

mode="${1:-}"
case "$mode" in
  attestation|core|resp|control-plane|qualification-local|qualification-client-surface) ;;
  *)
    echo "usage: scripts/perf/run-reference-measurement.sh <attestation|core|resp|control-plane|qualification-local|qualification-client-surface>" >&2
    exit 2
    ;;
esac

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
test "$(id --user --name)" = github-runner || {
  echo "reference measurements must run as github-runner" >&2
  exit 1
}
test "$(
  awk -F: '$1 == "Cpus_allowed_list" { gsub(/[[:space:]]/, "", $2); print $2 }' /proc/self/status
)" = "0,5-7" || {
  echo "reference measurement orchestration must remain on housekeeping CPUs 0,5-7" >&2
  exit 1
}

scripts/perf/reference-evidence-tmpfs.sh verify

warm_file() {
  local path="$1"
  test -f "$path"
  dd if="$path" of=/dev/null bs=1M status=none
}

warm_executable() {
  local executable="$1"
  local library
  warm_file "$executable"
  while IFS= read -r library; do
    test -z "$library" || warm_file "$library"
  done < <(
    ldd "$executable" 2>/dev/null |
      awk '
        $2 == "=>" && $3 ~ /^\// { print $3 }
        $1 ~ /^\// { print $1 }
      '
  )
}

warm_command() {
  local command_name="$1"
  local command_path
  command_path="$(command -v "$command_name")"
  test -n "$command_path"
  warm_executable "$(readlink --canonicalize "$command_path")"
}

warm_file Cargo.lock
warm_file .git/index
warm_file docs/plans/releases.toml
warm_file docs/testing/perf-profiles/reference-v1.toml
warm_file /etc/os-release
warm_file /var/lib/hydracache-perf/runner-provisioned.json
while IFS= read -r tracked_input; do
  test -z "$tracked_input" || warm_file "$tracked_input"
done < <(
  git ls-files \
    docs/testing/perf-scenarios/0.67 \
    docs/testing/gated-test-registry.toml
)
test -z "$(git status --porcelain=v1 --untracked-files=no)"
warm_command git
warm_command taskset
warm_command findmnt
warm_command lsblk
warm_command systemd-detect-virt
if [[ "$mode" == qualification-* ]]; then
  mkdir --parents target/test-evidence/0.67.1/qualification
fi

case "$mode" in
  attestation)
    command_argv=(
      target/debug/xtask
      perf-runner-preflight
      --release
      0.67
      --profile
      reference-v1
    )
    warm_executable target/debug/xtask
    ;;
  core|resp|control-plane)
    command_argv=(
      target/release/hydracache-loadgen
      suite
      "$mode"
      --profile
      reference-v1
      --output-dir
      target/test-evidence/0.67
    )
    warm_executable target/release/hydracache-loadgen
    warm_executable target/release/hydracache-server
    warm_file target/test-evidence/0.67/prebuild-manifest.json
    warm_file target/test-evidence/0.67/resp-reference-run-inputs.json
    ;;
  qualification-local)
    command_argv=(
      target/release/hydracache-loadgen
      tier
      local
      --profile
      smoke-v1
      --report
      target/test-evidence/0.67.1/qualification/local-smoke.json
    )
    warm_executable target/release/hydracache-loadgen
    warm_file target/test-evidence/0.67/prebuild-manifest.json
    ;;
  qualification-client-surface)
    command_argv=(
      target/release/hydracache-loadgen
      tier
      client-surface
      --profile
      smoke-v1
      --report
      target/test-evidence/0.67.1/qualification/client-surface-smoke.json
    )
    warm_executable target/release/hydracache-loadgen
    warm_file target/test-evidence/0.67/prebuild-manifest.json
    ;;
esac

if test "$mode" = resp; then
  warm_command docker
  warm_command redis-benchmark
  redis_image="redis@sha256:3aaec283e6e593bde528077d60280ac1589887067a39273348860837c9346d7e"
  docker pull --platform linux/amd64 "$redis_image" >/dev/null
  docker run \
    --rm \
    --cpuset-cpus 0 \
    --platform linux/amd64 \
    "$redis_image" \
    redis-server --version >/dev/null
fi

guard_log="target/test-evidence/0.67.1/runtime-irq-guard.log"
mkdir --parents "$(dirname "$guard_log")"
pre_output="$(scripts/perf/reference-runtime-irq-guard.sh "${mode}-pre")"
printf '%s\n' "$pre_output"
printf '%s\n' "$pre_output" >>"$guard_log"

set +e
HYDRACACHE_MEASUREMENT_IO_POLICY="tmpfs-housekeeping-orchestration-v1" \
  taskset --cpu-list 1-4 "${command_argv[@]}"
measurement_status=$?
set -e

set +e
post_output="$(scripts/perf/reference-runtime-irq-guard.sh "${mode}-post" 2>&1)"
post_status=$?
set -e
printf '%s\n' "$post_output"
printf '%s\n' "$post_output" >>"$guard_log"
if test "$post_status" -ne 0; then
  exit "$post_status"
fi
exit "$measurement_status"
