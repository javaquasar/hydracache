#!/bin/bash
set -euo pipefail
cd -- "$1"
source ./guard.sh
no_fixture
printf 'HC074-LIVE-WALK-ADMISSION start=false install=false supervisor_changed=false\n'
sha256sum ./tests-194 ./tests-188 ./guard.sh ./run.sh
for compiler in 194 188; do
    preflight
    printf 'HC074-LIVE-WALK-HOST compiler=%s begin=true\n' "$compiler"
    "./tests-$compiler" --exact "diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::leaf_cost_tests::component_cost_tests::live_walk_tests::original_live_worker_walk_attribution" --ignored --nocapture --test-threads=1
    preflight
    no_fixture
    printf 'HC074-LIVE-WALK-HOST compiler=%s completed=true admission=false\n' "$compiler"
done
preflight
no_fixture
printf 'HC074-LIVE-WALK-HOST completed=true supervisor_unchanged=true no_fixture_executable=true admission=false\n'
