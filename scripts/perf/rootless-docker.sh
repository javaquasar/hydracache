#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

action="${1:-}"
case "$action" in
  start|stop|status) ;;
  *)
    echo "usage: scripts/perf/rootless-docker.sh <start|stop|status>" >&2
    exit 2
    ;;
esac

test "$(id --user --name)" = "github-runner" || {
  echo "rootless Docker lifecycle must run as github-runner" >&2
  exit 1
}
for tool in docker grep systemctl; do
  command -v "$tool" >/dev/null || {
    echo "missing rootless Docker tool: $tool" >&2
    exit 1
  }
done

runtime_dir="/run/user/$(id --user)"
test -d "$runtime_dir"
export XDG_RUNTIME_DIR="$runtime_dir"
export DBUS_SESSION_BUS_ADDRESS="unix:path=${runtime_dir}/bus"
export DOCKER_HOST="unix://${runtime_dir}/docker.sock"

case "$action" in
  start)
    test ! -S /var/run/docker.sock || {
      echo "rootful Docker socket must remain absent" >&2
      exit 1
    }
    systemctl --user start docker.service
    for _ in $(seq 1 30); do
      if docker info --format '{{json .SecurityOptions}}' 2>/dev/null |
        grep --quiet rootless; then
        break
      fi
      sleep 1
    done
    docker info --format '{{json .SecurityOptions}}' | grep --quiet rootless
    if test -n "${GITHUB_ENV:-}"; then
      {
        printf 'XDG_RUNTIME_DIR=%s\n' "$XDG_RUNTIME_DIR"
        printf 'DBUS_SESSION_BUS_ADDRESS=%s\n' "$DBUS_SESSION_BUS_ADDRESS"
        printf 'DOCKER_HOST=%s\n' "$DOCKER_HOST"
      } >>"$GITHUB_ENV"
    fi
    ;;
  stop)
    if systemctl --user is-active --quiet docker.service; then
      systemctl --user stop docker.service
    fi
    test "$(systemctl --user is-active docker.service || true)" = "inactive"
    ;;
  status)
    systemctl --user is-active docker.service
    docker info --format '{{json .SecurityOptions}}' | grep --quiet rootless
    ;;
esac
