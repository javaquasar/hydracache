#!/bin/sh
set -eu
umask 077
cd "$1"
. ./guard.sh
[ ! -e /opt/hydracache-performance/0.74/diagnostic-pilot ]
[ ! -e ./bundle ]
mkdir -m 700 ./bundle
for name in timing-controls-074 build-receipt-v1.json build-log.jsonl Cargo.lock.root Cargo.lock.observer embedded.json direct.json resp2.json resp3.json; do
    cp -- "/tmp/hc074-safety-alarm-20261017.dzh0Im/bundle/$name" "./bundle/$name"
    chmod 444 "./bundle/$name"
done
chmod 555 ./bundle/timing-controls-074 ./bundle
sha256sum ./bundle/*
printf 'HC074-GROUPED-PROCESS-SETUP nonroot=true bundle_executed=false production_changed=false\n'
