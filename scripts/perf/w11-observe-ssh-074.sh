#!/bin/sh
set -eu
if [ "$#" -ne 0 ] || [ "${SSH_ORIGINAL_COMMAND-}" != observe-v2 ]; then
    printf '%s\n' 'unexpected SSH observer command' >&2
    exit 64
fi
exec /usr/bin/env -i PATH=/usr/bin:/bin LANG=C.UTF-8 \
    /usr/bin/sudo -n -- /opt/hydracache-w11-observer/tooling/scripts/perf/w11-observe-unit-074.sh
