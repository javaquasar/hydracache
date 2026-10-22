"""Offline-tested cleanup for a future disposable-namespace control only."""
import ctypes
import errno
import os
from pathlib import Path


def unmount_owned_leaf(target, parent, libc):
    target, parent = Path(target), Path(parent)
    if target.parent != parent or target.name != 'launcher' or parent.parent != Path('/tmp') or \
       not parent.name.startswith('hc074-bind-regular-control-'):
        raise ValueError('Outside the exact disposable fixture')
    # ismount() misses bind mounts on the same filesystem. Never use it to
    # decide whether this owned leaf needs a cleanup attempt.
    result = libc.umount2(ctypes.c_char_p(os.fsencode(target)), ctypes.c_int(0))
    if result < 0:
        code = ctypes.get_errno()
        if code != errno.EINVAL: raise OSError(code, os.strerror(code))
