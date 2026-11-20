#!/bin/sh
set -eu
if [ "$#" -ne 0 ] || [ "${SUDO_USER-}" != hydracache-admin ] || [ "$(/usr/bin/id -u)" -ne 0 ]; then
    printf '%s\n' 'unexpected observer unit launcher' >&2
    exit 64
fi
exec /usr/bin/systemd-run --quiet --wait --pipe --collect --unit=w11-build-observer-status-074 \
    --property=Type=exec \
    --property=WorkingDirectory=/opt/hydracache-w11-observer/tooling \
    --property=CPUAffinity=0 --property=RuntimeMaxSec=300 --property=TimeoutStopSec=5 \
    --property=ProtectHome=yes \
    -- /usr/bin/python3 -B -m scripts.perf.w11_observer_status_074
