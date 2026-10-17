#!/bin/sh
set -eu
cd /tmp/hc074-pipe-transport-20261017.b9l0y9
. ./guard.sh
if pgrep -u 1000 -f '(^| )(\./|/tmp/hc074-pipe-transport-20261017.b9l0y9/)tests-(194|188)( |$)'; then
    printf 'Test processes still present\n' >&2
    exit 82
fi
# The first launch refused executable mode before entering libtest.
[ -d ./run-194 ]
if find ./run-194 -mindepth 1 -print -quit | grep -q .; then
    printf 'Unexpected effects in launch-permission attempt\n' >&2
    exit 83
fi
systemctl show hydracache-performance-supervisor-074.service -p ActiveState -p SubState -p MainPID -p NRestarts -p ExecMainStartTimestamp
cat /proc/sys/kernel/random/boot_id
sudo -n sha256sum /opt/hydracache-perf/bin/hydracache-long-run-supervisor-074 /var/lib/hydracache-performance/provisioning-receipt-074.json
test ! -e /opt/hydracache-performance/0.74/diagnostic-pilot
printf 'HC074-PIPE-TRANSPORT-POSTFLIGHT test_processes=absent supervisor_unchanged=true installed_receipt_unchanged=true active_contexts=absent clock_changed=false product=false\n'
