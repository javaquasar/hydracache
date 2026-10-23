#!/bin/sh
# One local diagnostic fixture. No existing unit, clock or server mutation.
set -eu
lane=$1
bundle=$2
bootstrap_sha=$3
libtest_sha=$4
test "$(id -u)" = 0
test "$(cat /etc/hostname)" = hc074-native
test "$(systemctl --version | head -n 1 | cut -d ' ' -f 2)" = 257
case "$lane" in /mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/bootstrap-original-service-20261023/native-first) ;; *) exit 74;; esac
test "$(sha256sum "$lane/bootstrap" | cut -d ' ' -f 1)" = "$bootstrap_sha"
test "$(sha256sum "$lane/libtest" | cut -d ' ' -f 1)" = "$libtest_sha"
fixed=/opt/hydracache-perf/bin/diagnostic-held-bootstrap-074
for target in "$fixed" /etc/hydracache-perf/held-bootstrap-enrollment-074.json; do
    test ! -e "$target" && test ! -L "$target"
done
for entry in 'passwd hydracache-perf' 'passwd 986' 'group hydracache-perf' 'group 986' 'group 987' 'group hc074-fixture-extra'; do
    if getent $entry >/dev/null; then echo "preexisting identity: $entry" >&2; exit 74; fi
done
test -x /usr/sbin/nologin
test ! -e /opt/hydracache-perf
test ! -L /opt/hydracache-perf
test -d /sys/fs/cgroup/system.slice
# Do not enroll over campaigns or old fixture state. No broad cleanup.
for path in /var/lib/hydracache-perf/campaigns/active /var/lib/hydracache-perf/active-campaign.json /run/hydracache-perf/active-campaign; do
    test ! -e "$path" && test ! -L "$path"
done
temporary=$(mktemp -d /tmp/hc074-original-service-XXXXXX)
chmod 700 "$temporary"
root_group=0
extra_group=0
account=0
image=0
parents=0
main_files='passwd group shadow gshadow'
for name in $main_files; do
    cp -p "/etc/$name" "$temporary/$name.original"
    if test -e "/etc/$name-"; then cp -p "/etc/$name-" "$temporary/$name.backup"; fi
