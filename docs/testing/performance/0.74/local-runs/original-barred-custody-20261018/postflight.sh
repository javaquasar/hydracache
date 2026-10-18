#!/bin/bash
set -euo pipefail
cd "$1"
source ./guard.sh
for item in /proc/[0-9]*/exe; do
    executable=$(readlink "$item" 2>/dev/null || true)
    case "$executable" in
        /tmp/hc074-original-barred-custody-20261018.XleMGc/tests-*|"$PWD"/tests-*)
            printf 'HC074-BARRED-CUSTODY unexpected_executable=%s\n' "$executable"
            exit 91 ;;
    esac
done
printf 'HC074-BARRED-CUSTODY-FINAL old_fixture_absent=true new_fixture_absent=true supervisor_unchanged=true admission=false\n'
