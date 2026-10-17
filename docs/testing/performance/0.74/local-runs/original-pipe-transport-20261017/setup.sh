#!/bin/sh
set -eu
umask 077
cd /tmp/hc074-pipe-transport-20261017.b9l0y9
. ./guard.sh
[ ! -e ./bundle ]
mkdir -m 700 ./bundle
for name in timing-controls-074 build-receipt-v1.json build-log.jsonl Cargo.lock.root Cargo.lock.observer embedded.json direct.json resp2.json resp3.json; do
    cp -- "/tmp/hc074-start-material-20261017.vLX1GL/bundle/$name" "./bundle/$name"
    chmod 444 "./bundle/$name"
done
chmod 555 ./bundle/timing-controls-074 ./bundle
sha256sum ./bundle/*
printf 'HC074-PIPE-TRANSPORT-SETUP nonroot=true bundle_executed=false\n'
