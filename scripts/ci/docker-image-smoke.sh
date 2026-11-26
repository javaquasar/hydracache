#!/usr/bin/env bash
# Smoke-test a built hydracache-server container image.
#
# Usage: scripts/ci/docker-image-smoke.sh <image-ref> <expected-version>
#
# Checks, all of which must pass (fail loud, no partial success):
#   1. image metadata: runs as nonroot, version label matches, admin port exposed;
#   2. secure-by-default: with non-loopback listeners and no TLS, the daemon
#      refuses to start unless HYDRACACHE_TLS_ACK_INSECURE=true is set;
#   3. with the explicit insecure acknowledgement, /healthz and /readyz on the
#      admin port return 200 within the timeout.
set -euo pipefail

image="${1:?usage: docker-image-smoke.sh <image-ref> <expected-version>}"
expected_version="${2:?usage: docker-image-smoke.sh <image-ref> <expected-version>}"
timeout_seconds="${HYDRACACHE_SMOKE_TIMEOUT_SECONDS:-60}"
container="hydracache-image-smoke-$$"

fail() {
  echo "image smoke FAILED: $*" >&2
  exit 1
}

cleanup() {
  docker rm -f "$container" >/dev/null 2>&1 || true
  docker rm -f "${container}-insecure-default" >/dev/null 2>&1 || true
}
trap cleanup EXIT

# 1. Metadata.
user="$(docker image inspect --format '{{.Config.User}}' "$image")"
[ "$user" = "nonroot:nonroot" ] || fail "image user is '$user', expected nonroot:nonroot"

label_version="$(docker image inspect --format '{{index .Config.Labels "org.opencontainers.image.version"}}' "$image")"
[ "$label_version" = "$expected_version" ] \
  || fail "org.opencontainers.image.version is '$label_version', expected '$expected_version'"

exposed="$(docker image inspect --format '{{json .Config.ExposedPorts}}' "$image")"
case "$exposed" in
  *'"9091/tcp"'*) ;;
  *) fail "admin port 9091/tcp is not exposed (ExposedPorts=$exposed)" ;;
esac

# 2. Default configuration must refuse plaintext non-loopback listeners.
docker run -d --name "${container}-insecure-default" "$image" >/dev/null
refuse_deadline=$((SECONDS + 20))
while [ "$(docker inspect --format '{{.State.Running}}' "${container}-insecure-default")" = "true" ]; do
  [ "$SECONDS" -lt "$refuse_deadline" ] \
    || fail "daemon kept running without TLS or HYDRACACHE_TLS_ACK_INSECURE; plaintext must be opt-in"
  sleep 1
done
exit_code="$(docker inspect --format '{{.State.ExitCode}}' "${container}-insecure-default")"
[ "$exit_code" != "0" ] || fail "insecure default run exited 0"
echo "ok: default image refuses plaintext non-loopback listeners (exit $exit_code)"

# 3. Explicitly acknowledged insecure single member must become healthy and ready.
docker run -d --name "$container" \
  -e HYDRACACHE_TLS_ACK_INSECURE=true \
  -p 127.0.0.1::9091 \
  "$image" >/dev/null
admin_port="$(docker port "$container" 9091/tcp | head -n1 | sed 's/.*://')"
[ -n "$admin_port" ] || fail "could not resolve published admin port"

probe() {
  curl --silent --output /dev/null --write-out '%{http_code}' --max-time 2 \
    "http://127.0.0.1:${admin_port}$1" || true
}

deadline=$((SECONDS + timeout_seconds))
health=""
ready=""
while [ "$SECONDS" -lt "$deadline" ]; do
  running="$(docker inspect --format '{{.State.Running}}' "$container")"
  if [ "$running" != "true" ]; then
    docker logs "$container" >&2 || true
    fail "container exited before becoming ready"
  fi
  health="$(probe /healthz)"
  ready="$(probe /readyz)"
  if [ "$health" = "200" ] && [ "$ready" = "200" ]; then
    echo "ok: /healthz=200 /readyz=200 on admin port"
    echo "image smoke passed for $image ($expected_version)"
    exit 0
  fi
  sleep 1
done

docker logs "$container" >&2 || true
fail "timed out after ${timeout_seconds}s (last /healthz=$health /readyz=$ready)"
