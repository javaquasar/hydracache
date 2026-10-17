#!/bin/sh
set -eu
umask 077
cd "$1"
. ./guard.sh
mkdir -m 700 single-flight
[ ! -e /opt/hydracache-performance/0.74/diagnostic-pilot ]
strace --version
printf 'HC074-SYSCALL-ENV perf_event_paranoid=%s\n' "$(cat /proc/sys/kernel/perf_event_paranoid)"
prior=/tmp/hc074-controller-cost-20261017.uYyrhr
for lane in 194 188; do
    [ -f "$prior/tests-$lane" ] && [ ! -L "$prior/tests-$lane" ]
    cp --no-clobber "$prior/tests-$lane" "./tests-$lane"
    chmod 500 "./tests-$lane"
done
[ "$(stat -c '%u:%g:%h:%s' ./tests-194)" = 1000:1000:1:18764168 ]
[ "$(stat -c '%u:%g:%h:%s' ./tests-188)" = 1000:1000:1:18450456 ]
[ "$(sha256sum ./tests-194 | cut -d' ' -f1)" = 6968ce30bab424fff57b36b1f298b80929f0a0188ee98e4c1083fad6f017de37 ]
[ "$(sha256sum ./tests-188 | cut -d' ' -f1)" = 26f7e196bdc952164b0ae38a72608bf5f1f4c022c8060088db48880c7fac8aa9 ]
sha256sum ./tests-194 ./tests-188
test_name=diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::pipe_transport_tests::safety_alarm_tests::leaf_cost_tests::component_cost_tests::original_controller_component_cost_controls
for invocation in 194-plain 194-traced 188-traced 188-plain; do
    lane=${invocation%-*}
    mode=${invocation#*-}
    preflight
    mkdir -m 700 "tmp-$invocation"
    export TMPDIR="$PWD/tmp-$invocation"
    unset HC074_START_MATERIAL_BUNDLE
    set +e
    if [ "$mode" = traced ]; then
        timeout --signal=TERM --kill-after=5s 120s strace -f -c -w -o "syscalls-$lane.log" "./tests-$lane" "$test_name" --ignored --exact --nocapture --test-threads=1 > "run-$invocation.log" 2>&1
    else
        timeout --signal=TERM --kill-after=5s 120s "./tests-$lane" "$test_name" --ignored --exact --nocapture --test-threads=1 > "run-$invocation.log" 2>&1
    fi
    result=$?
    set -e
    printf 'HC074-SYSCALL-EXIT invocation=%s exit=%s\n' "$invocation" "$result"
    preflight
    cat "run-$invocation.log"
    [ "$result" -eq 0 ]
    grep -q '1 passed; 0 failed;' "run-$invocation.log"
    [ "$(grep -c 'HC074-COMPONENT-CONTROL sample=' "run-$invocation.log")" -eq 3 ]
    [ "$(grep -c 'HC074-COMPONENT-COST sample=' "run-$invocation.log")" -eq 30 ]
    printf 'HC074-SYSCALL-CONTROL invocation=%s samples=3 rows=30 production_changed=false schedule_changed=false admission=false\n' "$invocation"
done
printf 'HC074-SYSCALL-FAMILY invocations=4 completed=true\n'
