#!/bin/bash
set -euo pipefail
cd "$1"
source ./guard.sh
exec 9>"$PWD/.driver-lock"
flock -n 9
export HC074_START_MATERIAL_BUNDLE=/tmp/hc074-safety-alarm-20261017.dzh0Im/bundle
[ "$(sha256sum "$HC074_START_MATERIAL_BUNDLE/timing-controls-074" | cut -d' ' -f1)" = bda1c094f3de7c6d42ec45c757838c7a41a8ff1d63a025c661af84455169a101 ]
previous=/tmp/hc074-barred-collector-20261018.ET1hWc/tests-188
current="$PWD/tests-188"
[ "$(sha256sum "$previous" | cut -d' ' -f1)" = 33ede34173e7b73c53f09f3ef55f3ff971e55a7398b2dcf224f67c03c71d4796 ]
[ "$(sha256sum "$current" | cut -d' ' -f1)" = 767db888bc9d4b980a2eab002314a9ddd5f234e498656c6a1bf38e0a83fc9e4f ]
sha256sum "$previous" "$current" ./healthy-attribution.sh
sample=0
for role in previous current current previous; do
    preflight
    sample=$((sample+1))
    image="$current"
    if [ "$role" = previous ]; then image="$previous"; fi
    printf 'HC074-COLLECTOR-HEALTHY-ATTRIBUTION seed=0x0740c518 sample=%s role=%s begin=true\n' "$sample" "$role"
    set +e
    "$image" --exact diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::collector_tests::original_barred_collector_screen --ignored --nocapture --test-threads=1
    result=$?
    set -e
    printf 'HC074-COLLECTOR-HEALTHY-ATTRIBUTION seed=0x0740c518 sample=%s role=%s exit=%s admission=false\n' "$sample" "$role" "$result"
    [ "$result" -eq 0 ] || [ "$result" -eq 101 ]
    preflight
done
printf 'HC074-COLLECTOR-HEALTHY-ATTRIBUTION completed=true samples=4 stopped_matrix_not_replaced=true admission=false\n'
