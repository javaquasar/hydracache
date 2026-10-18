#!/bin/bash
set -euo pipefail
cd "$1"
source ./guard.sh
exec 9>"$PWD/.driver-lock"
flock -n 9
export HC074_START_MATERIAL_BUNDLE=/tmp/hc074-safety-alarm-20261017.dzh0Im/bundle
export PYTHONDONTWRITEBYTECODE=1
[ "$(sha256sum "$HC074_START_MATERIAL_BUNDLE/timing-controls-074" | cut -d' ' -f1)" = bda1c094f3de7c6d42ec45c757838c7a41a8ff1d63a025c661af84455169a101 ]
no_fixture() {
    for item in /proc/[0-9]*/exe; do
        executable=$(readlink "$item" 2>/dev/null || true)
        case "$executable" in /tmp/hc074-enrollment-frame-20261018.*/tests-*|/tmp/hc074-collector-loss-matrix-20261018.*/tests-*|/tmp/hc074-barred-collector-20261018.*/tests-*|/tmp/hc074-custody-loss-matrix-20261018.*/tests-*) printf 'unexpected_executable=%s\n' "$executable"; return 91 ;; esac
    done
}
no_fixture
sha256sum ./tests-194 ./tests-188 ./guard.sh ./run.sh ./*_owner_loss.py
python3 -m unittest -v test_original_enrollment_owner_loss.py test_original_custody_owner_loss.py
prefix='diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::collector_tests::enrollment_tests::'
for compiler in 194 188; do
    preflight
    printf 'HC074-ENROLLMENT-FRAME-SCREEN compiler=%s begin=true\n' "$compiler"
    "./tests-$compiler" descriptor_plan:: --nocapture --test-threads=1
    "./tests-$compiler" diagnostic_output_preparation::fixed_intent_storage::reserved_state::original_fence:: --skip original_fence_custody_matrix --nocapture --test-threads=1
    "./tests-$compiler" --exact "${prefix}original_enrollment_frame_screen" --ignored --nocapture --test-threads=1
    printf 'HC074-ENROLLMENT-FRAME-SCREEN compiler=%s completed=true\n' "$compiler"
    preflight
done
for compiler in 194 188; do
    preflight
    printf 'HC074-ENROLLMENT-FRAME-COHORT compiler=%s begin=true\n' "$compiler"
    for test in original_enrollment_frame_healthy original_enrollment_frame_refusals faults::original_enrollment_lost_b_notice; do
        "./tests-$compiler" --exact "${prefix}$test" --ignored --nocapture --test-threads=1
    done
    python3 original_enrollment_owner_loss.py --binary "./tests-$compiler" --repeats 3
    printf 'HC074-ENROLLMENT-FRAME-COHORT compiler=%s completed=true\n' "$compiler"
    preflight
done
no_fixture
printf 'HC074-ENROLLMENT-FRAME-HOST completed=true supervisor_unchanged=true no_fixture_executable=true admission=false\n'


