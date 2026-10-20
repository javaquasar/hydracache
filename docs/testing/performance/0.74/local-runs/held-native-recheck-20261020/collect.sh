#!/bin/sh
# Export this completed attempt only, never restart it or copy installed secrets.
set -eu
[ "$(id -u):$(id -g)" = 0:0 ]
umask 077
root=/tmp/hc074-native-recheck-root-20261020.hBFgSO
[ "$(stat -c '%a:%u:%g' "$root")" = 700:0:0 ]
[ ! -L "$root" ]
cd "$root"
[ -f wrapper.exit ] && [ -f postflight.exit ]
case "$(cat wrapper.exit)" in 0|1) ;; *) exit 94 ;; esac
[ "$(cat postflight.exit)" = 0 ]
task_export=$(mktemp -d /tmp/hc074-native-recheck-export-20261020.XXXXXX)
for name in wrapper.log wrapper.exit postflight.exit preflight.log postflight.log mountinfo-before.log mountinfo-after.log 194-preflight.log 194-postflight.log 194-matrix.log 194-matrix.exit 194-shape.exit 188-preflight.log 188-postflight.log 188-matrix.log 188-matrix.exit 188-shape.exit; do
    [ -f "$name" ] && [ ! -L "$name" ]
    install -m 400 "$name" "$task_export/$name"
done
(cd "$task_export"; sha256sum *.log *.exit) >"$task_export/remote-files.sha256"
chmod 400 "$task_export/remote-files.sha256"
cat "$task_export/remote-files.sha256"
printf 'EXPORT=%s\n' "$task_export"
chown 1000:1000 "$task_export" "$task_export"/*
chmod 700 "$task_export"
