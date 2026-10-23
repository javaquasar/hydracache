#!/bin/sh
set -eu
[ "$(id -u):$(id -g)" = 1000:1000 ]
[ "$(cat /proc/sys/kernel/random/boot_id)" = e81671d6-bf23-4ebf-b8fe-5dad1f45866f ]
unit=hydracache-performance-supervisor-074.service
systemctl show "$unit" -p ActiveState -p SubState -p MainPID -p NRestarts
for item in ActiveState:active SubState:running MainPID:8839 NRestarts:0; do
 key=${item%%:*}; value=${item#*:}
 [ "$(systemctl show "$unit" -p "$key" --value)" = "$value" ]
done
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
 printf 'ABSENT %s\n' "$path"
done
[ "$(sudo -n sha256sum /opt/hydracache-perf/bin/hydracache-long-run-supervisor-074 | cut -d' ' -f1)" = f797629f65385ebfd4d94577a43a2d1d020f71c1e841028dad7d02a51b47ad62 ]
[ "$(sudo -n sha256sum /var/lib/hydracache-performance/provisioning-receipt-074.json | cut -d' ' -f1)" = b314953d4deedaeaade3e24468da4844b7f13d348a5d77b8e8f41c672ff44525 ]
printf 'HC074_NATIVE_PREFLIGHT_OK\n'
