#!/bin/sh
# Single authorized isolated root fixture control; no original send/product.
set -eu
[ "$#" -eq 0 ]
[ "$(id -u):$(id -g)" = 0:0 ]
root=$(pwd -P)
case "$root" in /tmp/hc074-held-root-20261020.??????) ;; *) exit 80 ;; esac
[ "$(stat -c '%a:%u:%g' "$root")" = 700:0:0 ]
umask 077
mkdir matrix-started
preflight() {
    date -u
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
        if [ -e "$path" ] || [ -L "$path" ]; then return 91; fi
        printf 'ABSENT %s\n' "$path"
    done
    [ "$(sha256sum /opt/hydracache-perf/bin/hydracache-long-run-supervisor-074 | cut -d' ' -f1)" = f797629f65385ebfd4d94577a43a2d1d020f71c1e841028dad7d02a51b47ad62 ] || return 91
    [ "$(sha256sum /var/lib/hydracache-performance/provisioning-receipt-074.json | cut -d' ' -f1)" = b314953d4deedaeaade3e24468da4844b7f13d348a5d77b8e8f41c672ff44525 ] || return 91
    task_dirs=$(find /tmp -maxdepth 1 -name 'hc074-owned-root-*' -print) || return 91
    [ -z "$task_dirs" ] || return 91
    task_groups=$(find /sys/fs/cgroup -maxdepth 1 -name 'hc074-owned-root-*' -print) || return 91
    [ -z "$task_groups" ] || return 91
    systemctl show hydracache-performance-supervisor-074.service -p MainPID -p NRestarts -p ExecMainStartTimestamp -p ActiveState -p SubState || return 91
    printf 'HC074-HELD-ROOT preflight=ok installed_unchanged=true\n'
}
inputs() {
    sha256sum --check inputs.sha256 || return 92
    [ "$(stat -c '%a:%u:%g' run.sh)" = 500:0:0 ] || return 92
    for image in join-194 join-188; do
        [ -f "$image" ] && [ ! -L "$image" ] || return 92
        [ "$(stat -c '%a:%u:%g' "$image")" = 500:0:0 ] || return 92
    done
    [ "$(stat -c '%a:%u:%g' bundle)" = 500:0:0 ] || return 92
    for path in bundle/*; do
        [ -f "$path" ] && [ ! -L "$path" ] || return 92
        [ "$(stat -c '%a:%u:%g' "$path")" = 400:0:0 ] || return 92
    done
}
finish() {
    result=$?
    trap - EXIT
    post=0
    preflight >postflight.log 2>&1 || post=$?
    inputs >>postflight.log 2>&1 || post=$?
    printf 'HC074-HELD-ROOT exit=%s postflight_exit=%s server=true product=false\n' "$result" "$post"
    printf '%s\n' "$result" >wrapper.exit
    printf '%s\n' "$post" >postflight.exit
    sync -f "$root"
    if [ "$result" -ne 0 ]; then exit "$result"; fi
    exit "$post"
}
trap finish EXIT
preflight >preflight.log 2>&1
inputs >>preflight.log 2>&1
id >>preflight.log
uname -a >>preflight.log
getconf GNU_LIBC_VERSION >>preflight.log
cat /proc/self/mountinfo >mountinfo-before.log
test=diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::privileged_root::cgroup::held_live::privileged_consuming_held_live_matrix
failed=0
for lane in 194 188; do
    preflight >"$lane-preflight.log" 2>&1
    result=0
    /usr/bin/env -i PATH=/usr/sbin:/usr/bin:/sbin:/bin \
      HC074_OWNED_ROOT_FIXTURE=local-only-v1 \
      HC074_START_MATERIAL_BUNDLE="$root/bundle" \
      /usr/bin/timeout --signal=TERM --kill-after=5s 1200s \
      "$root/join-$lane" "$test" --ignored --exact --nocapture --test-threads=1 \
      >"$lane-matrix.log" 2>&1 || result=$?
    printf '%s\n' "$result" >"$lane-matrix.exit"
    sync -f "$root/$lane-matrix.log"
    sync -f "$root/$lane-matrix.exit"
    printf 'HC074-HELD-ROOT lane=%s exit=%s\n' "$lane" "$result"
    if [ "$result" -ne 0 ]; then failed=1; fi
    shape=0
    [ "$(grep -Fc 'test result: ok. 1 passed; 0 failed; 0 ignored;' "$lane-matrix.log")" -eq 6 ] || shape=1
    [ "$(grep -Fc 'HC074-HELD-LIVE-ORIGINAL ' "$lane-matrix.log")" -eq 5 ] || shape=1
    for case in healthy revalidate role-before role-after trust-after; do
        [ "$(grep -Fc "HC074-HELD-LIVE case=$case seed=0x074c051e actual_fixed=true actual_consuming=true" "$lane-matrix.log")" -eq 1 ] || shape=1
    done
    if grep -Fq 'HC074-HELD-LIVE-COST' "$lane-matrix.log"; then shape=1; fi
    printf '%s\n' "$shape" >"$lane-shape.exit"
    if [ "$shape" -ne 0 ]; then failed=1; fi
    preflight >"$lane-postflight.log" 2>&1
done
cat /proc/self/mountinfo >mountinfo-after.log
exit "$failed"
