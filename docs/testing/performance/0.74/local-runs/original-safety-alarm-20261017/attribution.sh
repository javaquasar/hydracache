#!/bin/sh
set -eu
umask 077
cd /tmp/hc074-safety-alarm-20261017.dzh0Im
. ./guard.sh
[ -f ./attribution-194 ]
[ ! -L ./attribution-194 ]
[ "$(stat -c '%u:%g:%h' ./attribution-194)" = 1000:1000:1 ]
[ "$(sha256sum ./attribution-194 | cut -d' ' -f1)" = fc7f33c25dea982703f97b4b26db2b7938ab8f6aa1b99add49dd0175613eb57d ]
chmod 500 ./attribution-194
[ ! -e ./attribution-run ]
mkdir -m 700 ./attribution-run
export TMPDIR="$PWD/attribution-run"
export HC074_START_MATERIAL_BUNDLE="$PWD/bundle"
timeout 180s ./attribution-194 'diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::original_safety_alarm_cost_attribution' --ignored --exact --nocapture --test-threads=1 > ./attribution.log 2>&1
cat ./attribution.log
grep -q '1 passed; 0 failed;' ./attribution.log
preflight
