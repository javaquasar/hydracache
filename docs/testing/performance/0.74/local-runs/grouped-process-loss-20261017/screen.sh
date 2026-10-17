#!/bin/sh
set -eu
umask 077
cd "$1"
. ./guard.sh
[ -f ./tests-194 ] && [ ! -L ./tests-194 ]
[ "$(stat -c '%u:%g:%h:%s' ./tests-194)" = '1000:1000:1:18930904' ]
[ "$(sha256sum ./tests-194 | cut -d' ' -f1)" = 2d0fdefbc296af3ab8c069fe0ff6e259a22cd65d302c6b3f03b97be5e779e2ef ]
chmod 500 ./tests-194
mkdir -m 700 run-194-screen
export TMPDIR="$PWD/run-194-screen"
export HC074_START_MATERIAL_BUNDLE="$PWD/bundle"
printf 'HC074-GROUPED-PROCESS-BINARY lane=194 mode=screen bytes=18930904 sha256=2d0fdefbc296af3ab8c069fe0ff6e259a22cd65d302c6b3f03b97be5e779e2ef\n'
set +e
timeout --signal=TERM --kill-after=5s 1200s ./tests-194 'diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::crash_tests::grouped_process_tests::original_grouped_process_screen' --ignored --exact --nocapture --test-threads=1 > ./screen-194.log 2>&1
result=$?
set -e
cat ./screen-194.log
printf 'HC074-GROUPED-PROCESS-EXIT lane=194 mode=screen exit=%s\n' "$result"
preflight
[ "$result" -eq 0 ]
grep -q '1 passed; 0 failed;' ./screen-194.log
grep -q 'HC074-GROUPED-PROCESS-SCREEN sigkills=1 controls=1 inspections=4 completed=true' ./screen-194.log
printf 'HC074-GROUPED-PROCESS-RUN lane=194 mode=screen completed=true admission=false\n'
