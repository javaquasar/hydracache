#!/bin/sh
set -eu
cd /tmp/hc074-controller-cost-20261017.uYyrhr
. ./guard.sh
[ ! -e /opt/hydracache-performance/0.74/diagnostic-pilot ]
for item in /proc/[0-9]*/exe; do
    executable=$(readlink "$item" 2>/dev/null || true)
    case "$executable" in
      /tmp/hc074-controller-cost-20261017.uYyrhr/tests-194*|/tmp/hc074-controller-cost-20261017.uYyrhr/tests-188*)
        printf 'HC074-COMPONENT-POSTFLIGHT unexpected_executable=%s\n' "$executable"
        exit 91 ;;
    esac
done
printf 'HC074-COMPONENT-POSTFLIGHT no_fixture_executable=true diagnostic_install_absent=true\n'
