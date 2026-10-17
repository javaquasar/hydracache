#!/bin/sh
set -eu
umask 077
cd "$1"
. ./guard.sh
lane="$2"
mode="$3"
case "$lane" in
194) expected=590eb3b4de779f161610929dfde204aa19d8510e103300fcbba1dd736d8400d5; bytes=18712568 ;;
188) expected=8dcd36e667db2e591e5c608fcbcac87f87fa6c56b008678b059e880b2d99d7d9; bytes=18573624 ;;
*) exit 81 ;;
esac
case "$mode" in screen|healthy|refusals) ;; *) exit 82 ;; esac
[ -f "./tests-$lane" ] && [ ! -L "./tests-$lane" ]
[ "$(stat -c '%u:%g:%h:%s' "./tests-$lane")" = "1000:1000:1:$bytes" ]
[ "$(sha256sum "./tests-$lane" | cut -d' ' -f1)" = "$expected" ]
chmod 500 "./tests-$lane"
mkdir -m 700 "run-$lane-$mode"
export TMPDIR="$PWD/run-$lane-$mode"
export HC074_START_MATERIAL_BUNDLE="$PWD/bundle"
printf 'HC074-GROUPED-BINARY lane=%s mode=%s bytes=%s sha256=%s\n' "$lane" "$mode" "$bytes" "$expected"
set +e
timeout --signal=TERM --kill-after=5s 600s "./tests-$lane" 'diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::grouped_tests::'"original_grouped_preparation_$mode" --ignored --exact --nocapture --test-threads=1 > "./$mode-$lane.log" 2>&1
result=$?
set -e
cat "./$mode-$lane.log"
printf 'HC074-GROUPED-EXIT lane=%s mode=%s exit=%s\n' "$lane" "$mode" "$result"
preflight
[ "$result" -eq 0 ]
grep -q '1 passed; 0 failed;' "./$mode-$lane.log"
printf 'HC074-GROUPED-RUN lane=%s mode=%s completed=true guardian=false send=false\n' "$lane" "$mode"
