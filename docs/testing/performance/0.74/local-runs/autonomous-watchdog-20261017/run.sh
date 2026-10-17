#!/bin/bash
set -euo pipefail
cd "$1"
source ./guard.sh
exec 9>"$PWD/.driver-lock"
flock -n 9
[ ! -e /opt/hydracache-performance/0.74/diagnostic-pilot ]
for compiler in 194 188; do
    preflight
    printf 'HC074-AUTONOMOUS-COHORT compiler=%s\n' "$compiler"
    sha256sum "tests-$compiler"
    timeout 120 "./tests-$compiler" watchdog::autonomous::tests --skip autonomous_writer_fence_matrix --nocapture --test-threads=1
    timeout 120 "./tests-$compiler" --exact watchdog::autonomous::tests::autonomous_writer_fence_matrix --ignored --nocapture --test-threads=1
    printf 'HC074-AUTONOMOUS-COHORT compiler=%s completed=true\n' "$compiler"
    preflight
done
for item in /proc/[0-9]*/exe; do
    executable=$(readlink "$item" 2>/dev/null || true)
    case "$executable" in "$PWD"/tests-*) printf 'HC074-AUTONOMOUS unexpected_executable=%s\n' "$executable"; exit 91 ;; esac
done
[ ! -e /opt/hydracache-performance/0.74/diagnostic-pilot ]
printf 'HC074-AUTONOMOUS-HOST completed=true supervisor_unchanged=true no_fixture_executable=true\n'
