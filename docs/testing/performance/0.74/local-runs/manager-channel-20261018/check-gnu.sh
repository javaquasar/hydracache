#!/bin/sh
set -eu
case "$1" in
194) version=1.94.0 ;;
188) version=1.88.0 ;;
*) exit 64 ;;
esac
export CARGO_TARGET_DIR="/home/artur/.cache/hydracache-074-$1-check"
/home/artur/.cargo/bin/rustc "+$version" -Vv
/home/artur/.cargo/bin/cargo "+$version" test --release -p hydracache-long-run-supervisor-074 --doc --locked OriginalDescriptorPlan
/home/artur/.cargo/bin/cargo "+$version" check -p hydracache-long-run-supervisor-074 --all-targets --locked
/home/artur/.cargo/bin/cargo "+$version" clippy -p hydracache-long-run-supervisor-074 --all-targets --all-features --locked -- -D warnings
printf 'HC074-MANAGER-GNU compiler=%s completed=true admission=false\n' "$1"
