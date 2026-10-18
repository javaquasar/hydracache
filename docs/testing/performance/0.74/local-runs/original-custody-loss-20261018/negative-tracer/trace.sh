#!/bin/bash
set -euo pipefail
cd /tmp/hc074-custody-loss-matrix-20261018.6G59my
source ./guard.sh
exec 9>"$PWD/.driver-lock"
flock -n 9
export HC074_START_MATERIAL_BUNDLE=/tmp/hc074-safety-alarm-20261017.dzh0Im/bundle
strace --version | head -1
sha256sum ./tests-194 ./tests-188 ./trace.sh
for compiler in 194 188; do
    preflight
    printf 'HC074-CUSTODY-LOSS-TRACE compiler=%s begin=true\n' "$compiler"
    strace --seccomp-bpf -f -s 16 -e trace=recvfrom -o "pending-recv-$compiler.log" "./tests-$compiler" --exact diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::custody_tests::descriptor_plan_tests::barred_custody_tests::loss_tests::original_custody_lost_pending_notice --ignored --nocapture --test-threads=1
    printf 'HC074-CUSTODY-LOSS-TRACE compiler=%s completed=true\n' "$compiler"
    preflight
done
printf 'HC074-CUSTODY-LOSS-TRACE completed=true product=false\n'
