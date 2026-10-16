#!/bin/sh
# Authorized nonroot temporary correctness fixtures only; never install/start product.
set -eu
[ "$(id -u)" -eq 1000 ]
[ "$(id -g)" -eq 1000 ]
root=$(pwd -P)
case "$root" in /tmp/hc074-original-sinks-20261017.??????) ;; *) exit 80 ;; esac
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
    printf 'HC074-ORIGINAL-SINK-SERVER preflight=ok supervisor_pid=8839 restarts=0\n'
}

preflight
printf '%s\n' \
 'c8d8088553e96d70cd56f32cd9174ee841a2309892a84069be51097d0185435b  original-sinks-194' \
 '845d22d28e973ce237a8bbef501d1f72fc89ad32ab5008d4ff3816b35ed98e54  original-sinks-188' | sha256sum --check
for lane in 194 188; do
    for family in joined crash; do
        preflight
        case "$family" in
          joined) test_name=diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::original_collector_sinks_joined_fault_and_live_refusal ;;
          crash) test_name=diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::crash_tests::original_sink_process_crash_all_stages_and_lost_ack ;;
          *) exit 81 ;;
        esac
        mkdir -m 700 "scratch-$lane-$family"
        result=0
        TMPDIR="$root/scratch-$lane-$family" /usr/bin/timeout --signal=TERM --kill-after=5s 1800s /usr/bin/nice -n 10 "$root/original-sinks-$lane" "$test_name" --ignored --exact --nocapture --test-threads=1 >"$family-$lane.log" 2>&1 || result=$?
        tail -n 12 "$family-$lane.log"
        printf 'HC074-ORIGINAL-SINK-SERVER lane=%s family=%s exit=%s synthetic_empty_outputs=true product=false\n' "$lane" "$family" "$result"
        preflight
        [ "$result" -eq 0 ] || exit "$result"
    done
done
printf 'HC074-ORIGINAL-SINK-SERVER complete=true supervisor_unchanged=true product=false\n'
