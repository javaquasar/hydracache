#!/bin/sh
# Read only, never stop, repair or install anything.
set -eu
[ "$(id -u):$(id -g)" = 0:0 ]
date -u
id
uname -r
[ "$(cat /proc/sys/kernel/random/boot_id)" = e81671d6-bf23-4ebf-b8fe-5dad1f45866f ]
for item in ActiveState:active SubState:running MainPID:8839 NRestarts:0; do
    key=${item%%:*}; value=${item#*:}
    [ "$(systemctl show hydracache-performance-supervisor-074.service -p "$key" --value)" = "$value" ]
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
    [ ! -e "$path" ] && [ ! -L "$path" ]
    printf 'ABSENT %s\n' "$path"
done
printf '%s\n' \
 'f797629f65385ebfd4d94577a43a2d1d020f71c1e841028dad7d02a51b47ad62  /opt/hydracache-perf/bin/hydracache-long-run-supervisor-074' \
 'b314953d4deedaeaade3e24468da4844b7f13d348a5d77b8e8f41c672ff44525  /var/lib/hydracache-performance/provisioning-receipt-074.json' | sha256sum --check
[ -z "$(find /tmp /sys/fs/cgroup -maxdepth 1 -name 'hc074-owned-root-*' -print)" ]
cat /proc/sys/kernel/random/boot_id
cat /sys/devices/system/clocksource/clocksource0/current_clocksource
systemctl show hydracache-performance-supervisor-074.service -p MainPID -p NRestarts -p ExecMainStartTimestamp -p ActiveState -p SubState
printf 'HC074-NATIVE-RECHECK preflight=ok server=true product=false\n'
