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
    preflight
    printf 'HC074-BARRED-CUSTODY-COHORT compiler=%s begin=true\n' "$compiler"
    sha256sum "tests-$compiler"
    timeout 120 "./tests-$compiler" descriptor_plan:: --nocapture --test-threads=1
    printf 'HC074-BARRED-CUSTODY-PHASE name=healthy begin=true\n'
    timeout 600 "./tests-$compiler" --exact diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::original_barred_custody_healthy --ignored --nocapture --test-threads=1
    printf 'HC074-BARRED-CUSTODY-PHASE name=healthy completed=true\n'
    printf 'HC074-BARRED-CUSTODY-PHASE name=refusals begin=true\n'
    timeout 1200 "./tests-$compiler" --exact diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::original_barred_custody_refusals --ignored --nocapture --test-threads=1
    printf 'HC074-BARRED-CUSTODY-PHASE name=refusals completed=true\n'
    printf 'HC074-BARRED-CUSTODY-PHASE name=process_matrix begin=true\n'
    timeout 1800 "./tests-$compiler" --exact diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::original_barred_custody_process_matrix --ignored --nocapture --test-threads=1
    printf 'HC074-BARRED-CUSTODY-PHASE name=process_matrix completed=true\n'
    printf 'HC074-BARRED-CUSTODY-COHORT compiler=%s completed=true\n' "$compiler"
    preflight
done
for item in /proc/[0-9]*/exe; do
    executable=$(readlink "$item" 2>/dev/null || true)
    case "$executable" in "$PWD"/tests-*) printf 'HC074-BARRED-CUSTODY unexpected_executable=%s\n' "$executable"; exit 91 ;; esac
done
printf 'HC074-BARRED-CUSTODY-HOST completed=true supervisor_unchanged=true no_fixture_executable=true admission=false\n'
