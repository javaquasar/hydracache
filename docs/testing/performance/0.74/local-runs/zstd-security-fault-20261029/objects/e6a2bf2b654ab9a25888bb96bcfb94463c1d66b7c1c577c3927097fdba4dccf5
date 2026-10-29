#!/bin/sh
# Root only in the previously reviewed private mount/cgroup test composition.
set -eu
[ "$#" = 0 ]
[ "$(id -u):$(id -g)" = 0:0 ]
root=$(pwd -P)
case "$root" in /tmp/hc074-native-held-root-20261023.??????) ;; *) exit 80;; esac
[ "$(stat -c '%a:%u:%g' "$root")" = 700:0:0 ]
umask 077
mkdir holder-once
preflight() {
 [ "$(cat /proc/sys/kernel/random/boot_id)" = e81671d6-bf23-4ebf-b8fe-5dad1f45866f ] || return 91
 for item in ActiveState:active SubState:running MainPID:8839 NRestarts:0; do
  key=${item%%:*}; value=${item#*:}
  [ "$(systemctl show hydracache-performance-supervisor-074.service -p "$key" --value)" = "$value" ] || return 91
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
  [ ! -e "$path" ] && [ ! -L "$path" ] || return 91
  printf 'ABSENT %s\n' "$path"
 done
 [ "$(sha256sum /opt/hydracache-perf/bin/hydracache-long-run-supervisor-074 | cut -d' ' -f1)" = f797629f65385ebfd4d94577a43a2d1d020f71c1e841028dad7d02a51b47ad62 ] || return 91
 [ "$(sha256sum /var/lib/hydracache-performance/provisioning-receipt-074.json | cut -d' ' -f1)" = b314953d4deedaeaade3e24468da4844b7f13d348a5d77b8e8f41c672ff44525 ] || return 91
 for base in /tmp /sys/fs/cgroup; do
  for prefix in 'hc074-owned-root-*' 'hc074-root-fixture-*'; do
   leftovers=$(find "$base" -maxdepth 1 -name "$prefix" -print) || return 91
   [ -z "$leftovers" ] || { printf '%s\n' "$leftovers"; return 91; }
  done
 done
 systemctl show hydracache-performance-supervisor-074.service -p MainPID -p NRestarts -p ActiveState -p SubState
}
inputs() {
 sha256sum --check inputs.sha256 || return 92
 for file in holder-session.sh libtest inputs.sha256; do
  [ -f "$file" ] && [ ! -L "$file" ] || return 92
  [ "$(stat -c '%a:%u:%g' "$file")" = 500:0:0 ] || return 92
 done
 [ "$(stat -c '%a:%u:%g' bundle)" = 500:0:0 ] || return 92
 for file in bundle/*; do
  [ -f "$file" ] && [ ! -L "$file" ] || return 92
  [ "$(stat -c '%a:%u:%g' "$file")" = 400:0:0 ] || return 92
 done
}
finish() {
 result=$?
 trap - EXIT
 post=0
 preflight >postflight.log 2>&1 || post=$?
 inputs >>postflight.log 2>&1 || post=$?
 cat /proc/self/mountinfo >mountinfo-after.log || post=$?
 if [ -f mountinfo-before.log ]; then cmp mountinfo-before.log mountinfo-after.log || post=$?; fi
 printf '%s\n' "$result" >wrapper.exit
 printf '%s\n' "$post" >postflight.exit
 sync -f "$root"
 printf 'HC074_NATIVE_HOLDER exit=%s postflight_exit=%s server=true product=false\n' "$result" "$post"
 if [ "$result" != 0 ]; then exit "$result"; fi
 exit "$post"
}
trap finish EXIT
preflight >preflight.log 2>&1
inputs >>preflight.log 2>&1
cat /proc/self/mountinfo >mountinfo-before.log
selector=diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::privileged_root::cgroup::held_live::privileged_consuming_held_live_matrix
result=0
/usr/bin/env -i PATH=/usr/sbin:/usr/bin:/sbin:/bin \
 HC074_OWNED_ROOT_FIXTURE=local-only-v1 HC074_START_MATERIAL_BUNDLE="$root/bundle" \
 /usr/bin/timeout --signal=TERM --kill-after=5s 1200s \
 nice -n 10 "$root/libtest" "$selector" --ignored --exact --nocapture --test-threads=1 \
 >matrix.log 2>&1 || result=$?
printf '%s\n' "$result" >matrix.exit
sync -f "$root/matrix.log"
if [ "$result" != 0 ]; then exit "$result"; fi
[ "$(grep -Fc 'test result: ok. 1 passed; 0 failed; 0 ignored;' matrix.log)" = 6 ]
[ "$(grep -Fc 'HC074-HELD-LIVE-ORIGINAL ' matrix.log)" = 5 ]
for case in healthy revalidate role-before role-after trust-after; do
 [ "$(grep -Fc "HC074-HELD-LIVE case=$case seed=0x074c051e actual_fixed=true actual_consuming=true" matrix.log)" = 1 ]
done
if grep -Fq 'HC074-HELD-LIVE-COST' matrix.log; then exit 93; fi
printf 'HC074_NATIVE_HOLDER_SHAPE_OK\n'
