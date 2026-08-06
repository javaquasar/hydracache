#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

usage() {
  cat >&2 <<'EOF'
usage: scripts/perf/prepare-reference-host.sh \
  <irq-layout-preflight|preflight|apply-services|install-isolation|verify|freeze|check-frozen> \
  [--profile PATH] [--state-dir PATH]

This wrapper deliberately does not reboot, register/enable the GitHub runner, or
start rootless Docker. Those remain explicit lifecycle boundaries.
EOF
  exit 2
}

action="${1:-}"
case "$action" in
  irq-layout-preflight|preflight|apply-services|install-isolation|verify|freeze|check-frozen) shift ;;
  *) usage ;;
esac

repo_root="$(git rev-parse --show-toplevel)"
profile="$repo_root/docs/testing/perf-host-profiles/ubuntu-24.04-reference-v1.json"
state_dir="/var/lib/hydracache-perf/host-tuning-v1"
while test "$#" -gt 0; do
  case "$1" in
    --profile)
      test "$#" -ge 2 || usage
      profile="$2"
      shift 2
      ;;
    --state-dir)
      test "$#" -ge 2 || usage
      state_dir="$2"
      shift 2
      ;;
    *) usage ;;
  esac
done

test "$(id --user)" -eq 0 || {
  echo "reference host preparation requires root" >&2
  exit 1
}
test -f "$profile"

tuning=(
  "$repo_root/scripts/perf/reference-host-tuning.sh"
)
common=(
  --profile "$profile"
  --state-dir "$state_dir"
)
case "$action" in
  irq-layout-preflight)
    "$repo_root/scripts/perf/reference-host-irq-layout-preflight.sh" \
      --profile "$profile" \
      --output-dir "$state_dir/irq-layout-preflight"
    echo "EARLY_IRQ_LAYOUT_ELIGIBLE=true"
    ;;
  preflight)
    "${tuning[@]}" plan "${common[@]}"
    echo "PRE_FLIGHT_ONLY=true"
    ;;
  apply-services)
    "${tuning[@]}" apply "${common[@]}"
    echo "SERVICE_POLICY_APPLIED=true"
    ;;
  install-isolation)
    "$repo_root/scripts/perf/provision-reference-isolation.sh" install
    echo "REBOOT_REQUIRED=true"
    ;;
  verify)
    "${tuning[@]}" verify "${common[@]}"
    echo "REFERENCE_HOST_VERIFIED=true"
    ;;
  freeze)
    "${tuning[@]}" freeze "${common[@]}"
    echo "SAMPLE_FAMILY_FROZEN=true"
    ;;
  check-frozen)
    "$repo_root/scripts/perf/check-reference-host-freeze.sh" "${common[@]}"
    ;;
esac
