#!/bin/sh
set -eu
cd /opt/hydracache-w11-observer/tooling
exec /usr/bin/env -i PATH=/usr/bin:/bin LANG=C.UTF-8 "SSH_ORIGINAL_COMMAND=${SSH_ORIGINAL_COMMAND-}" \
    /usr/bin/taskset -c 0,1,2 /usr/bin/python3 -B -m scripts.perf.w11_ssh_observer_074 serve
