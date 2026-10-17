#!/bin/sh
set -eu
cd /tmp/hc074-leaf-cost-20261017.mF2pzV
. ./guard.sh
[ ! -e /opt/hydracache-performance/0.74/diagnostic-pilot ]
for item in /proc/[0-9]*/exe; do
    executable=$(readlink "$item" 2>/dev/null || true)
    case "$executable" in
      /tmp/hc074-leaf-cost-20261017.mF2pzV/tests-194*|/tmp/hc074-leaf-cost-20261017.mF2pzV/tests-188*)
        printf 'HC074-LEAF-POSTFLIGHT unexpected_executable=%s\n' "$executable"
        exit 91 ;;
    esac
done
printf 'HC074-LEAF-POSTFLIGHT no_fixture_executable=true diagnostic_install_absent=true\n'
