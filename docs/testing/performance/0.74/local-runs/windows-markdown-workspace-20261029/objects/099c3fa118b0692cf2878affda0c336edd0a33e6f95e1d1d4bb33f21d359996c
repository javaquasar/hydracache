#!/bin/sh
# Exactly one existing unprivileged read-only sampler, no manager delivery.
set -eu
[ "$#" = 0 ]
[ "$(id -u):$(id -g)" = 1000:1000 ]
root=$(pwd -P)
case "$root" in /tmp/hc074-native-clock-20261023.??????) ;; *) exit 64;; esac
[ "$(stat -c '%a:%u:%g' "$root")" = 700:1000:1000 ]
umask 077
mkdir clock-once
sha256sum --check inputs.sha256 >input-check-before.log
sh ./preflight.sh >preflight-before.log 2>&1
identity() {
 cat /proc/sys/kernel/random/boot_id
 cat /proc/1/stat
 for who in 1 self; do
  for ns in user mnt pid time; do
   printf '%s/%s=' "$who" "$ns"
   sudo -n readlink "/proc/$who/ns/$ns"
  done
 done
 cat /proc/self/timens_offsets
}
metadata() {
 uname -srmo
 systemd --version | head -n 1
 for file in /sys/devices/system/clocksource/clocksource0/current_clocksource \
  /sys/devices/system/clocksource/clocksource0/available_clocksource \
  /sys/devices/system/cpu/online; do
  printf 'FILE %s\n' "$file"; cat "$file"
 done
 grep -E '^(Cpus_allowed_list|Mems_allowed_list):' /proc/self/status
 systemctl show systemd-timesyncd.service chrony.service ntp.service -p Id -p LoadState -p ActiveState -p MainPID
}
identity >identity-before.txt
metadata >metadata-before.txt 2>&1
result=0
/usr/bin/env -i PATH=/usr/bin:/bin /usr/bin/timeout --signal=TERM --kill-after=2s 80s \
 nice -n 10 "$root/sample" >samples.jsonl 2>sample.stderr.log || result=$?
printf '%s\n' "$result" >sample.exit
post=0
identity >identity-after.txt || post=$?
metadata >metadata-after.txt 2>&1 || post=$?
sh ./preflight.sh >preflight-after.log 2>&1 || post=$?
sha256sum --check inputs.sha256 >input-check-after.log 2>&1 || post=$?
printf '%s\n' "$post" >postflight.exit
printf 'HC074_NATIVE_CLOCK sample_exit=%s postflight_exit=%s server=true product=false\n' "$result" "$post"
if [ "$result" != 0 ]; then exit "$result"; fi
exit "$post"
