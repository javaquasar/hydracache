#!/bin/bash
set -euo pipefail
cd "$1"
source ./guard.sh
exec 9>"$PWD/.driver-lock"
flock -n 9
export HC074_START_MATERIAL_BUNDLE=/tmp/hc074-safety-alarm-20261017.dzh0Im/bundle
[ ! -e /opt/hydracache-performance/0.74/diagnostic-pilot ]
[ "$(sha256sum "$HC074_START_MATERIAL_BUNDLE/timing-controls-074" | cut -d' ' -f1)" = bda1c094f3de7c6d42ec45c757838c7a41a8ff1d63a025c661af84455169a101 ]
for compiler in 194 188; do
    sha256sum "tests-$compiler"
    printf 'HC074-BARRED-CUSTODY-SCREEN compiler=%s begin=true\n' "$compiler"
    timeout 180 "./tests-$compiler" --exact diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::original_barred_custody_screen --ignored --nocapture --test-threads=1
    preflight
    printf 'HC074-BARRED-CUSTODY-SCREEN compiler=%s completed=true\n' "$compiler"
done
