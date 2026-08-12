#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: $0 --base <git-sha> --head <git-sha> --output <directory>" >&2
  exit 2
}

base_sha=""
head_sha=""
output=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --base) base_sha="${2:-}"; shift 2 ;;
    --head) head_sha="${2:-}"; shift 2 ;;
    --output) output="${2:-}"; shift 2 ;;
    *) usage ;;
  esac
done
[ -n "$base_sha" ] && [ -n "$head_sha" ] && [ -n "$output" ] || usage

repo_root="$(git rev-parse --show-toplevel)"
base_sha="$(git rev-parse --verify "${base_sha}^{commit}")"
head_sha="$(git rev-parse --verify "${head_sha}^{commit}")"
case "$output" in
  /*) ;;
  *) output="$repo_root/$output" ;;
esac

command -v valgrind >/dev/null || { echo "valgrind is required" >&2; exit 1; }
command -v gungraun-runner >/dev/null || { echo "gungraun-runner 0.19.4 is required" >&2; exit 1; }
command -v python3 >/dev/null || { echo "python3 is required" >&2; exit 1; }

scratch="$(mktemp -d)"
cleanup() { rm -rf -- "$scratch"; }
trap cleanup EXIT
mkdir -p "$scratch/base" "$scratch/head" "$output/raw"
git archive "$base_sha" | tar -x -C "$scratch/base"
git archive "$head_sha" | tar -x -C "$scratch/head"
cp -a "$repo_root/scripts/perf/ci-instruction-harness" "$scratch/harness"
ln -s "$scratch/base" "$scratch/subject"

export CARGO_TARGET_DIR="$scratch/target"
export CARGO_TERM_COLOR=never
cd "$scratch/harness"

sync_subject_lock() {
  local side="$1"
  local subject_version
  local before_lock="$scratch/${side}-lock.before"
  local sync_log="$output/${side}-lock-sync.log"

  subject_version="$(python3 - "$scratch/subject/Cargo.toml" <<'PY'
import pathlib
import sys
import tomllib

with pathlib.Path(sys.argv[1]).open("rb") as handle:
    print(tomllib.load(handle)["workspace"]["package"]["version"])
PY
)"
  cp Cargo.lock "$before_lock"
  : >"$sync_log"
  for package in hydracache-core hydracache-macros hydracache; do
    cargo update --offline -p "$package" --precise "$subject_version" >>"$sync_log" 2>&1
  done

  python3 - "$before_lock" Cargo.lock "$subject_version" <<'PY'
import pathlib
import sys
import tomllib

before_path, after_path, subject_version = sys.argv[1:]

def load(path):
    with pathlib.Path(path).open("rb") as handle:
        return tomllib.load(handle)

def registry_packages(snapshot):
    return sorted(
        (
            package["name"],
            package["version"],
            package.get("source"),
            package.get("checksum"),
            tuple(package.get("dependencies", [])),
        )
        for package in snapshot["package"]
        if package.get("source") is not None
    )

before = load(before_path)
after = load(after_path)
if registry_packages(before) != registry_packages(after):
    raise SystemExit("subject lock synchronization changed a registry package")

expected = {"hydracache", "hydracache-core", "hydracache-macros"}
actual = {
    package["name"]: package["version"]
    for package in after["package"]
    if package["name"] in expected and package.get("source") is None
}
if set(actual) != expected or set(actual.values()) != {subject_version}:
    raise SystemExit(
        f"subject lock synchronization produced {actual}, expected {subject_version}"
    )
PY
  cp Cargo.lock "$output/${side}-harness.lock"
}

rustc --version --verbose >"$output/rustc.txt"
cargo --version --verbose >"$output/cargo.txt"
valgrind --version >"$output/valgrind.txt"
gungraun-runner --version >"$output/gungraun-runner.txt"
uname -a >"$output/uname.txt"
sha256sum \
  "$repo_root/docs/testing/perf-policies/ci-instruction-v1.json" \
  "$repo_root/scripts/perf/ci-instruction-harness/Cargo.toml" \
  "$repo_root/scripts/perf/ci-instruction-harness/Cargo.lock" \
  "$repo_root/scripts/perf/ci-instruction-harness/benches/cache_work.rs" \
  >"$output/contract-sha256.txt"

sync_subject_lock base
cargo bench --locked --bench cache_work -- \
  --save-baseline=base \
  --output-format=json \
  --save-summary=pretty-json \
  --allow-aslr=yes \
  --parallel=1 \
  >"$output/base.ndjson" 2>"$output/base.stderr.log"

rm "$scratch/subject"
ln -s "$scratch/head" "$scratch/subject"

sync_subject_lock head
set +e
cargo bench --locked --bench cache_work -- \
  --baseline=base \
  --callgrind-limits='ir=5.0%' \
  --output-format=json \
  --save-summary=pretty-json \
  --allow-aslr=yes \
  --parallel=1 \
  >"$output/head.ndjson" 2>"$output/head.stderr.log"
status=$?
set -e

find "$CARGO_TARGET_DIR/gungraun" -type f -print0 | while IFS= read -r -d '' file; do
  relative="${file#"$CARGO_TARGET_DIR/gungraun/"}"
  destination="$output/raw/$relative"
  mkdir -p "$(dirname "$destination")"
  cp -p "$file" "$destination"
done

python3 "$repo_root/scripts/perf/summarize-ci-instruction.py" \
  --base "$output/base.ndjson" \
  --head "$output/head.ndjson" \
  --base-sha "$base_sha" \
  --head-sha "$head_sha" \
  --status "$status" \
  --output "$output/report.json"

if [ "$status" -ne 0 ]; then
  echo "ci-instruction-v1 rejected head $head_sha (gungraun exit $status)" >&2
  exit "$status"
fi
echo "ci-instruction-v1 accepted head $head_sha against base $base_sha"
