#!/bin/bash
set -euo pipefail
cd -- "$1"
source ./guard.sh
no_fixture
export HC074_START_MATERIAL_BUNDLE="$root/bundle"
sha256sum ./tests-194 ./tests-188 ./guard.sh ./run.sh ./bundle/*
for compiler in 194 188; do
    preflight
    for name in original_manager_join_frame_controls original_manager_join_frame_refusals original_manager_join_nonroot_fixed_refusal; do
        preflight
        "./tests-$compiler" --exact "diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::collector_tests::enrollment_tests::manager_tests::$name" --ignored --nocapture --test-threads=1
        preflight
        no_fixture
    done
    printf 'HC074-ORIGINAL-MANAGER-HOST compiler=%s completed=true start=false\n' "$compiler"
done
preflight
no_fixture
printf 'HC074-ORIGINAL-MANAGER-HOST completed=true supervisor_unchanged=true no_fixture_executable=true admission=false\n'

