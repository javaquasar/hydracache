#!/bin/sh
set -eu
umask 077
cd /tmp/hc074-leaf-cost-20261017.mF2pzV
. ./guard.sh
[ -f ./tests-194 ] && [ ! -L ./tests-194 ]
[ "$(stat -c '%u:%g:%h:%s' ./tests-194)" = 1000:1000:1:18725928 ]
[ "$(sha256sum ./tests-194 | cut -d' ' -f1)" = 7c9c403b3156634aa6095c8ebe2de4b2d49b739f16729e50533cd13855e1710a ]
chmod 500 ./tests-194
[ ! -e ./run-194 ]
mkdir -m 700 ./run-194
export TMPDIR="$PWD/run-194"
unset HC074_START_MATERIAL_BUNDLE
set +e
timeout 180s ./tests-194 'diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::leaf_cost_tests::original_validation_leaf_cost_attribution' --ignored --exact --nocapture --test-threads=1 > ./run-194.log 2>&1
result=$?
set -e
cat ./run-194.log
preflight
[ "$result" -eq 0 ]
grep -q '1 passed; 0 failed;' ./run-194.log
[ "$(grep -c 'HC074-LEAF-CONTROL sample=' ./run-194.log)" -eq 3 ]
printf 'HC074-LEAF-HOST lane=194 samples=3 production_changed=false schedule_changed=false\n'
