#!/bin/sh
# Only the two refusal expectations differ from the original measured harness.
set -eu
umask 077
cd "$1"
. ./guard.sh
lane="$2"
expected="$3"
bytes="$4"
case "$lane" in 194|188) ;; *) exit 81 ;; esac
case "$expected" in *[!0-9a-f]*|'') exit 82 ;; esac
[ "${#expected}" -eq 64 ]
case "$bytes" in *[!0-9]*|'') exit 83 ;; esac
exec 9>"$PWD/.cohort.lock"
flock -n 9
[ -f .screen-ok ] && [ ! -L .screen-ok ]
[ -f "./tests-$lane-origin" ] && [ ! -L "./tests-$lane-origin" ]
[ "$(stat -c '%u:%g:%h:%s' "./tests-$lane-origin")" = "1000:1000:1:$bytes" ]
[ "$(sha256sum "./tests-$lane-origin" | cut -d' ' -f1)" = "$expected" ]
[ ! -e /opt/hydracache-performance/0.74/diagnostic-pilot ]
chmod 500 "./tests-$lane-origin"
mkdir -m 700 "run-$lane-origin-corrected"
export TMPDIR="$PWD/run-$lane-origin-corrected"
export HC074_START_MATERIAL_BUNDLE="$PWD/bundle"
printf 'HC074-FIXED-GROUPED-BINARY lane=%s mode=origin bytes=%s sha256=%s cohort=two-refusal-expectations\n' "$lane" "$bytes" "$expected"
set +e
timeout --signal=TERM --kill-after=5s 180s "./tests-$lane-origin" 'diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::fixed_grouped_tests::fixed_grouped_preparation_fixture_cannot_enter_production' --ignored --exact --nocapture --test-threads=1 > "./origin-corrected-$lane.log" 2>&1
result=$?
set -e
cat "./origin-corrected-$lane.log"
printf 'HC074-FIXED-GROUPED-EXIT lane=%s mode=origin exit=%s\n' "$lane" "$result"
preflight
[ "$result" -eq 0 ]
grep -q '1 passed; 0 failed;' "./origin-corrected-$lane.log"
printf 'HC074-FIXED-GROUPED-RUN lane=%s mode=origin completed=true guardian=false send=false\n' "$lane"
