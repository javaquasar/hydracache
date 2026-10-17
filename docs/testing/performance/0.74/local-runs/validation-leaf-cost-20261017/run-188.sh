#!/bin/sh
set -eu
umask 077
cd /tmp/hc074-leaf-cost-20261017.mF2pzV
. ./guard.sh
[ -f ./tests-188 ] && [ ! -L ./tests-188 ]
[ "$(stat -c '%u:%g:%h:%s' ./tests-188)" = 1000:1000:1:18238968 ]
[ "$(sha256sum ./tests-188 | cut -d' ' -f1)" = 08fa25a9cb8321b7cb4d51e1c8972edeb845850b17f2fd48aa3161613b1b6cd9 ]
chmod 500 ./tests-188
[ ! -e ./run-188 ]
mkdir -m 700 ./run-188
export TMPDIR="$PWD/run-188"
unset HC074_START_MATERIAL_BUNDLE
set +e
timeout 180s ./tests-188 'diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::leaf_cost_tests::original_validation_leaf_cost_attribution' --ignored --exact --nocapture --test-threads=1 > ./run-188.log 2>&1
result=$?
set -e
cat ./run-188.log
preflight
[ "$result" -eq 0 ]
grep -q '1 passed; 0 failed;' ./run-188.log
[ "$(grep -c 'HC074-LEAF-CONTROL sample=' ./run-188.log)" -eq 3 ]
printf 'HC074-LEAF-HOST lane=188 samples=3 production_changed=false schedule_changed=false\n'
