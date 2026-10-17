#!/bin/sh
set -eu
umask 077
cd /tmp/hc074-safety-alarm-20261017.dzh0Im
. ./guard.sh
[ -f ./diagnostic-final-194 ]
[ ! -L ./diagnostic-final-194 ]
[ "$(stat -c '%u:%g:%h:%s' ./diagnostic-final-194)" = 1000:1000:1:18651160 ]
[ "$(sha256sum ./diagnostic-final-194 | cut -d' ' -f1)" = 60b1685d7d5279337d2a1dff509c2f241775423d23b0b2da9289c1972e25e077 ]
chmod 500 ./diagnostic-final-194
[ ! -e ./diagnostic-final-run ]
mkdir -m 700 ./diagnostic-final-run
export TMPDIR="$PWD/diagnostic-final-run"
export HC074_START_MATERIAL_BUNDLE="$PWD/bundle"
set +e
timeout 180s ./diagnostic-final-194 'diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::original_safety_alarm_cost_attribution' --ignored --exact --nocapture --test-threads=1 > ./diagnostic-final.log 2>&1
result=$?
set -e
cat ./diagnostic-final.log
preflight
[ "$result" -eq 0 ]
grep -q '1 passed; 0 failed;' ./diagnostic-final.log
[ "$(grep -c 'HC074-ALARM-ATTRIBUTION-CLOSED sample=.* accepted=false renewed=false guardian=false send=false' ./diagnostic-final.log)" -eq 3 ]
printf 'HC074-ALARM-DIAGNOSTIC samples=3 admitted=false production_api=false\n'
