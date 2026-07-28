#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

action="${1:-}"
case "$action" in
  online|offline|status) ;;
  *)
    echo "usage: scripts/perf/runner-service.sh <online|offline|status>" >&2
    exit 2
    ;;
esac

mapfile -t units < <(
  systemctl list-unit-files --type=service --no-legend 'actions.runner.javaquasar-hydracache.*.service' |
    awk '{print $1}'
)
test "${#units[@]}" -eq 1
unit="${units[0]}"
test "$(systemctl show "$unit" --property=User --value)" = "github-runner"

case "$action" in
  online)
    sudo systemctl start "$unit"
    systemctl is-active --quiet "$unit"
    ;;
  offline)
    sudo systemctl stop "$unit"
    test "$(systemctl is-active "$unit" || true)" = "inactive"
    ;;
  status)
    systemctl --no-pager --full status "$unit"
    ;;
esac
