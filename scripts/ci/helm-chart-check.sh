#!/usr/bin/env bash
# Lint, render and schema-validate the Helm chart and the raw k8s manifests.
#
# Usage: scripts/ci/helm-chart-check.sh
# Requires `helm` and `kubeconform` on PATH. KUBERNETES_VERSION pins the
# schema set kubeconform validates against.
#
# Proves, without a cluster:
# - every supported value combination lints and renders to schema-valid objects;
# - combinations the server refuses to start with fail at render time with the
#   expected message instead of producing crash-looping pods;
# - the rendered env matches the chosen transport posture.
set -euo pipefail

CHART="deploy/helm/hydracache"
KUBERNETES_VERSION="${KUBERNETES_VERSION:-1.32.0}"
out="$(mktemp -d)"
trap 'rm -rf "$out"' EXIT

secure=(
  --set tls.secretName=hydracache-mtls
  --set clusterAuth.keyId=cluster-v1
  --set clusterAuth.secretName=hydracache-cluster-auth
)
insecure=(--set tls.enabled=false --set tls.acknowledgeInsecure=true)

fail() {
  echo "helm-chart-check: $*" >&2
  exit 1
}

kubeconform_check() {
  kubeconform -strict -summary -kubernetes-version "$KUBERNETES_VERSION" "$@"
}

render() {
  local name="$1"
  shift
  helm template hydracache "$CHART" "$@" > "$out/$name.yaml"
  kubeconform_check "$out/$name.yaml"
}

expect_render_failure() {
  local expected="$1"
  shift
  local log="$out/failure.log"
  if helm template hydracache "$CHART" "$@" > /dev/null 2> "$log"; then
    fail "expected render failure containing '$expected' for: $*"
  fi
  grep -qF -- "$expected" "$log" || {
    cat "$log" >&2
    fail "render failed, but without '$expected'"
  }
  echo "ok: refused ($expected)"
}

expect_in() {
  local name="$1" needle="$2"
  grep -qF -- "$needle" "$out/$name.yaml" || fail "$name render lacks: $needle"
}

expect_not_in() {
  local name="$1" needle="$2"
  if grep -qF -- "$needle" "$out/$name.yaml"; then
    fail "$name render unexpectedly contains: $needle"
  fi
}

helm lint --strict "$CHART" "${secure[@]}"
helm lint --strict "$CHART" "${insecure[@]}"

render secure "${secure[@]}"
expect_in secure "HYDRACACHE_CLUSTER_AUTH_TOKEN_FILE"
expect_in secure "secretName: hydracache-mtls"
expect_in secure "secretName: hydracache-cluster-auth"
expect_in secure "hydracache-0.hydracache-headless:7000,hydracache-1.hydracache-headless:7000,hydracache-2.hydracache-headless:7000"
expect_not_in secure "HYDRACACHE_TLS_ACK_INSECURE"
expect_not_in secure "HYDRACACHE_BACKUP_LOCATION"

render insecure "${insecure[@]}"
expect_in insecure "HYDRACACHE_TLS_ACK_INSECURE"
expect_not_in insecure "name: tls"
expect_not_in insecure "HYDRACACHE_CLUSTER_AUTH_KEY_ID"

render backup "${secure[@]}" --set backup.enabled=true --set backup.location=s3://example/hydracache
expect_in backup "HYDRACACHE_BACKUP_LOCATION"

# Configurations the server refuses must not render.
expect_render_failure "tls.enabled requires tls.secretName"
expect_render_failure "tls.enabled requires clusterAuth.keyId" --set tls.secretName=hydracache-mtls
expect_render_failure "set tls.acknowledgeInsecure=true" --set tls.enabled=false
expect_render_failure "must be set together" "${insecure[@]}" --set clusterAuth.keyId=cluster-v1
expect_render_failure "backup.enabled requires backup.location" "${secure[@]}" --set backup.enabled=true
expect_render_failure "replicaCount must be at least 1" "${secure[@]}" --set replicaCount=0

kubeconform_check deploy/k8s/*.yaml

echo "Helm chart and k8s manifests are valid"
