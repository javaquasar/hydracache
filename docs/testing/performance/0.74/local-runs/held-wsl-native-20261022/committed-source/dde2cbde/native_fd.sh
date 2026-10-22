#!/bin/bash
# Synthetic transport mechanics only, not a full original-owner frame.
set -euo pipefail
test "$(id -u)" = 0
if test "${1:-}" = --helper; then
    test "$(hostname)" = hc074-native
    test "${LISTEN_PID:-}" = "$$"
    test "${LISTEN_FDS:-}" = 2
    test "${LISTEN_FDNAMES:-}" = 'role:empty'
    test "$(stat -Lc '%d:%i:%f' /proc/self/fd/3)" = "$(stat -Lc '%d:%i:%f' /usr/bin/true)"
    test "$(stat -Lc '%d:%i:%f' /proc/self/fd/4)" = "$(stat -Lc '%d:%i:%f' /dev/null)"
    test "$(sha256sum /proc/self/fd/3 | cut -d' ' -f1)" = "$(sha256sum /usr/bin/true | cut -d' ' -f1)"
    printf 'native-fd-mapping-ok\n'
    printf 'listen=%s:%s:%s\n' "$LISTEN_PID" "$LISTEN_FDS" "$LISTEN_FDNAMES"
    printf 'role='; stat -Lc '%d:%i:%f' /proc/self/fd/3
    printf 'empty='; stat -Lc '%d:%i:%f' /proc/self/fd/4
    printf 'stdout='; stat -Lc '%d:%i:%f' "/proc/$$/fd/1"
    printf 'stderr='; stat -Lc '%d:%i:%f' "/proc/$$/fd/2"
    printf 'executable='; readlink "/proc/$$/exe"
    cat /proc/self/cgroup
    exit 0
