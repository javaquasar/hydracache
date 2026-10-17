#!/bin/sh
set -eu
umask 077
cd /tmp/hc074-start-material-20261017.vLX1GL
. ./guard.sh
[ "$(sha256sum ./tests-attribution-194 | cut -d' ' -f1)" = "8928fc339406b96c5ef5aa87fd085f8773b838db49b4efce81d5696102caf170" ]
[ ! -e ./run-attribution ]
mkdir -m 700 ./run-attribution
export TMPDIR="$PWD/run-attribution"
export HC074_START_MATERIAL_BUNDLE="$PWD/bundle"
timeout 120s ./tests-attribution-194 'diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::original_start_material_cost_attribution' --ignored --exact --nocapture --test-threads=1 > ./attribution-194.log 2>&1
cat ./attribution-194.log
preflight
# End.
