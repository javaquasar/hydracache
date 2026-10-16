#!/bin/sh
# Exact authorized nonroot test runner; never an installation or supervisor entry.
set -eu
[ "$(id -u)" -eq 1000 ]
[ "$(id -g)" -eq 1000 ]
root=$(pwd -P)
case "$root" in /tmp/hc074-starting-framed-20261016.??????) ;; *) exit 80 ;; esac
[ "$(stat -c '%a:%u:%g' "$root")" = '700:1000:1000' ]
umask 077
preflight() {
    [ "$(cat /proc/sys/kernel/random/boot_id)" = e81671d6-bf23-4ebf-b8fe-5dad1f45866f ]
    [ "$(systemctl show hydracache-performance-supervisor-074.service -p ActiveState --value)" = active ]
    [ "$(systemctl show hydracache-performance-supervisor-074.service -p SubState --value)" = running ]
    [ "$(systemctl show hydracache-performance-supervisor-074.service -p MainPID --value)" = 8839 ]
    [ "$(systemctl show hydracache-performance-supervisor-074.service -p NRestarts --value)" = 0 ]
    for path in \
      /var/lib/hydracache-performance/campaigns/active-campaign \
      /var/lib/hydracache-performance/campaigns/active-diagnostic.json \
      /var/lib/hydracache-performance/campaigns/.active-diagnostic.pending \
      /var/lib/hydracache-performance/campaigns/diagnostic-requests.json \
      /var/lib/hydracache-performance/campaigns/.diagnostic-requests.pending \
      /var/lib/hydracache-performance/campaign-lifecycle-smoke-v1.json \
      /var/lib/hydracache-performance/controller-loss-smoke-v1.json \
      /var/lib/hydracache-performance/diagnostic-intents-074 \
      /var/lib/hydracache-performance/worker-authority-074; do
        sudo -n test ! -e "$path"
        sudo -n test ! -L "$path"
    done
    [ "$(sudo -n sha256sum /opt/hydracache-perf/bin/hydracache-long-run-supervisor-074 | cut -d' ' -f1)" = f797629f65385ebfd4d94577a43a2d1d020f71c1e841028dad7d02a51b47ad62 ]
    [ "$(sudo -n sha256sum /var/lib/hydracache-performance/provisioning-receipt-074.json | cut -d' ' -f1)" = b314953d4deedaeaade3e24468da4844b7f13d348a5d77b8e8f41c672ff44525 ]
    printf 'HC074-STARTING-FRAMED-SERVER preflight=ok supervisor_pid=8839 restarts=0\n'
}
preflight
printf '%s\n' \
 'afaf9350617a648dbf3414766a3fc87a2582481d2681932e4c6878433b9b0726  starting-framed-194' \
 'b1f79ba78644c40814d5d32967477b55141b5fcca53421a26dc727c48d0f56c3  starting-framed-188' | sha256sum --check
prefix=diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests
# No crash campaign unless BOTH independently built fault lanes pass.
for phase in fault crash; do
    if [ "$phase" = fault ]; then
        test_name="$prefix::collector_starting_publication_boundaries_and_live_refusal"
    else
        test_name="$prefix::crash_tests::collector_starting_process_crash_all_stages_and_lost_ack"
    fi
    for lane in 194 188; do
        preflight
        mkdir -m 700 "scratch-$phase-$lane"
        result=0
        TMPDIR="$root/scratch-$phase-$lane" /usr/bin/timeout --signal=TERM --kill-after=5s 1800s /usr/bin/nice -n 10 "$root/starting-framed-$lane" "$test_name" --ignored --exact --nocapture --test-threads=1 >"$phase-$lane.log" 2>&1 || result=$?
        tail -n 10 "$phase-$lane.log"
        printf 'HC074-STARTING-FRAMED-SERVER phase=%s lane=%s exit=%s installation=false product=false\n' "$phase" "$lane" "$result"
        preflight
        [ "$result" -eq 0 ] || exit "$result"
    done
done
preflight
