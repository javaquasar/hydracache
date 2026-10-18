#!/bin/sh
set -eu
umask 077
[ "$(umask)" = 0077 ]
[ "$(id -u)" -eq 1000 ]
[ "$(id -g)" -eq 1000 ]
root=$(pwd -P)
case "$root" in /tmp/hc074-original-preparation-budget-20261018.??????) ;; *) exit 80 ;; esac
[ "$(stat -c '%a:%u:%g' "$root")" = '700:1000:1000' ]
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
    printf 'HC074-ORIGINAL-MANAGER-HOST preflight=ok supervisor_pid=8839 restarts=0\n'
}
no_fixture() {
    for item in /proc/[0-9]*/exe; do
        executable=$(readlink "$item" 2>/dev/null || true)
        case "$executable" in /tmp/hc074-original-manager-join-20261018.*/tests-*|/tmp/hc074-original-preparation-budget-20261018.*/tests-*|/tmp/hc074-original-manager-message-20261018.*/tests-*|/tmp/hc074-original-manager-grouped-message-20261018.*/tests-*) printf 'unexpected_fixture=%s\n' "$executable"; return 91 ;; esac
    done
}
preflight
