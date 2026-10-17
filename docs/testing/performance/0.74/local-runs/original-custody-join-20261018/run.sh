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
    printf 'HC074-CUSTODY-JOIN-COHORT compiler=%s\n' "$compiler"
    sha256sum "tests-$compiler"
    timeout 120 "./tests-$compiler" diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence:: --skip original_fence_custody_matrix --nocapture --test-threads=1
    timeout 120 "./tests-$compiler" custody_process_tests --skip original_custody_join_process_matrix --skip original_custody_join_process_screen --nocapture --test-threads=1
    printf 'HC074-CUSTODY-JOIN-PHASE name=healthy begin=true\n'
    timeout 600 "./tests-$compiler" --exact diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::original_custody_join_healthy --ignored --nocapture --test-threads=1
    printf 'HC074-CUSTODY-JOIN-PHASE name=healthy completed=true\n'
    printf 'HC074-CUSTODY-JOIN-PHASE name=refusals begin=true\n'
    timeout 900 "./tests-$compiler" --exact diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::original_custody_join_refusals --ignored --nocapture --test-threads=1
    printf 'HC074-CUSTODY-JOIN-PHASE name=refusals completed=true\n'
    printf 'HC074-CUSTODY-JOIN-PHASE name=later begin=true\n'
    timeout 600 "./tests-$compiler" --exact diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::original_custody_join_later_refusals --ignored --nocapture --test-threads=1
    printf 'HC074-CUSTODY-JOIN-PHASE name=later completed=true\n'
    printf 'HC074-CUSTODY-JOIN-PHASE name=process begin=true\n'
    timeout 1800 "./tests-$compiler" --exact diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::crash_tests::fixed_grouped_process_tests::custody_process_tests::original_custody_join_process_matrix --ignored --nocapture --test-threads=1
    printf 'HC074-CUSTODY-JOIN-PHASE name=process completed=true\n'
    printf 'HC074-CUSTODY-JOIN-COHORT compiler=%s completed=true\n' "$compiler"
    preflight
done
for item in /proc/[0-9]*/exe; do
    executable=$(readlink "$item" 2>/dev/null || true)
    case "$executable" in "$PWD"/tests-*) printf 'HC074-CUSTODY-JOIN unexpected_executable=%s\n' "$executable"; exit 91 ;; esac
done
[ ! -e /opt/hydracache-performance/0.74/diagnostic-pilot ]
printf 'HC074-CUSTODY-JOIN-HOST completed=true supervisor_unchanged=true no_fixture_executable=true admission=false\n'

