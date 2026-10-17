#!/bin/sh
set -eu
cd /tmp/hc074-safety-alarm-20261017.dzh0Im
. ./guard.sh
[ ! -e /opt/hydracache-performance/0.74/diagnostic-pilot ]
# Check actual executable objects, not argv: these tests run as ./binary.
for item in /proc/[0-9]*/exe; do
    executable=$(readlink "$item" 2>/dev/null || true)
    case "$executable" in
      /tmp/hc074-safety-alarm-20261017.dzh0Im/tests-194*|/tmp/hc074-safety-alarm-20261017.dzh0Im/tests-188*|/tmp/hc074-safety-alarm-20261017.dzh0Im/attribution-194*|/tmp/hc074-safety-alarm-20261017.dzh0Im/diagnostic-final-194*)
        printf 'HC074-SAFETY-ALARM-POSTFLIGHT unexpected_executable=%s\n' "$executable"
        exit 91
        ;;
    esac
done
printf 'HC074-SAFETY-ALARM-POSTFLIGHT no_fixture_executable=true diagnostic_install_absent=true\n'
