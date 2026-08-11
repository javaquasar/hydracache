#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

usage() {
  echo "usage: scripts/hc2/verify-fixed-host.sh --output PATH" >&2
  exit 2
}

output=""
while (($# > 0)); do
  case "$1" in
    --output)
      shift
      (($# > 0)) || usage
      output="$1"
      ;;
    *)
      usage
      ;;
  esac
  shift
done

[[ -n "$output" ]] || usage
mkdir -p -- "$(dirname -- "$output")"
umask 077
exec > >(tee "$output") 2>&1

fail() {
  echo "HC/2 fixed-host preflight failed: $*" >&2
  exit 1
}

os_release_value() {
  local key="$1"
  awk -F= -v key="$key" '
    $1 == key {
      value = substr($0, index($0, "=") + 1)
      gsub(/^"|"$/, "", value)
      print value
      exit
    }
  ' /etc/os-release
}

[[ "$(uname -s)" == "Linux" ]] || fail "kernel must be Linux"
[[ "$(uname -m)" == "x86_64" ]] || fail "architecture must be x86_64"
[[ -r /etc/os-release ]] || fail "/etc/os-release is unavailable"
[[ "$(os_release_value ID)" == "ubuntu" ]] || fail "distribution must be Ubuntu"
[[ "$(os_release_value VERSION_ID)" == "24.04" ]] || fail "Ubuntu version must be 24.04"
[[ "${GITHUB_ACTIONS:-}" == "true" ]] || fail "must execute inside GitHub Actions"
[[ "${RUNNER_OS:-}" == "Linux" ]] || fail "RUNNER_OS must be Linux"
[[ "${RUNNER_ARCH:-}" == "X64" ]] || fail "RUNNER_ARCH must be X64"
[[ "${HC2_FIXED_HOST_PROFILE:-}" == "hc2-fixed-soak-v1" ]] || fail "unexpected evidence profile"
[[ "${RUNNER_NAME:-}" =~ ^[A-Za-z0-9._-]{1,128}$ ]] || fail "runner name is absent or unsafe"
[[ "${GITHUB_SHA:-}" =~ ^[0-9a-f]{40}$ ]] || fail "GITHUB_SHA must be a full lowercase commit"
[[ "$(id -u)" -ne 0 ]] || fail "runner service must not execute as root"

for command_name in git cargo rustc uname lscpu awk tee; do
  command -v "$command_name" >/dev/null || fail "required command is absent: $command_name"
done

rustc_version="$(rustc --version)"
cargo_version="$(cargo --version)"
[[ "$rustc_version" == "rustc 1.94.0 "* ]] || fail "rustc must be pinned to 1.94.0"
[[ "$cargo_version" == "cargo 1.94.0 "* ]] || fail "cargo must be pinned to 1.94.0"

candidate_sha="$(git rev-parse HEAD)"
[[ "$candidate_sha" == "$GITHUB_SHA" ]] || fail "checkout does not match GITHUB_SHA"
[[ -z "$(git status --short --untracked-files=no)" ]] || fail "tracked checkout is dirty"

echo "profile=$HC2_FIXED_HOST_PROFILE"
echo "os_id=$(os_release_value ID)"
echo "os_version_id=$(os_release_value VERSION_ID)"
echo "architecture=$(uname -m)"
echo "runner_name=$RUNNER_NAME"
echo "candidate_sha=$candidate_sha"
uname -a
lscpu
git --version
rustc --version --verbose
cargo --version --verbose
echo "HC/2 fixed-host preflight passed"
