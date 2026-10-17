#!/bin/sh
set -eu
umask 077
cd /tmp/hc074-safety-alarm-20261017.dzh0Im
. ./guard.sh
lane="$1"
case "$lane" in
194) expected=97f61c58f60f7f93f0c2b7697037c82f5fd3a39d80660bb47b2b715b4aec6a0a; bytes=18743224 ;;
188) expected=ca5e670486023429798fc9e2633dc0a74e4b173d2e33bc1ff9e61384eb72ebc6; bytes=18343832 ;;
*) exit 81 ;;
esac
[ -f "./tests-$lane" ]
[ ! -L "./tests-$lane" ]
[ "$(stat -c '%u:%g:%h' "./tests-$lane")" = 1000:1000:1 ]
[ "$(sha256sum "./tests-$lane" | cut -d' ' -f1)" = "$expected" ]
[ "$(stat -c %s "./tests-$lane")" -eq "$bytes" ]
chmod 500 "./tests-$lane"
printf 'HC074-SAFETY-ALARM-BINARY lane=%s bytes=%s sha256=%s\n' "$lane" "$bytes" "$expected"
[ ! -e "./run-$lane" ]
mkdir -m 700 "./run-$lane"
export TMPDIR="$PWD/run-$lane"
export HC074_START_MATERIAL_BUNDLE="$PWD/bundle"
prefix='diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::'
timeout 120s "./tests-$lane" "${prefix}original_safety_alarm_joined_healthy" --ignored --exact --nocapture --test-threads=1 > "./healthy-$lane.log" 2>&1
cat "./healthy-$lane.log"
grep -q '1 passed; 0 failed;' "./healthy-$lane.log"
preflight
timeout 300s "./tests-$lane" "${prefix}original_safety_alarm_joined_refusals" --ignored --exact --nocapture --test-threads=1 > "./refusals-$lane.log" 2>&1
cat "./refusals-$lane.log"
grep -q '1 passed; 0 failed;' "./refusals-$lane.log"
preflight
printf 'HC074-SAFETY-ALARM-RUN lane=%s completed=true guardian=false send=false\n' "$lane"
