#!/bin/bash
set -euo pipefail
for compiler in 194 188; do
    image=/home/artur/.cache/hydracache-074-194-check/release/deps/hydracache_long_run_supervisor_074-171a8ee7d3705ebd
    if [ "$compiler" = 188 ]; then image=/home/artur/.cache/hydracache-074-188-check/release/deps/hydracache_long_run_supervisor_074-81989065b672da94; fi
    printf 'HC074-COLLECTOR-NOTICES compiler=%s begin=true admission=false\n' "$compiler"
    sha256sum "$image"
    "$image" descriptor_plan:: --nocapture --test-threads=1
    printf 'HC074-COLLECTOR-NOTICES compiler=%s completed=true admission=false\n' "$compiler"
done
