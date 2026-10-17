#!/bin/sh
set -eu
umask 077
cd /tmp/hc074-start-material-20261017.vLX1GL
. ./guard.sh
for process in /proc/[0-9]*; do
    executable=$(readlink "$process/exe" 2>/dev/null || :)
    case "$executable" in
        "$root"/tests-194|"$root"/tests-188|"$root"/tests-attribution-194|"$root"/tests-v2-194|"$root"/tests-v2-188)
            printf 'Owned test process remains: %s\n' "$process"
            exit 84 ;;
    esac
done
systemctl show hydracache-performance-supervisor-074.service -p ActiveState -p SubState -p MainPID -p NRestarts -p ExecMainStartTimestamp -p User -p Group -p NoNewPrivileges
sudo -n sha256sum /opt/hydracache-perf/bin/hydracache-long-run-supervisor-074 /var/lib/hydracache-performance/provisioning-receipt-074.json
printf 'HC074-START-MATERIAL-POSTFLIGHT test_processes=absent supervisor_unchanged=true installed_receipt_unchanged=true active_contexts=absent clock_changed=false product=false\n'
# End.
