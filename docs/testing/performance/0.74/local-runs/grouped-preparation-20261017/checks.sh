#!/bin/sh
set -eu
cd /mnt/c/Workspace/prj/jq/cashe/hydracache-074
lane="$1"
case "$lane" in 194) toolchain=1.94.0 ;; 188) toolchain=1.88.0 ;; *) exit 81 ;; esac
export CARGO_TARGET_DIR="/home/artur/.cache/hydracache-074-$lane-check"
cargo=/home/artur/.cargo/bin/cargo
/home/artur/.cargo/bin/rustc "+$toolchain" -Vv
"$cargo" "+$toolchain" test -p hydracache-long-run-supervisor-074 --release --lib original_grouped_preparation_ --locked -- --list
"$cargo" "+$toolchain" test -p hydracache-long-run-supervisor-074 --release --lib original_alarm_ --locked
printf 'HC074-GROUPED-BUILD lane=%s completed=true\n' "$lane"
"$cargo" "+$toolchain" test -p hydracache-long-run-supervisor-074 --lib original_alarm_ --locked
"$cargo" "+$toolchain" test -p hydracache-long-run-supervisor-074 --lib validation_trace --locked
"$cargo" "+$toolchain" test -p hydracache-long-run-supervisor-074 --test diagnostic_original_pipe_transport --test diagnostic_original_start_material --test diagnostic_original_output_join --test diagnostic_original_collector_pipes --test diagnostic_collector_starting_publication --test diagnostic_intent_projection --test diagnostic_start_consent --locked
"$cargo" "+$toolchain" test -p hydracache-long-run-supervisor-074 --doc OriginalCollectorPipeTransport --locked
"$cargo" "+$toolchain" check -p hydracache-long-run-supervisor-074 --all-targets --locked
"$cargo" "+$toolchain" clippy -p hydracache-long-run-supervisor-074 --all-targets --all-features --locked -- -D warnings
printf 'HC074-GROUPED-CHECKS lane=%s completed=true\n' "$lane"
