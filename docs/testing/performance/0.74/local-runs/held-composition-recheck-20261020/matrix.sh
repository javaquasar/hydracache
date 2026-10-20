#!/bin/bash
# Local fixture only. All process/cgroup mutation belongs to the existing fixture.
set -eu -o pipefail
task_dir=$(cd -- "$(dirname -- "$0")" && pwd)
test "$(id -u)" = 0
test "$#" = 4
for task_file in preflight.log postflight.log matrix-194.log matrix-188.log exits.txt; do
    test ! -e "$task_dir/$task_file"
done
snapshot() {
    id
    uname -r
    printf 'boot='; cat /proc/sys/kernel/random/boot_id
    printf 'clocksource='; cat /sys/devices/system/clocksource/clocksource0/current_clocksource
    printf 'uptime='; cat /proc/uptime
    printf 'timesyncd='; systemctl is-active systemd-timesyncd || true
    stat -c '%a %u %g %n' /sys/fs/cgroup /run/systemd/private
    find /tmp /sys/fs/cgroup -maxdepth 1 -name 'hc074-owned-root-*' -print
}
snapshot > "$task_dir/preflight.log"
test -z "$(find /tmp /sys/fs/cgroup -maxdepth 1 -name 'hc074-owned-root-*' -print -quit)"
task_boot=$(cat /proc/sys/kernel/random/boot_id)
task_test='diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::privileged_root::cgroup::held_live::privileged_consuming_held_live_matrix'
for task_lane in 194 188; do
    task_image=$1
    task_sha=$2
    shift 2
    test "$(sha256sum "$task_image" | cut -c1-64)" = "$task_sha"
    set +e
    env -u HC074_CONSUMING_HELD_LIVE_CASE -u HC074_CONSUMING_HELD_COST \
        HC074_OWNED_ROOT_FIXTURE=local-only-v1 \
        HC074_START_MATERIAL_BUNDLE=/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/diagnostic-start-material-bundle-20261017 \
        /usr/bin/timeout --signal=TERM --kill-after=5s 1200s \
        "$task_image" "$task_test" --exact --ignored --nocapture --test-threads=1 \
        > "$task_dir/matrix-$task_lane.log" 2>&1
    task_exit=$?
    set -e
    printf '%s %s\n' "$task_lane" "$task_exit" >> "$task_dir/exits.txt"
    printf 'lane=%s matrix_exit=%s\n' "$task_lane" "$task_exit"
    test "$(sha256sum "$task_image" | cut -c1-64)" = "$task_sha"
done
snapshot > "$task_dir/postflight.log"
test "$task_boot" = "$(cat /proc/sys/kernel/random/boot_id)"
test -z "$(find /tmp /sys/fs/cgroup -maxdepth 1 -name 'hc074-owned-root-*' -print -quit)"
