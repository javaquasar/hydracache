#!/bin/bash
# Read-only first-boot observation of this exact separate environment.
set -euo pipefail
test "${WSL_DISTRO_NAME:-}" = HydraCache074Native
test "$(id -u)" = 0
test "$(cat /proc/1/comm)" = systemd
printf 'distribution\n'; cat /etc/os-release
printf 'systemd\n'; systemctl --version
printf 'packages\n'; dpkg-query -W systemd systemd-sysv dbus
printf 'private-config\n'; cat /etc/wsl.conf
printf 'clock-masks\n'
for unit in systemd-timesyncd.service systemd-timedated.service systemd-time-wait-sync.service \
    chrony.service chronyd.service ntp.service ntpd.service ntpsec.service openntpd.service \
    apt-daily.timer apt-daily-upgrade.timer fstrim.timer man-db.timer e2scrub_all.timer logrotate.timer; do
    target=$(readlink "/etc/systemd/system/$unit")
    test "$target" = /dev/null
    printf '%s=%s\n' "$unit" "$target"
done
printf 'active-services\n'; systemctl list-units --type=service --state=running --no-pager --no-legend
printf 'manager-identity\n'
owner=$(busctl --system --auto-start=no --timeout=250ms call org.freedesktop.DBus /org/freedesktop/DBus org.freedesktop.DBus GetNameOwner s org.freedesktop.systemd1)
printf '%s\n' "$owner"
[[ "$owner" =~ ^s\ \":[0-9]+\.[0-9]+\"$ ]]
owner=${owner#s \"}; owner=${owner%\"}
test "$(busctl --system --auto-start=no --timeout=250ms call org.freedesktop.DBus /org/freedesktop/DBus org.freedesktop.DBus GetConnectionUnixProcessID s "$owner")" = 'u 1'
test "$(busctl --system --auto-start=no --timeout=250ms call org.freedesktop.DBus /org/freedesktop/DBus org.freedesktop.DBus GetConnectionUnixUser s "$owner")" = 'u 0'
busctl --system --auto-start=no --timeout=250ms get-property "$owner" /org/freedesktop/systemd1 org.freedesktop.systemd1.Manager Version
printf 'kernel-context\n'; uname -a
readlink /proc/1/exe /proc/self/ns/mnt /proc/self/ns/pid /proc/self/ns/user
cat /proc/1/cgroup
printf 'read-only-clock-samples\n'
for sample in 1 2 3; do
    printf 'sample=%s realtime=' "$sample"; date +%s%N
    printf 'uptime='; cat /proc/uptime
    printf 'boot='; cat /proc/sys/kernel/random/boot_id
    test "$sample" = 3 || sleep 1
done
printf 'finished-read-only-no-units-submitted\n'
