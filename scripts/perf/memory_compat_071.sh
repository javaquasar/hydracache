#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: memory_compat_071.sh --candidate-sha SHA --workflow-sha SHA --campaign-id ID --output PATH" >&2
}

candidate_sha=""
workflow_sha=""
campaign_id=""
output=""
while (($#)); do
  case "$1" in
    --candidate-sha) candidate_sha="${2:-}"; shift 2 ;;
    --workflow-sha) workflow_sha="${2:-}"; shift 2 ;;
    --campaign-id) campaign_id="${2:-}"; shift 2 ;;
    --output) output="${2:-}"; shift 2 ;;
    *) usage; exit 2 ;;
  esac
done

full_sha='^[0-9a-f]{40}$'
[[ "$candidate_sha" =~ $full_sha ]] || { echo "candidate SHA must be lowercase and complete" >&2; exit 2; }
[[ "$workflow_sha" =~ $full_sha ]] || { echo "workflow SHA must be lowercase and complete" >&2; exit 2; }
[[ -n "$campaign_id" && -n "$output" ]] || { usage; exit 2; }

root="$(git rev-parse --show-toplevel)"
test "$(git -C "$root" rev-parse HEAD)" = "$workflow_sha"
test "$(git -C "$root" rev-parse 'v0.70.0^{commit}')" = "75719b0bf5de2250cf4eb16a30073dd7429538e3"
git -C "$root" cat-file -e "$candidate_sha^{commit}"

if [[ -e "$output" ]]; then
  python3 "$root/scripts/perf/memory_compat_receipt_071.py" \
    --receipt "$output" \
    --campaign-id "$campaign_id" \
    --candidate-sha "$candidate_sha" \
    --workflow-sha "$workflow_sha"
  exit 0
fi

temp_base="${RUNNER_TEMP:-${TMPDIR:-/tmp}}"
work_root="$(mktemp -d "$temp_base/memory-compat-071.XXXXXXXX")"
cleanup() {
  case "$work_root" in
    "$temp_base"/memory-compat-071.*) rm -rf -- "$work_root" ;;
    *) echo "refusing unsafe compatibility cleanup: $work_root" >&2 ;;
  esac
}
trap cleanup EXIT

baseline_src="$work_root/baseline-src"
candidate_src="$work_root/candidate-src"
mkdir -p "$baseline_src" "$candidate_src"
git -C "$root" archive 'v0.70.0^{commit}' | tar -x -C "$baseline_src"
git -C "$root" archive "$candidate_sha" | tar -x -C "$candidate_src"
for source_root in "$baseline_src" "$candidate_src"; do
  mkdir -p "$source_root/crates/hydracache/examples"
  cp "$root/scripts/perf/memory_compat_driver_071.rs" \
    "$source_root/crates/hydracache/examples/memory_compat_071.rs"
done

baseline_target="$work_root/baseline-target"
candidate_target="$work_root/candidate-target"
cargo build --manifest-path "$baseline_src/Cargo.toml" --locked --release \
  -p hydracache --example memory_compat_071 --features durable-value-store \
  --target-dir "$baseline_target"
cargo build --manifest-path "$candidate_src/Cargo.toml" --locked --release \
  -p hydracache --example memory_compat_071 --features durable-value-store \
  --target-dir "$candidate_target"

baseline_driver="$baseline_target/release/examples/memory_compat_071"
candidate_driver="$candidate_target/release/examples/memory_compat_071"
store="$work_root/store"
"$baseline_driver" create-baseline "$store"
"$candidate_driver" verify-and-mutate-candidate "$store"
"$candidate_driver" verify-complete "$store"
"$baseline_driver" verify-complete "$store"

hold_log="$work_root/hold.log"
"$candidate_driver" hold-after-flush "$store" >"$hold_log" 2>&1 &
hold_pid=$!
ready=0
for _ in $(seq 1 100); do
  if grep -q '^READY$' "$hold_log"; then ready=1; break; fi
  if ! kill -0 "$hold_pid" 2>/dev/null; then break; fi
  sleep 0.1
done
test "$ready" = 1
kill -KILL "$hold_pid"
wait "$hold_pid" || true
"$candidate_driver" verify-complete "$store"

cp -a "$store" "$work_root/baseline-backup"
cp -a "$work_root/baseline-backup" "$work_root/future-store"
"$candidate_driver" write-future-and-refuse "$work_root/future-store"
"$baseline_driver" verify-complete "$work_root/baseline-backup"

cargo test --manifest-path "$baseline_src/Cargo.toml" --locked --release \
  -p hydracache-client-protocol --test protocol --test versioned_codec \
  --target-dir "$baseline_target"
cargo test --manifest-path "$candidate_src/Cargo.toml" --locked --release \
  -p hydracache-client-protocol --test protocol --test versioned_codec \
  --target-dir "$candidate_target"

cargo build --manifest-path "$baseline_src/Cargo.toml" --locked --release \
  -p hydracache-server --target-dir "$baseline_target"
cargo build --manifest-path "$candidate_src/Cargo.toml" --locked --release \
  -p hydracache-server --target-dir "$candidate_target"
baseline_server="$baseline_target/release/hydracache-server"
candidate_server="$candidate_target/release/hydracache-server"
HYDRACACHE_MEMORY_071_COMPAT_REQUIRED=1 \
HYDRACACHE_MEMORY_071_BASELINE_BINARY="$baseline_server" \
HYDRACACHE_MEMORY_071_CANDIDATE_BINARY="$candidate_server" \
  cargo test --manifest-path "$root/Cargo.toml" --locked \
    -p hydracache-server --test memory_compat_process_071 -- \
    --test-threads=1

mkdir -p "$(dirname "$output")"
python3 - "$output" "$campaign_id" "$candidate_sha" "$workflow_sha" \
  "$(sha256sum "$root/scripts/perf/memory_compat_driver_071.rs" | cut -d' ' -f1)" \
  "$(sha256sum "$baseline_server" | cut -d' ' -f1)" \
  "$(sha256sum "$candidate_server" | cut -d' ' -f1)" <<'PY'
import datetime
import hashlib
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
receipt = {
    "schema_version": 1,
    "release": "0.71",
    "campaign_id": sys.argv[2],
    "baseline_tag": "v0.70.0",
    "baseline_commit": "75719b0bf5de2250cf4eb16a30073dd7429538e3",
    "candidate_sha": sys.argv[3],
    "workflow_sha": sys.argv[4],
    "result": "success",
    "generated_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    "driver_sha256": sys.argv[5],
    "binary_sha256": {"baseline": sys.argv[6], "candidate": sys.argv[7]},
    "checks": [
        "baseline-create-candidate-read-mutate-restart",
        "candidate-create-candidate-restart",
        "candidate-to-baseline-compatible-rollback",
        "rolling-baseline-candidate-all-role-orders",
        "snapshot-empty-max-record-crash-upgrade",
        "unknown-future-refuse-before-mutation-and-backup-restore",
        "hc1-hc2-versioned-wire-corpus-both-binaries",
    ],
}
canonical = json.dumps(receipt, sort_keys=True, separators=(",", ":")).encode()
receipt["receipt_sha256"] = hashlib.sha256(canonical).hexdigest()
path.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY

python3 "$root/scripts/perf/memory_compat_receipt_071.py" \
  --receipt "$output" \
  --campaign-id "$campaign_id" \
  --candidate-sha "$candidate_sha" \
  --workflow-sha "$workflow_sha"

echo "memory compatibility 0.71: OK ($output)"
