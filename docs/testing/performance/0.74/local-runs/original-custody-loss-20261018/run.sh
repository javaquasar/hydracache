#!/bin/bash
set -euo pipefail
cd "$1"
source ./guard.sh
exec 9>"$PWD/.driver-lock"
flock -n 9
export HC074_START_MATERIAL_BUNDLE=/tmp/hc074-safety-alarm-20261017.dzh0Im/bundle
[ "$(sha256sum "$HC074_START_MATERIAL_BUNDLE/timing-controls-074" | cut -d' ' -f1)" = bda1c094f3de7c6d42ec45c757838c7a41a8ff1d63a025c661af84455169a101 ]
sha256sum ./tests-194 ./tests-188 ./guard.sh ./run.sh ./original_custody_owner_loss.py ./test_original_custody_owner_loss.py
python3 -m unittest -v test_original_custody_owner_loss.py
for compiler in 194 188; do
    preflight
    printf 'HC074-CUSTODY-LOSS-COHORT compiler=%s begin=true\n' "$compiler"
    "./tests-$compiler" descriptor_plan:: --nocapture --test-threads=1
    printf 'HC074-CUSTODY-LOSS-PHASE name=screen begin=true\n'
    "./tests-$compiler" --exact diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::original_barred_custody_screen --ignored --nocapture --test-threads=1
    printf 'HC074-CUSTODY-LOSS-PHASE name=screen completed=true\n'
    printf 'HC074-CUSTODY-LOSS-PHASE name=notice begin=true\n'
    "./tests-$compiler" --exact diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::loss_tests::original_custody_lost_pending_notice --ignored --nocapture --test-threads=1
    printf 'HC074-CUSTODY-LOSS-PHASE name=notice completed=true\n'
    printf 'HC074-CUSTODY-LOSS-PHASE name=owner begin=true\n'
    timeout --signal=INT --kill-after=30s 1200 python3 original_custody_owner_loss.py --binary "./tests-$compiler" --repeats 3
    printf 'HC074-CUSTODY-LOSS-PHASE name=owner completed=true\n'
    printf 'HC074-CUSTODY-LOSS-COHORT compiler=%s completed=true\n' "$compiler"
    preflight
done
for item in /proc/[0-9]*/exe; do
    executable=$(readlink "$item" 2>/dev/null || true)
    case "$executable" in "$PWD"/tests-*) printf 'unexpected_executable=%s\n' "$executable"; exit 91 ;; esac
done
printf 'HC074-CUSTODY-LOSS-HOST completed=true supervisor_unchanged=true no_fixture_executable=true admission=false\n'
