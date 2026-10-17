#!/bin/sh
set -eu
cd /mnt/c/Workspace/prj/jq/cashe/hydracache-074
lane="$1"
case "$lane" in 194) toolchain=1.94.0 ;; 188) toolchain=1.88.0 ;; *) exit 81 ;; esac
export CARGO_TARGET_DIR="/home/artur/.cache/hydracache-074-$lane-check"
cargo=/home/artur/.cargo/bin/cargo
/home/artur/.cargo/bin/rustc "+$toolchain" -Vv
"$cargo" "+$toolchain" test -p hydracache-long-run-supervisor-074 --release --lib original_pipe_message --locked -- --nocapture
"$cargo" "+$toolchain" check -p hydracache-long-run-supervisor-074 --all-targets --locked
"$cargo" "+$toolchain" clippy -p hydracache-long-run-supervisor-074 --all-targets --all-features --locked -- -D warnings
printf 'HC074-PIPE-FINAL lane=%s completed=true\n' "$lane"
