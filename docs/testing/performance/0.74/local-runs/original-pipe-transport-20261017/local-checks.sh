#!/bin/sh
set -eu
cd /mnt/c/Workspace/prj/jq/cashe/hydracache-074
packet=docs/testing/performance/0.74/local-runs/original-pipe-transport-20261017
lane="$1"
case "$lane" in 194) toolchain=1.94.0 ;; 188) toolchain=1.88.0 ;; *) exit 81 ;; esac
export CARGO_TARGET_DIR="/home/artur/.cache/hydracache-074-$lane-check"
cargo=/home/artur/.cargo/bin/cargo
"$cargo" "+$toolchain" test -p hydracache-long-run-supervisor-074 --lib original_pipe_message --locked -- --nocapture
"$cargo" "+$toolchain" test -p hydracache-long-run-supervisor-074 --test diagnostic_original_pipe_transport --test diagnostic_original_start_material --test diagnostic_original_output_join --test diagnostic_original_collector_pipes --test diagnostic_intent_projection --test diagnostic_start_consent --locked
"$cargo" "+$toolchain" test -p hydracache-long-run-supervisor-074 --doc pipe_transport --locked
"$cargo" "+$toolchain" check -p hydracache-long-run-supervisor-074 --all-targets --locked
"$cargo" "+$toolchain" clippy -p hydracache-long-run-supervisor-074 --all-targets --all-features --locked -- -D warnings
"$cargo" "+$toolchain" test -p hydracache-long-run-supervisor-074 --release --lib original_pipe_message --locked -- --nocapture
printf 'HC074-PIPE-LOCAL lane=%s completed=true\n' "$lane"
