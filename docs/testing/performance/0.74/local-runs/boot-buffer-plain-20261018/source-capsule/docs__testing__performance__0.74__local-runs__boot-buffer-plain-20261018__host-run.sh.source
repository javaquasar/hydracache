#!/bin/bash
set -euo pipefail
cd -- "$1"
source ./guard.sh
no_fixture
sha256sum -c SHA256SUMS
export HC074_START_MATERIAL_BUNDLE="$root/bundle"
printf 'HC074-PLAIN-ADMISSION start=false install=false supervisor_changed=false\n'
for compiler in 194 188; do
    for pair in 0 1 2 3 4; do
        case "$pair" in 1|3) roles='Candidate Baseline' ;; *) roles='Baseline Candidate' ;; esac
        for role in $roles; do
            preflight
            no_fixture
            printf 'HC074-PLAIN-INVOCATION compiler=%s pair=%s role=%s begin=true\n' "$compiler" "$pair" "$role"
            "./tests-$role-$compiler" --exact "diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::leaf_cost_tests::component_cost_tests::live_walk_tests::plain_worker_tests::original_plain_worker_batch" --ignored --nocapture --test-threads=1
            preflight
            no_fixture
            printf 'HC074-PLAIN-INVOCATION compiler=%s pair=%s role=%s completed=true\n' "$compiler" "$pair" "$role"
        done
    done
done
for compiler in 194 188; do
    for name in original_grouped_addressed_frame_controls original_grouped_addressed_frame_refusals original_grouped_addressed_nonroot_fixed_refusal; do
        preflight
        no_fixture
        printf 'HC074-PLAIN-PREPARATION compiler=%s test=%s begin=true\n' "$compiler" "$name"
        "./tests-Candidate-$compiler" --exact "diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::collector_tests::enrollment_tests::manager_tests::addressed_tests::$name" --ignored --nocapture --test-threads=1
        preflight
        no_fixture
        printf 'HC074-PLAIN-PREPARATION compiler=%s test=%s completed=true\n' "$compiler" "$name"
    done
done
preflight
no_fixture
printf 'HC074-PLAIN-HOST completed=true supervisor_unchanged=true no_fixture_executable=true start=false\n'
