#!/bin/bash
set -euo pipefail
cd "$1"
source ./guard.sh
for item in /proc/[0-9]*/exe; do
    executable=$(readlink "$item" 2>/dev/null || true)
    case "$executable" in /tmp/hc074-collector-loss-matrix-20261018.*/tests-*|/tmp/hc074-barred-collector-20261018.*/tests-*|/tmp/hc074-custody-loss-matrix-20261018.*/tests-*) printf 'unexpected_executable=%s\n' "$executable"; exit 91;; esac
done
printf 'HC074-COLLECTOR-LOSS-STOPPED postflight=ok no_fixture_executable=true supervisor_unchanged=true admission=false\n'
