#!/bin/sh
# One metadata -> fixed observer -> postflight session. No enrollment or units.
set -eu
test "$#" -eq 2
case "$1" in /mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/wsl-clock-readonly-20261023/sample) ;; *) exit 64;; esac
case "$2" in /mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/wsl-clock-readonly-20261023) ;; *) exit 64;; esac
identity() {
    cat /proc/sys/kernel/random/boot_id
    cat /proc/1/stat
    for who in 1 self; do
        for ns in user mnt pid time; do
            printf '%s/%s=' "$who" "$ns"
            readlink "/proc/$who/ns/$ns"
        done
    done
    cat /proc/self/timens_offsets
}
metadata() {
    uname -a
    systemctl --version
    for file in /sys/devices/system/clocksource/clocksource0/current_clocksource \
        /sys/devices/system/clocksource/clocksource0/available_clocksource \
        /proc/uptime /sys/devices/system/cpu/online /proc/sys/kernel/osrelease; do
        printf 'file=%s\n' "$file"
        cat "$file"
    done
    grep -E '^(Cpus_allowed_list|Mems_allowed_list):' /proc/self/status
    printf 'clock masks\n'
    find /etc/systemd/system -maxdepth 1 -type l -name '*time*' -printf '%f -> %l\n' | sort
    printf 'loaded clock related units\n'
    systemctl list-units --all --no-pager --no-legend | grep -Ei 'time|chron|ntp|supervisor' || test "$?" -eq 1
    printf 'process names related to clocks\n'
    ps -eo pid,comm | grep -Ei 'timesync|chron|ntpd|timedated|hv_' || test "$?" -eq 1
    printf 'bounded kernel clock messages\n'
    dmesg --color=never | grep -Ei 'clocksource|timekeeping|tsc|time.*(jump|sync|change)' | tail -n 50
    printf 'bounded current boot synchronization journal\n'
    journalctl -b --no-pager -n 30 -u systemd-timesyncd -u systemd-timedated -u chrony -u ntp
}
identity > "$2/identity-before.txt"
metadata > "$2/metadata-before.txt" 2>&1
set +e
timeout --signal=TERM --kill-after=2s 80s "$1" > "$2/samples.jsonl" 2> "$2/sample.stderr.log"
rc=$?
set -e
identity > "$2/identity-after.txt"
metadata > "$2/metadata-after.txt" 2>&1
exit "$rc"
