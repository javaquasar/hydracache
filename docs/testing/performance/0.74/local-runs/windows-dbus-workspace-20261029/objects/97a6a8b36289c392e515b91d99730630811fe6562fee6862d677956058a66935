"""Distinct unsealed-file attribution, NEVER original-launcher qualification."""
import ctypes
import errno
import json
import os
from pathlib import Path
import signal
import tempfile
import probe as p


def main():
    assert os.getuid() == os.geteuid() == 0
    assert os.stat('/proc/self/ns/mnt').st_ino != os.stat('/proc/1/ns/mnt').st_ino
    with tempfile.TemporaryDirectory(prefix='hc074-bind-regular-control-') as tmp:
        parent = Path(tmp)
        source = parent / 'source'
        target = parent / 'launcher'
        source.write_bytes(b'namespace backed attribution only\n')
        source.chmod(0o555)
        target.touch(mode=0o444, exist_ok=False)
        before = source.stat()
        reader, writer = os.pipe2(os.O_CLOEXEC)
        child = os.fork()
        if child == 0:
            os.close(reader)
            try:
                fd = os.open(source, os.O_RDONLY | os.O_CLOEXEC)
                tree = p.syscall(428, ctypes.c_int(fd), ctypes.c_char_p(b''), ctypes.c_uint(1 | os.O_CLOEXEC | 0x1000))
                attr = (ctypes.c_ulonglong * 4)(7, 0, 0, 0)
                p.syscall(442, ctypes.c_int(tree), ctypes.c_char_p(b''), ctypes.c_uint(0x1000),
                          ctypes.byref(attr), ctypes.c_size_t(ctypes.sizeof(attr)))
                p.syscall(429, ctypes.c_int(tree), ctypes.c_char_p(b''), ctypes.c_int(-100),
                          ctypes.c_char_p(os.fsencode(target)), ctypes.c_uint(4))
                os.close(tree)
                os.close(fd)
                source.unlink()
                os.write(writer, b'mounted')
                signal.pause()
            except BaseException:
                os.write(writer, b'failed')
            os._exit(1)
        os.close(writer)
        reaped = False
        try:
            assert os.read(reader, 16) == b'mounted', 'Regular control failed before creator cut'
            os.kill(child, signal.SIGKILL)
            pid, status = os.waitpid(child, 0)
            reaped = True
            assert pid == child and os.WIFSIGNALED(status) and os.WTERMSIG(status) == signal.SIGKILL
            after = target.stat()
            assert (before.st_dev, before.st_ino, before.st_size) == (after.st_dev, after.st_ino, after.st_size)
            assert target.read_bytes() == b'namespace backed attribution only\n'
            assert os.statvfs(target).f_flag & 15 == 7
            try:
                writable = os.open(target, os.O_WRONLY)
            except OSError as error:
                assert error.errno == errno.EROFS
            else:
                os.close(writable)
                raise AssertionError('Writable alias')
            print(json.dumps(dict(regular_control=True, source_sealed=False, eligible_original_launcher=False,
                original_candidate_pass=False, source_fd_closed=True, creator_sigkill=True,
                mount_survived=True, readonly=True, private_namespace=True, server=False, product=False)), flush=True)
        finally:
            if not reaped:
                try: os.kill(child, signal.SIGKILL)
                except ProcessLookupError: pass
                os.waitpid(child, 0)
            os.close(reader)
            if os.path.ismount(target):
                assert target.parent == parent and str(target).startswith('/tmp/hc074-bind-regular-control-')
                assert p.libc.umount2(ctypes.c_char_p(os.fsencode(target)), ctypes.c_int(0)) == 0


if __name__ == '__main__': main()
