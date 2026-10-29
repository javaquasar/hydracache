#!/usr/bin/python3
"""Synthetic timeout target only. Never imports or executes HydraCache code."""
import json
import os
from pathlib import Path
import signal
import sys

EXPECTED = ['diagnostic_worker_policy::authority_registry::fixture::tests::runtime_cpu::runtime_cpu_measurement',
            '--exact', '--nocapture', '--test-threads=1']


def identity(pid):
    raw = Path(f'/proc/{pid}/stat').read_text()
    fields = raw[raw.rindex(')') + 2:].split()
    return dict(pid=pid, state=fields[0], ppid=int(fields[1]), pgrp=int(fields[2]),
                session=int(fields[3]), start_ticks=int(fields[19]))


def main():
    if sys.argv[1:] != EXPECTED or os.getuid() == 0: raise SystemExit('synthetic arguments/owner refused')
    root = Path(__file__).resolve().parent
    read_fd, write_fd = os.pipe()
    child = os.fork()
    if child == 0:
        os.close(read_fd)
        os.write(write_fd, b'ready'); os.close(write_fd)
        while True: signal.pause()
    os.close(write_fd)
    if os.read(read_fd, 5) != b'ready': raise SystemExit('child handshake refused')
    os.close(read_fd)
    value = dict(schema='authority-native-synthetic-timeout-v1', uid=os.getuid(),
                 leader=identity(os.getpid()), child=identity(child), product=False, measurement=False)
    with (root / 'ready.json').open('x') as stream:
        json.dump(value, stream); stream.flush(); os.fsync(stream.fileno())
    print('HC074-SYNTHETIC-TIMEOUT-ONLY', flush=True)
    while True: signal.pause()


if __name__ == '__main__': main()
