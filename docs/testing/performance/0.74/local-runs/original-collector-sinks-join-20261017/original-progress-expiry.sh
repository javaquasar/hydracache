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

printf '%s\n' '60cd2c24355587aeb1e31cee8fe6683a63b9ab33fc1c67603a76679907c18286  output-j1-matrix-194' '727508d3f7ad9fd4a62e58de9bf1ac59316064397f161c2594c2360a4928eb84  output-j1-matrix-188' | sha256sum --check
for lane in 194 188; do
 mkdir -m 700 "scratch-original-progress-expiry-$lane"
 result=0
 TMPDIR="$root/scratch-original-progress-expiry-$lane" /usr/bin/timeout --signal=TERM --kill-after=5s 60s /usr/bin/nice -n 10 "$root/output-j1-matrix-$lane" diagnostic_kernel_admission::collector_inputs::progress::tests::collector_progress_real_packet_expires_without_receive_time_renewal --exact --nocapture --test-threads=1 >"original-progress-expiry-$lane.log" 2>&1 || result=$?
 cat "original-progress-expiry-$lane.log"
 preflight
 [ "$result" -eq 0 ] || exit "$result"
done
# End of authorized script.
