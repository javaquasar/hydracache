#!/usr/bin/env bash
set -uo pipefail

tooling_sha="${1:?tooling SHA is required}"
evidence_dir="${2:?evidence directory is required}"
workspace="${GITHUB_WORKSPACE:-$(pwd)}"
runner_temp="${RUNNER_TEMP:-$workspace/target/compatibility-073-ci-temp}"
b72_sha="24927c28c279c6c34ad90111ee6470b4065e0815"
c73_sha="7e3070894aa51af96cdcb3e350eff923a309e1fa"
c73_tree="e2c438b9a248586a4f72d3eca3d1c5369fff440b"
run_key="${GITHUB_RUN_ID:-local}-${GITHUB_RUN_ATTEMPT:-1}"

mkdir -p "$evidence_dir"

run_campaign() (
  set -euo pipefail
  cd "$workspace"
  [[ "$tooling_sha" =~ ^[0-9a-f]{40}$ ]]
  test "$(git rev-parse HEAD)" = "$tooling_sha"
  test -z "$(git status --porcelain=v1 --untracked-files=normal)"

  b72_root="$runner_temp/hydracache-b72-$run_key"
  c73_root="$runner_temp/hydracache-c73-compat-$run_key"
  b72_stage="$runner_temp/compatibility-b72-stage-$run_key"
  c73_stage="$runner_temp/compatibility-c73-stage-$run_key"
  git worktree prune
  git worktree add --detach "$b72_root" "$b72_sha"
  git worktree add --detach "$c73_root" "$c73_sha"
  test "$(git -C "$b72_root" rev-parse HEAD)" = "$b72_sha"
  test "$(git -C "$b72_root" rev-parse 'v0.72.0^{commit}')" = "$b72_sha"
  test "$(git -C "$c73_root" rev-parse HEAD)" = "$c73_sha"
  test "$(git -C "$c73_root" rev-parse 'HEAD^{tree}')" = "$c73_tree"

  mkdir -p "$b72_stage/tools" "$c73_stage/tools"
  cp -a tools/compatibility-073 "$b72_stage/tools/"
  cp -a tools/compatibility-073 "$c73_stage/tools/"
  cp "$b72_stage/tools/compatibility-073/Cargo.b72.lock" \
    "$b72_stage/tools/compatibility-073/Cargo.lock"
  ln -s "$b72_root/Cargo.toml" "$b72_stage/Cargo.toml"
  ln -s "$b72_root/crates" "$b72_stage/crates"
  ln -s "$c73_root/Cargo.toml" "$c73_stage/Cargo.toml"
  ln -s "$c73_root/crates" "$c73_stage/crates"

  cargo build --manifest-path "$b72_root/Cargo.toml" --release --locked -p hydracache-server
  cargo build --manifest-path "$c73_root/Cargo.toml" --release --locked -p hydracache-server
  cargo build --manifest-path "$b72_stage/tools/compatibility-073/Cargo.toml" --release --locked
  cargo build --manifest-path "$c73_stage/tools/compatibility-073/Cargo.toml" --release --locked
  test -z "$(git -C "$b72_root" status --porcelain=v1 --untracked-files=normal)"
  test -z "$(git -C "$c73_root" status --porcelain=v1 --untracked-files=normal)"

  b72_server="$b72_root/target/release/hydracache-server"
  c73_server="$c73_root/target/release/hydracache-server"
  b72_harness="$b72_stage/tools/compatibility-073/target/release/compatibility-073"
  c73_harness="$c73_stage/tools/compatibility-073/target/release/compatibility-073"
  common=(
    --b72-harness "$b72_harness"
    --c73-harness "$c73_harness"
    --b72-server "$b72_server"
    --c73-server "$c73_server"
    --b72-root "$b72_root"
    --c73-root "$c73_root"
    --scenario docs/testing/performance/0.73/w10-published-072-compatibility-contract.toml
  )
  python3 scripts/perf/compatibility_073.py \
    --mode canary "${common[@]}" --output "$evidence_dir/canary"
  python3 scripts/perf/compatibility_073.py \
    --mode campaign "${common[@]}" --output "$evidence_dir/campaign"

  mkdir -p "$evidence_dir/rolling"
  env \
    HYDRACACHE_RUN_COMPATIBILITY_073_ROLLING=1 \
    HYDRACACHE_072_DAEMON_BINARY="$b72_server" \
    HYDRACACHE_072_DAEMON_SOURCE_REF=v0.72.0 \
    HYDRACACHE_072_DAEMON_SOURCE_COMMIT="$b72_sha" \
    HYDRACACHE_C73_DAEMON_BINARY="$c73_server" \
    HYDRACACHE_C73_SOURCE_COMMIT="$c73_sha" \
    "CARGO_BIN_EXE_hydracache-server=$c73_server" \
    HYDRACACHE_COMPATIBILITY_073_ROLLING_OUTPUT="$evidence_dir/rolling/rolling.json" \
    cargo test -p hydracache-server --test performance_compatibility_073 --locked \
      real_published_072_candidate_upgrade_restart_and_same_disk_rollback -- \
      --exact --nocapture --test-threads=1
)

