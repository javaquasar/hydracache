#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'
export LC_ALL=C

action="${1:-}"
case "$action" in
  prepare|verify|materialize) ;;
  *)
    echo "usage: scripts/perf/reference-evidence-tmpfs.sh <prepare|verify|materialize>" >&2
    exit 2
    ;;
esac

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
test -d .git

tmp_root="/dev/shm/hydracache-reference-evidence-v1"
test_evidence_root="target/test-evidence"
release_067_link="${test_evidence_root}/0.67"
release_0671_link="${test_evidence_root}/0.67.1"
release_067_tmp="${tmp_root}/0.67"
release_0671_tmp="${tmp_root}/0.67.1"

test "$(findmnt --noheadings --output FSTYPE --target /dev/shm | xargs)" = tmpfs

verify_link() {
  local link="$1"
  local expected="$2"
  test -L "$link"
  test "$(readlink --canonicalize "$link")" = "$expected"
  test "$(findmnt --noheadings --output FSTYPE --target "$expected" | xargs)" = tmpfs
}

if test "$action" = prepare; then
  test ! -e "$release_067_link"
  test ! -L "$release_067_link"
  test ! -e "$release_0671_link"
  test ! -L "$release_0671_link"
  rm --recursive --force -- "$tmp_root"
  mkdir --parents "$release_067_tmp" "$release_0671_tmp" "$test_evidence_root"
  printf '%s\n' "$(git rev-parse HEAD)" >"${tmp_root}/source-commit"
  ln --symbolic "$release_067_tmp" "$release_067_link"
  ln --symbolic "$release_0671_tmp" "$release_0671_link"
  verify_link "$release_067_link" "$release_067_tmp"
  verify_link "$release_0671_link" "$release_0671_tmp"
  echo "reference evidence tmpfs prepared: root=${tmp_root}"
  exit 0
fi

if test "$action" = verify; then
  verify_link "$release_067_link" "$release_067_tmp"
  verify_link "$release_0671_link" "$release_0671_tmp"
  test "$(cat "${tmp_root}/source-commit")" = "$(git rev-parse HEAD)"
  echo "reference evidence tmpfs verified: root=${tmp_root}"
  exit 0
fi

materialize_one() {
  local link="$1"
  local expected="$2"
  local staging="${link}.materializing"
  if test ! -L "$link"; then
    test ! -e "$link" || {
      echo "reference evidence path is already materialized: ${link}"
      return 0
    }
    echo "reference evidence path is absent; nothing to materialize: ${link}"
    return 0
  fi
  verify_link "$link" "$expected"
  rm --recursive --force -- "$staging"
  mkdir --parents "$staging"
  cp --archive -- "$expected/." "$staging/"
  rm -- "$link"
  mv -- "$staging" "$link"
  test -d "$link"
  test ! -L "$link"
}

materialize_one "$release_067_link" "$release_067_tmp"
materialize_one "$release_0671_link" "$release_0671_tmp"
rm --recursive --force -- "$tmp_root"
echo "reference evidence materialized on housekeeping storage"
