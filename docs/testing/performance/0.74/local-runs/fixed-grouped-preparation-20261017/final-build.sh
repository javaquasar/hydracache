#!/bin/sh
set -eu
cd /mnt/c/Workspace/prj/jq/cashe/hydracache-074
lane="$1"
case "$lane" in 194) toolchain=1.94.0 ;; 188) toolchain=1.88.0 ;; *) exit 81 ;; esac
export CARGO_TARGET_DIR="/home/artur/.cache/hydracache-074-$lane-check"
# Rebuild after the final test edits, without racing source formatting.
# This is the sole release libtest used by the remote fixture cohorts.
exec /home/artur/.cargo/bin/cargo "+$toolchain" test -p hydracache-long-run-supervisor-074 --release --lib fixed_grouped_alarm_ --locked
