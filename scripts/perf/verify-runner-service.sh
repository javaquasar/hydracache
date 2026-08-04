#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

expected_label=""
while (($# > 0)); do
  case "$1" in
    --expected-label)
      shift
      (($# > 0)) || exit 2
      expected_label="$1"
      ;;
    *)
      echo "usage: scripts/perf/verify-runner-service.sh --expected-label LABEL" >&2
      exit 2
      ;;
  esac
  shift
done

test "$expected_label" = "hydracache-perf-v1" || {
  echo "expected label must be exactly hydracache-perf-v1" >&2
  exit 1
}

command -v jq >/dev/null
command -v systemctl >/dev/null
contract_path="/etc/hydracache-perf/runner-contract.json"
runner_path="/opt/actions-runner/.runner"
test -r "$contract_path"
test -r "$runner_path"

jq --exit-status \
  --arg expected "$expected_label" '
    .schema_version == 1 and
    .repository == "javaquasar/hydracache" and
    .runner_name == "hydracache-perf-v1" and
    .labels == ["self-hosted", "linux", "x64", $expected] and
    .service_user == "github-runner"
  ' "$contract_path" >/dev/null

jq --exit-status '
  .agentName == "hydracache-perf-v1" and
  .gitHubUrl == "https://github.com/javaquasar/hydracache" and
  .workFolder == "_work"
' "$runner_path" >/dev/null

test "$(stat --format=%U /opt/actions-runner)" = "github-runner"
test "$(stat --format=%U "$runner_path")" = "github-runner"

mapfile -t units < <(
  systemctl list-unit-files --type=service --no-legend 'actions.runner.javaquasar-hydracache.*.service' |
    awk '{print $1}'
)
test "${#units[@]}" -eq 1
unit="${units[0]}"
service_user="$(systemctl show "$unit" --property=User --value)"
test "$service_user" = "github-runner"

printf 'runner service contract passed: unit=%s state=%s\n' \
  "$unit" \
  "$(systemctl is-active "$unit" || true)"
