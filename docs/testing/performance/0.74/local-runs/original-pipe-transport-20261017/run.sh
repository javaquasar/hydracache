#!/bin/sh
set -eu
umask 077
cd /tmp/hc074-pipe-transport-20261017.b9l0y9
. ./guard.sh
lane="$1"
case "$lane" in
194) expected=8f31b62d578544360cecbfb483fb3971ef673a6d14f8464b2bddb3d2853f9963; bytes=18629416 ;;
188) expected=f6873fca94df31a0375ac1849a9228b2d2a911047e61edef81cfdf7053f97f9d; bytes=18293896 ;;
*) exit 81 ;;
esac
[ "$(sha256sum "./tests-$lane" | cut -d' ' -f1)" = "$expected" ]
[ "$(stat -c %s "./tests-$lane")" -eq "$bytes" ]
[ ! -L "./tests-$lane" ]
[ "$(stat -c '%u:%g' "./tests-$lane")" = 1000:1000 ]
chmod 500 "./tests-$lane"
printf 'HC074-PIPE-TRANSPORT-BINARY lane=%s bytes=%s sha256=%s\n' "$lane" "$bytes" "$expected"
[ ! -e "./run-final-$lane" ]
mkdir -m 700 "./run-final-$lane"
export TMPDIR="$PWD/run-final-$lane"
export HC074_START_MATERIAL_BUNDLE="$PWD/bundle"
prefix='diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::'
timeout 120s "./tests-$lane" "${prefix}original_pipe_transport_joined_healthy" --ignored --exact --nocapture --test-threads=1 > "./healthy-$lane.log" 2>&1
cat "./healthy-$lane.log"
grep -q '1 passed; 0 failed;' "./healthy-$lane.log"
preflight
timeout 240s "./tests-$lane" "${prefix}original_pipe_transport_joined_refusals" --ignored --exact --nocapture --test-threads=1 > "./refusals-$lane.log" 2>&1
cat "./refusals-$lane.log"
grep -q '1 passed; 0 failed;' "./refusals-$lane.log"
preflight
printf 'HC074-PIPE-TRANSPORT-RUN lane=%s completed=true send=false\n' "$lane"
