#!/bin/sh
# Read-only queries in existing Ubuntu; no bus activation and no trace writes.
set -eu
test "$#" -eq 1
test "$1" = /mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/wsl-clock-causality-root-context-20261023
out=$1
identity() {
    cat /proc/sys/kernel/random/boot_id /proc/1/stat
    for who in 1 self; do
        for ns in user mnt pid time; do
            printf '%s/%s=' "$who" "$ns"
            readlink "/proc/$who/ns/$ns"
        done
    done
    cat /proc/self/timens_offsets
}
context() {
    uname -a
    cat /proc/cmdline /sys/module/hv_utils/parameters/timesync_implicit
    cat /sys/devices/system/clocksource/clocksource0/current_clocksource
    systemctl show systemd-timesyncd.service -p MainPID -p ActiveState -p ActiveEnterTimestampMonotonic -p DropInPaths
    busctl --system --auto-start=no status org.freedesktop.timesync1
    for prop in Frequency ServerName ServerAddress PollIntervalUSec; do
        busctl --system --auto-start=no get-property org.freedesktop.timesync1 /org/freedesktop/timesync1 org.freedesktop.timesync1.Manager "$prop"
    done
    sha256sum /usr/lib/systemd/systemd-timesyncd
    cat /usr/lib/systemd/system/systemd-timesyncd.service.d/wsl.conf
    dpkg-query -W systemd-timesyncd
    journalctl -b --no-pager -n 30 -u systemd-timesyncd
}
identity > "$out/identity-before.txt"
context > "$out/context-before.txt" 2>&1
i=0
while test "$i" -lt 81; do
    runuser -u artur -- "$out/discipline-correlation" >> "$out/kernel.jsonl"
    printf '%s ' "$i" >> "$out/ntp.txt"
    busctl --system --auto-start=no get-property org.freedesktop.timesync1 /org/freedesktop/timesync1 org.freedesktop.timesync1.Manager NTPMessage >> "$out/ntp.txt"
    i=$((i + 1))
    if test "$i" -lt 81; then sleep 1; fi
done
identity > "$out/identity-after.txt"
context > "$out/context-after.txt" 2>&1
