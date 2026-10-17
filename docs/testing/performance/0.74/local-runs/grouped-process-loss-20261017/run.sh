#!/bin/sh
set -eu
umask 077
cd "$1"
. ./guard.sh
lane="$2"
case "$lane" in
194) expected=2d0fdefbc296af3ab8c069fe0ff6e259a22cd65d302c6b3f03b97be5e779e2ef; bytes=18930904 ;;
188) expected=0c092111ec656473f488cfaa94c10e3883fda159d6a886e7d42839be16d2c70e; bytes=18631528 ;;
*) exit 81 ;;
esac
[ -f "./tests-$lane" ] && [ ! -L "./tests-$lane" ]
[ "$(stat -c '%u:%g:%h:%s' "./tests-$lane")" = "1000:1000:1:$bytes" ]
[ "$(sha256sum "./tests-$lane" | cut -d' ' -f1)" = "$expected" ]
chmod 500 "./tests-$lane"
mkdir -m 700 "run-$lane-matrix"
export TMPDIR="$PWD/run-$lane-matrix"
export HC074_START_MATERIAL_BUNDLE="$PWD/bundle"
printf 'HC074-GROUPED-PROCESS-BINARY lane=%s mode=matrix bytes=%s sha256=%s\n' "$lane" "$bytes" "$expected"
set +e
timeout --signal=TERM --kill-after=5s 1200s "./tests-$lane" 'diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::crash_tests::grouped_process_tests::original_grouped_process_matrix' --ignored --exact --nocapture --test-threads=1 > "./matrix-$lane.log" 2>&1
result=$?
set -e
cat "./matrix-$lane.log"
printf 'HC074-GROUPED-PROCESS-EXIT lane=%s mode=matrix exit=%s\n' "$lane" "$result"
preflight
[ "$result" -eq 0 ]
grep -q '1 passed; 0 failed;' "./matrix-$lane.log"
[ "$(grep -c 'HC074-GROUPED-PROCESS-SUMMARY seed=' "./matrix-$lane.log")" -eq 3 ]
printf 'HC074-GROUPED-PROCESS-RUN lane=%s mode=matrix completed=true admission=false\n' "$lane"
