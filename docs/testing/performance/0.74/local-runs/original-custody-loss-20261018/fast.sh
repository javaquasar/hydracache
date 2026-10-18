#!/bin/bash
set -euo pipefail
for compiler in 194 188; do
    case "$compiler" in
        194) bin=/home/artur/.cache/hydracache-074-194-check/release/deps/hydracache_long_run_supervisor_074-171a8ee7d3705ebd ;;
        188) bin=/home/artur/.cache/hydracache-074-188-check/release/deps/hydracache_long_run_supervisor_074-81989065b672da94 ;;
    esac
    printf 'HC074-CUSTODY-LOSS-FAST compiler=%s\n' "$compiler"
    "$bin" descriptor_plan:: --nocapture --test-threads=1
    "$bin" diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence:: --skip original_fence_custody_matrix --nocapture --test-threads=1
done
cd tools/long-run-supervisor-074/tests
python3 -m unittest -v test_original_custody_owner_loss.py
