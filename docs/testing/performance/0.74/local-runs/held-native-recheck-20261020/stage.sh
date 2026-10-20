#!/bin/sh
# Exact reviewed root copy; never execute from the admin upload directory.
set -eu
[ "$(id -u):$(id -g)" = 0:0 ]
umask 077
upload=/tmp/hc074-native-recheck-upload-20261020.OiTtzO
[ "$(stat -c '%a:%u:%g' "$upload")" = 700:1000:1000 ]
[ ! -L "$upload" ]
root=$(mktemp -d /tmp/hc074-native-recheck-root-20261020.XXXXXX)
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
 '5d52bde41aedce9ac1a7d5b9943d32fd3145bee8f789bfb20419cf17c8a10534  run.sh' \
 '9435b7c519e0368b459aee9cb765c11a6ca2c0373c1ac27ec5a8e937026c9da0  inputs.sha256' | sha256sum --check
sha256sum --check inputs.sha256
/bin/sh -n run.sh
printf 'ROOT=%s\n' "$root"
find "$root" -maxdepth 2 -type f -printf '%m:%u:%g %p\n'
nohup /usr/bin/env -i PATH=/usr/sbin:/usr/bin:/sbin:/bin /bin/sh "$root/run.sh" >wrapper.log 2>&1 </dev/null &
printf 'WRAPPER_PID=%s\n' "$!"
