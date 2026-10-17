#!/bin/sh
set -eu
umask 077
cd /tmp/hc074-controller-cost-20261017.uYyrhr
. ./guard.sh
stat -c '%a:%u:%g' .
cat /proc/sys/kernel/random/boot_id
printf 'perf_event_paranoid='
cat /proc/sys/kernel/perf_event_paranoid
perf --version
set +e
perf stat -e task-clock -- true > ./perf-probe.log 2>&1
result=$?
set -e
cat ./perf-probe.log
printf 'HC074-COMPONENT-PERF-PROBE exit_code=%s configuration_changed=false\n' "$result"
preflight
