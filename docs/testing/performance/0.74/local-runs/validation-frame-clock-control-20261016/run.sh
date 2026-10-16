#!/bin/sh
# Operator-approved local environment control, never a server/product entry.
set -eu
[ "$(id -u)" -eq 0 ]
[ "$(uname -r | tr '[:upper:]' '[:lower:]' | grep -c microsoft)" -eq 1 ]
cd /mnt/c/Workspace/prj/jq/cashe/hydracache-074
unit=systemd-timesyncd.service
systemctl is-active --quiet "$unit"
printf 'HC074-CLOCK-CONTROL service_before='
systemctl is-active "$unit"
restore() {
    result=$?
    trap - 0 HUP INT TERM
    if ! systemctl start "$unit"; then exit 91; fi
    if ! systemctl is-active --quiet "$unit"; then exit 92; fi
    printf 'HC074-CLOCK-CONTROL service_restored=active result=%s\n' "$result"
    exit "$result"
}
trap restore 0
trap 'exit 130' INT
trap 'exit 143' HUP TERM
systemctl stop "$unit"
if systemctl is-active --quiet "$unit"; then exit 90; fi
printf 'HC074-CLOCK-CONTROL service_during=inactive\n'
id artur
cat /proc/sys/kernel/random/boot_id
printf '%s\n' \
  '65f901ca510c240e4beff482c19d58ac6279ab685ee592c072142f98e4a91590  /home/artur/.cache/hydracache-074-194-check/release/deps/hydracache_long_run_supervisor_074-171a8ee7d3705ebd' \
  'c2e6ff7a751a7b6c0278f5d37910dba2c8128ea7356793a9af63373a7f013187  /home/artur/.cache/hydracache-074-188-check/release/deps/hydracache_long_run_supervisor_074-81989065b672da94' | sha256sum --check
/usr/bin/python3 -u - <<'PY'
import time

previous = None
for index in range(91):
    realtime = time.clock_gettime_ns(time.CLOCK_REALTIME)
    boottime = time.clock_gettime_ns(time.CLOCK_BOOTTIME)
    if previous is not None:
        real_delta = realtime - previous[0]
        boot_delta = boottime - previous[1]
        if real_delta < 0 or boot_delta < 0 or abs(real_delta - boot_delta) > 100_000_000:
            print(f"HC074-CLOCK-PREFLIGHT refused index={index} realtime_delta={real_delta} boottime_delta={boot_delta}", flush=True)
            raise SystemExit(93)
    print(f"HC074-CLOCK-PREFLIGHT index={index} realtime_ns={realtime} boottime_ns={boottime}", flush=True)
    previous = (realtime, boottime)
    if index < 90:
        time.sleep(1)
PY
for lane in 194 188; do
    case "$lane" in
      194) binary=/home/artur/.cache/hydracache-074-194-check/release/deps/hydracache_long_run_supervisor_074-171a8ee7d3705ebd ;;
      188) binary=/home/artur/.cache/hydracache-074-188-check/release/deps/hydracache_long_run_supervisor_074-81989065b672da94 ;;
      *) exit 94 ;;
    esac
    printf 'HC074-CLOCK-CONTROL matrix_begin=%s\n' "$lane"
    /usr/bin/timeout --signal=TERM --kill-after=5s 900s /usr/sbin/runuser -u artur -- "$binary" validation_frame_live_drift_at_every_cut_and_reference_refusal --ignored --nocapture
    printf 'HC074-CLOCK-CONTROL matrix_complete=%s\n' "$lane"
done
for lane in 194 188; do
    case "$lane" in
      194) binary=/home/artur/.cache/hydracache-074-194-check/release/deps/hydracache_long_run_supervisor_074-171a8ee7d3705ebd ;;
      188) binary=/home/artur/.cache/hydracache-074-188-check/release/deps/hydracache_long_run_supervisor_074-81989065b672da94 ;;
      *) exit 94 ;;
    esac
    printf 'HC074-CLOCK-CONTROL budget_begin=%s\n' "$lane"
    /usr/bin/timeout --signal=TERM --kill-after=5s 900s /usr/sbin/runuser -u artur -- "$binary" validation_frame_publication_budget --ignored --nocapture
    printf 'HC074-CLOCK-CONTROL budget_complete=%s\n' "$lane"
done
