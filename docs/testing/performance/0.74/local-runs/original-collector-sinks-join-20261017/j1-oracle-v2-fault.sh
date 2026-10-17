#!/bin/sh
# Authorized nonroot temporary correctness fixtures only; never install/start product.
set -eu
[ "$(id -u)" -eq 1000 ]
[ "$(id -g)" -eq 1000 ]
root=$(pwd -P)
case "$root" in /tmp/hc074-output-join-20261017.??????) ;; *) exit 80 ;; esac
[ "$(stat -c '%a:%u:%g' "$root")" = '700:1000:1000' ]
umask 077
preflight() {
    [ "$(cat /proc/sys/kernel/random/boot_id)" = e81671d6-bf23-4ebf-b8fe-5dad1f45866f ]
    [ "$(systemctl show hydracache-performance-supervisor-074.service -p ActiveState --value)" = active ]
    [ "$(systemctl show hydracache-performance-supervisor-074.service -p SubState --value)" = running ]
    [ "$(systemctl show hydracache-performance-supervisor-074.service -p MainPID --value)" = 8839 ]
    [ "$(systemctl show hydracache-performance-supervisor-074.service -p NRestarts --value)" = 0 ]
    [ "$(systemctl show hydracache-performance-supervisor-074.service -p ExecMainStartTimestamp --value)" = 'Tue 2026-10-06 15:03:40 CEST' ]
    for path in \
      /var/lib/hydracache-performance/campaigns/active-campaign \
      /var/lib/hydracache-performance/campaigns/active-diagnostic.json \
      /var/lib/hydracache-performance/campaigns/.active-diagnostic.pending \
      /var/lib/hydracache-performance/campaigns/diagnostic-requests.json \
      /var/lib/hydracache-performance/campaigns/diagnostic-requests-v1.json \
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

printf '%s\n' 'c9d0c16b726a326b066d758cae7e12da2e7e4e0914a5a1e674f449c4a6eec1ad  output-j1v2-matrix-194' '699e31d7568911f46bc0699924561ea9242c7d984f4466b050e2e250b564c0b0  output-j1v2-matrix-188' | sha256sum --check
for lane in 194 188; do
 for family in sink-fault join-fault; do
  preflight
  case "$family" in
   sink-fault) test_name='diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::original_collector_sinks_joined_fault_and_live_refusal' ;;
   join-fault) test_name='diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::join_fault_tests::original_output_join_all_effects_live_refusal' ;;
   *) exit 81 ;;
  esac
  mkdir -m 700 "scratch-j1v2-$family-$lane"
  result=0
  TMPDIR="$root/scratch-j1v2-$family-$lane" /usr/bin/timeout --signal=TERM --kill-after=5s 1800s /usr/bin/nice -n 10 "$root/output-j1v2-matrix-$lane" "$test_name" --ignored --exact --nocapture --test-threads=1 >"j1v2-$family-$lane.log" 2>&1 || result=$?
  tail -n 12 "j1v2-$family-$lane.log"
  preflight
  printf 'HC074-OUTPUT-J1V2 lane=%s family=%s exit=%s product=false\n' "$lane" "$family" "$result"
  [ "$result" -eq 0 ] || exit "$result"
 done
done
printf 'HC074-OUTPUT-J1V2 joined_complete=true compilers=2 product=false\n'
# End of authorized script.
