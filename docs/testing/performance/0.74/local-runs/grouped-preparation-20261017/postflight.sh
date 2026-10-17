#!/bin/sh
set -eu
cd "$1"
. ./guard.sh
[ ! -e /opt/hydracache-performance/0.74/diagnostic-pilot ]
for item in /proc/[0-9]*/exe; do
    executable=$(readlink "$item" 2>/dev/null || true)
    case "$executable" in
      "$PWD"/tests-194*|"$PWD"/tests-188*)
        printf 'HC074-GROUPED-POSTFLIGHT unexpected_executable=%s\n' "$executable"
        exit 91 ;;
    esac
done
printf 'HC074-GROUPED-POSTFLIGHT no_fixture_executable=true diagnostic_install_absent=true\n'
