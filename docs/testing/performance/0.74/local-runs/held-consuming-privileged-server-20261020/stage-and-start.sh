#!/bin/sh
# Reviewed stdin recipe: never execute code from the admin upload directory.
set -eu
[ "$(id -u):$(id -g)" = 0:0 ]
umask 077
upload=/tmp/hc074-held-upload-20261020.PShZT2
[ "$(stat -c '%a:%u:%g' "$upload")" = 700:1000:1000 ]
[ ! -L "$upload" ]
root=$(mktemp -d /tmp/hc074-held-root-20261020.XXXXXX)
[ "$(stat -c '%a:%u:%g' "$root")" = 700:0:0 ]
mkdir -m 700 "$root/bundle"
for file in run.sh inputs.sha256 join-194 join-188; do
    [ -f "$upload/$file" ] && [ ! -L "$upload/$file" ]
    install -m 500 "$upload/$file" "$root/$file"
done
for file in timing-controls-074 build-receipt-v1.json build-log.jsonl Cargo.lock.root Cargo.lock.observer embedded.json direct.json resp2.json resp3.json; do
    [ -f "$upload/bundle/$file" ] && [ ! -L "$upload/bundle/$file" ]
    install -m 400 "$upload/bundle/$file" "$root/bundle/$file"
done
chmod 500 "$root/bundle"
cd "$root"
printf '%s\n' \
  'f18122dfa990b0044838db11c6d9291dd54e37fbe0492f675d04cfd4ce3bf109  run.sh' \
  'f68dd8ce74e4cd0d6b31c10ce30b3e429d2ed284233b53cd7a64040f80bdd095  inputs.sha256' | sha256sum --check
sha256sum --check inputs.sha256
/bin/sh -n run.sh
printf 'ROOT=%s\n' "$root"
find "$root" -maxdepth 2 -type f -printf '%m:%u:%g %p\n'
# No SUDO_*, case, cost, LD_* or caller-controlled fixture environment.
nohup /usr/bin/env -i PATH=/usr/sbin:/usr/bin:/sbin:/bin /bin/sh "$root/run.sh" >wrapper.log 2>&1 </dev/null &
printf 'WRAPPER_PID=%s\n' "$!"
