#!/bin/sh
set -eu
if [ "$#" -ne 0 ] || [ "${SUDO_USER-}" != hydracache-admin ] || [ "$(/usr/bin/id -u)" -ne 0 ]; then
    printf '%s\n' 'unexpected observer unit launcher' >&2
    exit 64
fi
exec /usr/bin/systemd-run --quiet --wait --pipe --collect --unit=w11-build-observer-ssh-074 \
    --property=Type=exec --property=DynamicUser=yes --property=User=hydracache-w11-observer \
    --property=StateDirectory=hydracache-w11-observer --property=StateDirectoryMode=0700 \
    --property=WorkingDirectory=/opt/hydracache-w11-observer/tooling \
    --property='CPUAffinity=0 1 2' --property=RuntimeMaxSec=300 --property=TimeoutStopSec=5 \
    --property=NoNewPrivileges=yes --property=CapabilityBoundingSet= \
    --property=ProtectHome=yes --property=PrivateNetwork=yes \
    --property=InaccessiblePaths=-/run/hydracache-perf \
    -- /bin/sh -c 'exec /usr/bin/env -i PATH=/usr/bin:/bin LANG=C.UTF-8 SSH_ORIGINAL_COMMAND=observe-v2 "STATE_DIRECTORY=${STATE_DIRECTORY:?}" /usr/bin/python3 -B -m scripts.perf.w11_ssh_observer_074 serve'
