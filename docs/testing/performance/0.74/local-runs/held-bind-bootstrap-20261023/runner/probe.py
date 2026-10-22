"""One local private-namespace memfd mount precondition; no worker or services."""
import ctypes
import json
import os
import platform
from pathlib import Path
import tempfile

libc = ctypes.CDLL(None, use_errno=True)


def syscall(number, *args):
    result = libc.syscall(ctypes.c_long(number), *args)
    if result < 0: raise OSError(ctypes.get_errno(), os.strerror(ctypes.get_errno()))
    return result


def main():
    assert os.getuid() == os.geteuid() == 0
    assert platform.machine() == 'x86_64'
    assert os.stat('/proc/self/ns/mnt').st_ino != os.stat('/proc/1/ns/mnt').st_ino
    with tempfile.TemporaryDirectory(prefix='hc074-bind-precondition-') as tmp:
        source = os.memfd_create('hc074-original-sealed', os.MFD_CLOEXEC | os.MFD_ALLOW_SEALING)
        os.write(source, b'original sealed image precondition\n')
        os.fchmod(source, 0o555)
        import fcntl
        fcntl.fcntl(source, fcntl.F_ADD_SEALS, 15)
        readonly = os.open('/proc/self/fd/' + str(source), os.O_RDONLY | os.O_CLOEXEC)
        os.close(source)
        tree = None
        attached = False
        target = Path(tmp) / 'launcher'
        target.touch(mode=0o444, exist_ok=False)
        try:
            tree = syscall(428, ctypes.c_int(readonly), ctypes.c_char_p(b''), ctypes.c_uint(1 | os.O_CLOEXEC | 0x1000))
            print('detached_original_inode=true', flush=True)
            attributes = (ctypes.c_ulonglong * 4)(7, 0, 0, 0)
            syscall(442, ctypes.c_int(tree), ctypes.c_char_p(b''), ctypes.c_uint(0x1000),
                    ctypes.byref(attributes), ctypes.c_size_t(ctypes.sizeof(attributes)))
            syscall(429, ctypes.c_int(tree), ctypes.c_char_p(b''), ctypes.c_int(-100),
                    ctypes.c_char_p(os.fsencode(target)), ctypes.c_uint(4))
            attached = True
            before = os.fstat(readonly)
            after = target.stat()
            assert (before.st_dev, before.st_ino, before.st_size) == (after.st_dev, after.st_ino, after.st_size)
            flags = os.statvfs(target).f_flag
            assert flags & 15 == 7
            print(json.dumps(dict(original_inode=True, readonly=True, private_namespace=True,
                                  service=False, server=False, product=False)), flush=True)
        finally:
            if attached:
                assert target.parent == Path(tmp) and str(target).startswith('/tmp/hc074-bind-precondition-')
                assert libc.umount2(ctypes.c_char_p(os.fsencode(target)), ctypes.c_int(0)) == 0
            if tree is not None: os.close(tree)
            os.close(readonly)


if __name__ == '__main__': main()
