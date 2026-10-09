#!/usr/bin/env bash
# Local read-only audit repeat; no host connection, admission, install or exec.
set -euo pipefail
checkout=/home/artur/.cache/hydracache-074-builder-feature-checkout
capture=/mnt/c/Workspace/prj/jq/cashe/hydracache-074/docs/testing/performance/0.74/local-runs/diagnostic-builder-hosted-37923423889
bundle=/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/diagnostic-hosted-build-37923423889
tools=/home/artur/.cache/hydracache-074-registration-tools
cd "$checkout"
test "$(git rev-parse HEAD)" = 33596da76b5f31fdd27eea51ec219aefe805180c
test -z "$(git status --porcelain=v1)"
export PATH=/home/artur/.cargo/bin:$PATH
export RUSTUP_TOOLCHAIN=1.94.0
export CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-manager-local-target
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2
test ! -e "$capture/verifier-library-build.jsonl"
cargo build -p hydracache-long-run-supervisor-074 --lib --locked --message-format=json \
  > "$capture/verifier-library-build.jsonl" 2> "$capture/verifier-library-build.stderr.log"
rustfmt +1.94.0 --edition 2021 --check "$capture/receipt-verifier.rs"
reader="$tools/receipt-verifier-hosted-37923423889"
tests="$tools/receipt-verifier-hosted-tests-37923423889"
test ! -e "$reader" && test ! -e "$tests"
rustc +1.94.0 --edition=2021 -D warnings "$capture/receipt-verifier.rs" \
  --extern hydracache_long_run_supervisor_074="$CARGO_TARGET_DIR/debug/libhydracache_long_run_supervisor_074.rlib" \
  -L dependency="$CARGO_TARGET_DIR/debug/deps" -o "$reader"
rustc +1.94.0 --edition=2021 -D warnings --test "$capture/receipt-verifier.rs" \
  --extern hydracache_long_run_supervisor_074="$CARGO_TARGET_DIR/debug/libhydracache_long_run_supervisor_074.rlib" \
  -L dependency="$CARGO_TARGET_DIR/debug/deps" -o "$tests"
test -z "$(git status --porcelain=v1)"
"$tests"
"$reader" /mnt/c/Workspace/prj/jq/cashe/hydracache-074/docs/testing/performance/0.74/diagnostic-builder-public-policy.json \
  "$bundle/signed/build-receipt-v1.json" "$bundle/unsigned"
sha256sum "$reader" "$CARGO_TARGET_DIR/debug/libhydracache_long_run_supervisor_074.rlib"
