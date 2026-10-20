#!/bin/sh
# Nonroot isolated test comparison only; no release/service/clock operation.
set -eu
[ "$(id -u)" -eq 1000 ]
[ "$(id -g)" -eq 1000 ]
root=$(pwd -P)
case "$root" in /tmp/hc074-consuming-server-20261020.??????) ;; *) exit 80 ;; esac
[ "$(stat -c '%a:%u:%g' "$root")" = '700:1000:1000' ]
umask 077
[ ! -e matrix-started ]
mkdir matrix-started
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
    printf 'HC074-CONSUMING-SERVER preflight=ok supervisor_pid=8839 restarts=0\n'
}
images() {
    printf '%s\n' \
      'ff41048700a4577dcd1efdaf1a2332a9431523086fd84855cb194b3dd98648fd  join-194' \
      '13d4a4b274a172d6e3cc68dbaa352c6a6649ebcea65551cd1925725e4867c135  join-188' \
      'fb9ae321690d0fb4f7eb29aace444e4f4cf06729b5dbb3a77a134ea1cf2bca14  old-194' \
      '64254b3936c76be2dc1fe208ae5c0020d5db713568afd19881c84c129de8d441  old-188' | sha256sum --check
}
preflight
images
id
uname -a
cat /proc/sys/kernel/random/boot_id
getconf GNU_LIBC_VERSION
systemctl show hydracache-performance-supervisor-074.service -p MainPID -p NRestarts -p ExecMainStartTimestamp
for image in join-194 join-188 old-194 old-188; do ldd "$root/$image"; done
failed=0
for lane in 194 188; do
    for round in 1 2; do
        for selection in new trust fixed old; do
            preflight
            name="$lane-$selection-$round"
            mkdir -m 700 "scratch-$name"
            case "$selection" in
              new) set -- "$root/join-$lane" consuming_held; count=17 ;;
              trust) set -- "$root/join-$lane" held_runtime::tests:: --skip consuming_held_bridge; count=26 ;;
              fixed) set -- "$root/join-$lane" fixed_slice::tests::fixed_placement_; count=5 ;;
              old) set -- "$root/old-$lane"; count=38 ;;
            esac
            printf 'HC074-CONSUMING-SERVER selection=%s expected=%s begin=true\n' "$name" "$count"
            result=0
            TMPDIR="$root/scratch-$name" /usr/bin/timeout --signal=TERM --kill-after=5s 900s /usr/bin/nice -n 10 "$@" --nocapture --test-threads=1 >"$name.log" 2>&1 || result=$?
            cat "$name.log"
            printf 'HC074-CONSUMING-SERVER selection=%s expected=%s exit=%s\n' "$name" "$count" "$result"
            printf '%s\n' "$result" >"$name.exit"
            if [ "$result" -ne 0 ]; then failed=1; fi
            if ! grep -Fq "test result: ok. $count passed; 0 failed; 0 ignored;" "$name.log"; then failed=1; fi
            preflight
        done
    done
done
images
preflight
printf 'HC074-CONSUMING-SERVER complete=true failed=%s supervisor_unchanged=true\n' "$failed"
exit "$failed"
