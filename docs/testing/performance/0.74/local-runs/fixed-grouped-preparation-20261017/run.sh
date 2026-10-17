#!/bin/sh
set -eu
umask 077
cd "$1"
. ./guard.sh
lane="$2"
mode="$3"
expected="$4"
bytes="$5"
case "$lane" in 194|188) ;; *) exit 81 ;; esac
case "$expected" in *[!0-9a-f]*|'') exit 82 ;; esac
[ "${#expected}" -eq 64 ]
case "$bytes" in *[!0-9]*|'') exit 83 ;; esac
case "$mode" in
screen) [ "$lane" = 194 ]; selected=fixed_grouped_preparation_screen ;;
healthy) selected=fixed_grouped_preparation_healthy ;;
refusals) selected=fixed_grouped_preparation_refusals ;;
origin) selected=fixed_grouped_preparation_fixture_cannot_enter_production ;;
*) exit 84 ;;
esac
exec 9>"$PWD/.cohort.lock"
flock -n 9
if [ "$mode" != screen ]; then [ -f .screen-ok ] && [ ! -L .screen-ok ]; fi
[ -f "./tests-$lane" ] && [ ! -L "./tests-$lane" ]
[ "$(stat -c '%u:%g:%h:%s' "./tests-$lane")" = "1000:1000:1:$bytes" ]
[ "$(sha256sum "./tests-$lane" | cut -d' ' -f1)" = "$expected" ]
[ ! -e /opt/hydracache-performance/0.74/diagnostic-pilot ]
chmod 500 "./tests-$lane"
mkdir -m 700 "run-$lane-$mode"
export TMPDIR="$PWD/run-$lane-$mode"
export HC074_START_MATERIAL_BUNDLE="$PWD/bundle"
printf 'HC074-FIXED-GROUPED-BINARY lane=%s mode=%s bytes=%s sha256=%s\n' "$lane" "$mode" "$bytes" "$expected"
set +e
timeout --signal=TERM --kill-after=5s 600s "./tests-$lane" 'diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::'"$selected" --ignored --exact --nocapture --test-threads=1 > "./$mode-$lane.log" 2>&1
result=$?
set -e
cat "./$mode-$lane.log"
printf 'HC074-FIXED-GROUPED-EXIT lane=%s mode=%s exit=%s\n' "$lane" "$mode" "$result"
preflight
[ "$result" -eq 0 ]
grep -q '1 passed; 0 failed;' "./$mode-$lane.log"
if [ "$mode" = screen ]; then
    [ ! -e .screen-ok ]
    touch .screen-ok
fi
printf 'HC074-FIXED-GROUPED-RUN lane=%s mode=%s completed=true guardian=false send=false\n' "$lane" "$mode"