run_campaign
campaign_status=$?

python3 - "$evidence_dir" "$tooling_sha" "$campaign_status" <<'PY'
import hashlib
import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
tooling_sha = sys.argv[2]
campaign_status = int(sys.argv[3])
canary_path = root / "canary" / "canary.json"
campaign_path = root / "campaign" / "compatibility-campaign.json"
rolling_path = root / "rolling" / "rolling.json"
required = (canary_path, campaign_path, rolling_path)
missing = [str(path.relative_to(root)) for path in required if not path.is_file()]
digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
if missing or campaign_status != 0:
    manifest = {
        "schema_version": 1,
        "release": "0.73",
        "profile_id": "published-072-compatibility-073-v1",
        "tooling_sha": tooling_sha,
        "result": "incomplete",
        "campaign_exit_code": campaign_status,
        "missing": missing,
        "available_sha256": {
            str(path.relative_to(root)): digest(path)
            for path in required if path.is_file()
        },
        "long_run_allowed": False,
    }
    (root / "manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    raise SystemExit(campaign_status or 1)

canary = json.loads(canary_path.read_text(encoding="utf-8"))
campaign = json.loads(campaign_path.read_text(encoding="utf-8"))
rolling = json.loads(rolling_path.read_text(encoding="utf-8"))
if canary.get("result") != "passed" or not canary.get("marker_observed") or not canary.get("receipt_absent"):
    raise SystemExit("compatibility canary did not fail closed")
if campaign.get("result") != "passed-local-wire-and-durable-only":
    raise SystemExit("wire/durable campaign result changed")
if campaign.get("wire_cells_passed") != 4 or campaign.get("durable_transitions_passed") != 3:
    raise SystemExit("wire or durable matrix incomplete")
expected_rolling = [
    "B72-leader-C73-followers",
    "leadership-change-during-mixed-cluster",
    "C73-leader-B72-follower",
    "B72-follower-same-disk-restart",
    "full-C73-cluster",
    "same-disk-rollback-to-B72",
]
if rolling.get("result") != "passed" or [item.get("id") for item in rolling.get("scenarios", [])] != expected_rolling:
    raise SystemExit("rolling matrix incomplete")
manifest = {
    "schema_version": 1,
    "release": "0.73",
    "profile_id": "published-072-compatibility-073-v1",
    "tooling_sha": tooling_sha,
    "published_source_commit": campaign["published_source_commit"],
    "candidate_source_commit": campaign["candidate_source_commit"],
    "candidate_tree_oid": campaign["candidate_tree_oid"],
    "canary_sha256": digest(canary_path),
    "campaign_sha256": digest(campaign_path),
    "rolling_sha256": digest(rolling_path),
    "wire_cells_passed": 4,
    "durable_transitions_passed": 3,
    "rolling_scenarios_passed": 6,
    "result": "passed",
    "long_run_allowed": True,
    "host_performance_claim_allowed": False,
}
(root / "manifest.json").write_text(
    json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
)
PY
seal_status=$?

if [[ "$campaign_status" -ne 0 ]]; then
  exit "$campaign_status"
fi
exit "$seal_status"
