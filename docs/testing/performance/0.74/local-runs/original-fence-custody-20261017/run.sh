#!/bin/bash
set -euo pipefail
cd "$1"
source ./guard.sh
exec 9>"$PWD/.driver-lock"
flock -n 9
export TMPDIR="$PWD"
[ ! -e /opt/hydracache-performance/0.74/diagnostic-pilot ]
for compiler in 194 188; do
    preflight
    printf 'HC074-FENCE-CUSTODY-COHORT compiler=%s\n' "$compiler"
    sha256sum "tests-$compiler"
    timeout 120 "./tests-$compiler" diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::tests --skip original_fence_custody_matrix --nocapture --test-threads=1
    timeout 120 "./tests-$compiler" --exact diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence::tests::custody::original_fence_custody_matrix --ignored --nocapture --test-threads=1
    printf 'HC074-FENCE-CUSTODY-COHORT compiler=%s completed=true\n' "$compiler"
    preflight
done
for item in /proc/[0-9]*/exe; do
    executable=$(readlink "$item" 2>/dev/null || true)
    case "$executable" in "$PWD"/tests-*) printf 'HC074-FENCE-CUSTODY unexpected_executable=%s\n' "$executable"; exit 91 ;; esac
done
[ ! -e /opt/hydracache-performance/0.74/diagnostic-pilot ]
printf 'HC074-FENCE-CUSTODY-HOST completed=true supervisor_unchanged=true no_fixture_executable=true\n'
