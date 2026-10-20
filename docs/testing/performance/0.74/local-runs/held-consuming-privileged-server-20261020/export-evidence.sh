#!/bin/sh
# Export only completed control logs, never private keys or installed files.
set -eu
[ "$(id -u):$(id -g)" = 0:0 ]
umask 077
root=/tmp/hc074-held-root-20261020.Scmc35
[ "$(stat -c '%a:%u:%g' "$root")" = 700:0:0 ]
[ ! -L "$root" ]
cd "$root"
[ -f wrapper.exit ] && [ -f postflight.exit ]
[ "$(cat wrapper.exit)" = 1 ] && [ "$(cat postflight.exit)" = 0 ]
export=$(mktemp -d /tmp/hc074-held-export-20261020.XXXXXX)
for name in wrapper.log wrapper.exit postflight.exit preflight.log postflight.log mountinfo-before.log mountinfo-after.log 194-preflight.log 194-postflight.log 194-matrix.log 194-matrix.exit 194-shape.exit 188-preflight.log 188-postflight.log 188-matrix.log 188-matrix.exit 188-shape.exit; do
    [ -f "$name" ] && [ ! -L "$name" ]
    install -m 400 "$name" "$export/$name"
done
(cd "$export"; sha256sum *.log *.exit) >"$export/remote-files.sha256"
cat "$export/remote-files.sha256"
printf 'EXPORT=%s\n' "$export"
chown -R 1000:1000 "$export"
chmod 700 "$export"
exit 0
