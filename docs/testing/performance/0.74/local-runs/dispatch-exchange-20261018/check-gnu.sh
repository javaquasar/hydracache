#!/bin/sh
set -eu
case "$1" in
194) version=1.94.0; image=171a8ee7d3705ebd ;;
188) version=1.88.0; image=81989065b672da94 ;;
*) exit 64 ;;
esac
export CARGO_TARGET_DIR="/home/artur/.cache/hydracache-074-$1-check"
binary="$CARGO_TARGET_DIR/release/deps/hydracache_long_run_supervisor_074-$image"
/home/artur/.cargo/bin/rustc "+$version" -Vv
sha256sum "$binary"
"$binary" descriptor_plan:: --skip dispatch_exchange:: --nocapture --test-threads=1
"$binary" diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence:: --skip original_fence_custody_matrix --nocapture --test-threads=1
/home/artur/.cargo/bin/cargo "+$version" test --release -p hydracache-long-run-supervisor-074 --doc --locked OriginalDescriptorPlan
/home/artur/.cargo/bin/cargo "+$version" check -p hydracache-long-run-supervisor-074 --all-targets --locked
/home/artur/.cargo/bin/cargo "+$version" clippy -p hydracache-long-run-supervisor-074 --all-targets --all-features --locked -- -D warnings
sha256sum "$binary"
printf 'HC074-DISPATCH-GNU compiler=%s completed=true admission=false\n' "$1"