done
snapshot() {
    cat /proc/sys/kernel/random/boot_id
    stat -Lc '%d:%i' /proc/1/ns/mnt /proc/1/ns/user
    systemctl show hydracache-performance-supervisor-074.service --property=LoadState,ActiveState,SubState,MainPID,InvocationID || test "$?" = 4
    for path in /etc/systemd/system/systemd-timesyncd.service /etc/systemd/system/systemd-timedated.service; do
        test "$(readlink "$path")" = /dev/null
        readlink "$path"
    done
}
snapshot > "$lane/preflight.txt"
cleanup() {
    original_rc=$?
    trap - EXIT HUP INT TERM
    set +e
    cleanup_rc=0
    # A journaled intent is emitted before any attempt-unit mutation. It is
    # cleanup-only; never replay or dispatch another service from it.
    intent=$(sed -n 's/^HC074-ORIGINAL-SCOPE-INTENT //p' "$lane/live.stdout.log" 2>/dev/null)
    if test -n "$intent"; then
        slice=$(printf '%s\n' "$intent" | sed -n 's/^slice=\([^ ]*\) service=.*/\1/p')
        service=$(printf '%s\n' "$intent" | sed -n 's/^slice=[^ ]* service=\([^ ]*\) marker=.*/\1/p')
        if printf '%s\n' "$slice" | grep -Eq '^system-hc074[0-9a-f]{64}\.slice$' && printf '%s\n' "$service" | grep -Eq '^hydracache-diagnostic-074-[0-9a-f]{64}-1\.service$'; then
            if test "$(systemctl show --value --property=LoadState "$service")" != not-found; then
                test "$(systemctl show --value --property=Transient "$service")" = yes &&
                test "$(systemctl show --value --property=Description "$service")" = 'HydraCache held diagnostic projection; no transport' &&
                test "$(systemctl show --value --property=Slice "$service")" = "$slice" &&
                systemctl stop "$service" && systemctl reset-failed "$service"
                # reset-failed may report an already unloaded transient unit.
                test "$(systemctl show --value --property=LoadState "$service")" = not-found || cleanup_rc=74
            fi
            if test "$(systemctl show --value --property=LoadState "$slice")" != not-found; then
                digest=${slice#system-hc074}; digest=${digest%.slice}
                test "$(systemctl show --value --property=Transient "$slice")" = yes &&
                test "$(systemctl show --value --property=Description "$slice")" = "HC074 isolated original service $digest" &&
                test "$(cat "/sys/fs/cgroup/system.slice/$slice/cgroup.events")" = "$(printf 'populated 0\nfrozen 0')" &&
                systemctl stop "$slice" || cleanup_rc=74
            fi
        else cleanup_rc=74; fi
    fi
    # Never remove the account while a writer (or descriptor holder) survives.
    if test "$account" = 1; then
        test "$(id -u hydracache-perf)" = 986 && test "$(id -g hydracache-perf)" = 986 || cleanup_rc=74
        if pgrep -u 986 >/dev/null; then cleanup_rc=74; else
            test "$cleanup_rc" = 0 && userdel hydracache-perf || cleanup_rc=74
        fi
    fi
    if test "$cleanup_rc" = 0; then
        if test "$extra_group" = 1 && getent group hc074-fixture-extra >/dev/null; then
            test "$(getent group hc074-fixture-extra | cut -d : -f 3)" = 987 && groupdel hc074-fixture-extra || cleanup_rc=74
        fi
        if test "$root_group" = 1 && getent group hydracache-perf >/dev/null; then
            test "$(getent group hydracache-perf | cut -d : -f 3)" = 986 && groupdel hydracache-perf || cleanup_rc=74
        fi
    fi
    # Account tools must preserve every original entry byte-for-byte. Restore
    # only their backups after proving the current main databases are original.
    for name in $main_files; do
        cmp -s "/etc/$name" "$temporary/$name.original" || cleanup_rc=74
    done
    if test "$cleanup_rc" = 0; then
        for name in $main_files; do
            if test -e "$temporary/$name.backup"; then cp -p "$temporary/$name.backup" "/etc/$name-" || cleanup_rc=74;
            else rm -f -- "/etc/$name-" || cleanup_rc=74; fi
        done
    fi
    if test "$image" = 1; then
        test "$(stat -Lc '%d:%i:%s:%u:%g:%a' "$fixed")" = "$(cat "$temporary/image.pin")" &&
        test "$(sha256sum "$fixed" | cut -d ' ' -f 1)" = "$bootstrap_sha" &&
        rm -- "$fixed" || cleanup_rc=74
    fi
    if test "$parents" = 1; then rmdir /opt/hydracache-perf/bin /opt/hydracache-perf || cleanup_rc=74; fi
    snapshot > "$lane/postflight.txt" || cleanup_rc=74
    cmp -s "$lane/preflight.txt" "$lane/postflight.txt" || cleanup_rc=74
    if test "$cleanup_rc" = 0; then
        for name in $main_files; do rm -f -- "$temporary/$name.original" "$temporary/$name.backup"; done
        rm -f -- "$temporary/image.pin" "$temporary/libtest"
        rmdir -- "$temporary" || cleanup_rc=74
    else printf 'Recovery files retained, root-only: %s\n' "$temporary" >&2; fi
    printf 'test_rc=%s cleanup_rc=%s\n' "$original_rc" "$cleanup_rc" > "$lane/postflight-result.txt"
    if test "$cleanup_rc" != 0; then exit "$cleanup_rc"; fi
    exit "$original_rc"
}
trap cleanup EXIT HUP INT TERM
root_group=1; groupadd --gid 986 hydracache-perf
extra_group=1; groupadd --gid 987 hc074-fixture-extra
account=1; useradd --no-create-home --home-dir /nonexistent --shell /usr/sbin/nologin --uid 986 --gid 986 --groups 987 --no-user-group --no-log-init -K CREATE_MAIL_SPOOL=no --password '!' hydracache-perf
test "$(id -G hydracache-perf)" = '986 987'
parents=1; mkdir /opt/hydracache-perf /opt/hydracache-perf/bin
chmod 755 /opt/hydracache-perf /opt/hydracache-perf/bin
install --mode=555 "$lane/bootstrap" "$fixed"
image=1
stat -Lc '%d:%i:%s:%u:%g:%a' "$fixed" > "$temporary/image.pin"
test "$(sha256sum "$fixed" | cut -d ' ' -f 1)" = "$bootstrap_sha"
install --mode=555 "$lane/libtest" "$temporary/libtest"
env -i PATH=/usr/bin:/bin HOME=/root HC074_ORIGINAL_SERVICE=native-once-v1 HC074_OWNED_ROOT_FIXTURE=local-only-v1 HC074_START_MATERIAL_BUNDLE="$bundle" \
    timeout --signal=TERM --kill-after=5s 120s "$temporary/libtest" privileged_original_manager_service_once --ignored --nocapture --test-threads=1 > "$lane/live.stdout.log" 2> "$lane/live.stderr.log"