fi
test "${WSL_DISTRO_NAME:-}" = HydraCache074Native
nonce=${1:-}
[[ "$nonce" =~ ^[0-9a-f]{32}$ ]]
unit="hc074-nativefd-$nonce.service"
slice="system-hc074nativefd$nonce.slice"
description="HydraCache074 synthetic FD control $nonce"
output="/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/held-native-fd-v2-20261022"
test -f "$output/intent.json"
mkdir "$output/calls"
bus() {
    local prefix rc index
    index=$(find "$output/calls" -maxdepth 1 -name '*.argv' -printf x | wc -c)
    printf -v prefix '%s/calls/%03d' "$output" "$index"
    test ! -e "$prefix.argv"
    printf '%q ' busctl --system --auto-start=no --allow-interactive-authorization=no --timeout=250ms -- "$@" > "$prefix.argv"
    printf '\n' >> "$prefix.argv"
    if busctl --system --auto-start=no --allow-interactive-authorization=no --timeout=250ms -- "$@" > "$prefix.stdout" 2> "$prefix.stderr"; then rc=0; else rc=$?; fi
    printf '%s\n' "$rc" > "$prefix.exit"
    cat "$prefix.stdout"
    return "$rc"
}
owner_reply=$(bus call org.freedesktop.DBus /org/freedesktop/DBus org.freedesktop.DBus GetNameOwner s org.freedesktop.systemd1)
[[ "$owner_reply" =~ ^s\ \":[0-9]+\.[0-9]+\"$ ]]
owner=${owner_reply#s \"}; owner=${owner%\"}
test "$(bus call org.freedesktop.DBus /org/freedesktop/DBus org.freedesktop.DBus GetConnectionUnixProcessID s "$owner")" = 'u 1'
test "$(bus call org.freedesktop.DBus /org/freedesktop/DBus org.freedesktop.DBus GetConnectionUnixUser s "$owner")" = 'u 0'
printf 'boot='; cat /proc/sys/kernel/random/boot_id
bus get-property "$owner" /org/freedesktop/systemd1 org.freedesktop.systemd1.Manager Version
path=/org/freedesktop/systemd1
interface=org.freedesktop.systemd1.Manager
if bus call "$owner" "$path" "$interface" GetUnit s "$unit"; then exit 1; fi
grep -Fx "Call failed: Unit $unit not loaded." "$output/calls/004.stderr"
attempted=0
cleanup() {
    local status=$? current
    trap - EXIT TERM INT
    set +e
    if test "$attempted" = 1; then
        current=$(bus call org.freedesktop.DBus /org/freedesktop/DBus org.freedesktop.DBus GetNameOwner s org.freedesktop.systemd1)
        if test "$current" = "$owner_reply" && test "$(bus get-property "$owner" "$unit_path" org.freedesktop.systemd1.Unit Description)" = "s \"$description\""; then
            bus call "$owner" "$path" "$interface" StopUnit ss "$unit" fail || status=1
            for step in {1..40}; do
                state=$(bus get-property "$owner" "$unit_path" org.freedesktop.systemd1.Unit ActiveState)
                test "$state" != 's "active"' || { sleep 0.05; continue; }
                break
            done
            if test "$state" != 's "inactive"' && test -n "$state"; then status=1; fi
            if test -d "/sys/fs/cgroup/system.slice/$slice/$unit"; then
                grep -Fx 'populated 0' "/sys/fs/cgroup/system.slice/$slice/$unit/cgroup.events" || status=1
            fi
            printf 'cleanup-exact-unit-stopped\n'
        else
            printf 'cleanup-uncertain-do-not-resend\n'; status=1
        fi
    fi
    printf 'control-exit=%s\n' "$status"
    exit "$status"
}
unit_path="/org/freedesktop/systemd1/unit/${unit//-/_2d}"
unit_path=${unit_path//./_2e}
trap cleanup EXIT
trap 'exit 1' TERM INT
exec 3</usr/bin/true 4</dev/null
set -o noclobber
exec 5>"$output/helper.stdout" 6>"$output/helper.stderr"
set +o noclobber
printf 'stdout=' > "$output/stream-identities.txt"
stat -Lc '%d:%i:%f' "/proc/$$/fd/5" >> "$output/stream-identities.txt"
printf 'stderr=' >> "$output/stream-identities.txt"
stat -Lc '%d:%i:%f' "/proc/$$/fd/6" >> "$output/stream-identities.txt"
attempted=1
bus call "$owner" "$path" "$interface" StartTransientUnit 'ssa(sv)a(sa(sv))' "$unit" fail 13 \
    Description s "$description" Type s oneshot RemainAfterExit b true Restart s no NoNewPrivileges b true \
    Slice s "$slice" TimeoutStartUSec t 5000000 TimeoutStopUSec t 1000000 KillMode s control-group \
    ExtraFileDescriptors 'a(hs)' 2 3 role 4 empty StandardOutputFileDescriptor h 5 StandardErrorFileDescriptor h 6 \
    ExecStart 'a(sasb)' 1 /bin/bash 3 /bin/bash /mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/held-wsl-native-074/native_fd.sh --helper false 0
exec 3<&- 4<&- 5>&- 6>&-
for step in {1..40}; do
    sub=$(bus get-property "$owner" "$unit_path" org.freedesktop.systemd1.Unit SubState)
    test "$sub" != 's "exited"' || break
    test "$sub" != 's "failed"'
    sleep 0.05
done
test "$sub" = 's "exited"'
test "$(bus get-property "$owner" "$unit_path" org.freedesktop.systemd1.Service Result)" = 's "success"'
test "$(bus get-property "$owner" "$unit_path" org.freedesktop.systemd1.Service ExecMainStatus)" = 'i 0'
test "$(bus get-property "$owner" "$unit_path" org.freedesktop.systemd1.Service MainPID)" = 'u 0'
bus get-property "$owner" "$unit_path" org.freedesktop.systemd1.Unit ControlGroup
grep -Fx native-fd-mapping-ok "$output/helper.stdout"
while IFS= read -r identity; do grep -Fx "$identity" "$output/helper.stdout"; done < "$output/stream-identities.txt"
grep -Fx 'executable=/usr/bin/bash' "$output/helper.stdout"
test ! -s "$output/helper.stderr"
grep -Fx "0::/system.slice/$slice/$unit" "$output/helper.stdout"
test "$(bus call org.freedesktop.DBus /org/freedesktop/DBus org.freedesktop.DBus GetNameOwner s org.freedesktop.systemd1)" = "$owner_reply"
printf 'native-fd-mechanics-passed-not-original-delivery\n'
