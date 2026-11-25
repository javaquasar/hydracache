"""Read the isolated daemon's limits and join them to its original executable."""

from array import array
from contextlib import contextmanager, ExitStack
from dataclasses import dataclass
from hashlib import file_digest, sha256
import os
from pathlib import Path
import select
import socket
import stat
from struct import Struct

from . import performance_signed_role_probe_074 as probe
from . import w11_product_build_receipt_074 as receipt


SOCKET = "w11-effective-config.sock"
BOOT_ID = Path("/proc/sys/kernel/random/boot_id")
MAX_BYTES = 256
CREDENTIALS = Struct("=iII")
READY = b'{"status":"ok"}\n'


@dataclass(frozen=True)
class ConfigObservation:
    configuration: dict
    generation: dict
    raw: bytes

    @property
    def sha256(self):
        return sha256(self.raw).hexdigest()


@contextmanager
def private_channel(directory):
    meta = directory.lstat()
    probe.require(directory.resolve(strict=True) == directory and stat.S_ISDIR(meta.st_mode)
                  and meta.st_uid == os.getuid() and stat.S_IMODE(meta.st_mode) == 0o700,
                  "config socket requires the private launch directory")
    with socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM) as reader:
        reader.setsockopt(socket.SOL_SOCKET, socket.SO_PASSCRED, 1)
        reader.bind(str(directory / SOCKET))
        yield reader


@contextmanager
def observe_child(child, image, binary, expected_boot):
    pinned, _ = probe.read_pinned(image, binary["sha256"])
    probe.require(pinned.st_size == binary["bytes"], "expected binary size differs")
    with ExitStack() as held:
        process = os.pidfd_open(child.pid)
        held.callback(os.close, process)
        proc = os.open(f"/proc/{child.pid}", os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC)
        held.callback(os.close, proc)
        poll = select.poll()
        poll.register(process, select.POLLIN | select.POLLHUP | select.POLLERR)

        def verify_live():
            probe.require(child.poll() is None and not poll.poll(0), "original daemon exited")
            probe.require(BOOT_ID.read_text().strip() == expected_boot, "daemon boot changed")

        def exe():
            return os.fdopen(os.open("exe", os.O_RDONLY | os.O_CLOEXEC, dir_fd=proc), "rb")

        def verify_exe(stream):
            live = os.fstat(stream.fileno())
            probe.require(os.path.samestat(pinned, live) and live.st_size == binary["bytes"],
                          "running daemon has a different executable")

        def snapshot():
            verify_live()
            fd = os.open("stat", os.O_RDONLY | os.O_CLOEXEC, dir_fd=proc)
            with os.fdopen(fd, "rb") as stream:
                state = stream.read(4097)
            probe.require(len(state) <= 4096 and state.startswith(f"{child.pid} (".encode()),
                          "unexpected daemon proc stat")
            fields = state.rsplit(b")", 1)[1].split()
            probe.require(len(fields) > 19 and fields[19].isdigit(), "missing original start ticks")
            with exe() as stream:
                verify_exe(stream)
            verify_live()
            return {"boot_id": expected_boot, "pid": child.pid, "start_ticks": int(fields[19]),
                    "elf_sha256": binary["sha256"]}

        verify_live()
        with exe() as stream:
            verify_exe(stream)
            probe.require(file_digest(stream, "sha256").hexdigest() == binary["sha256"],
                          "original executable digest differs from its receipt")
            verify_exe(stream)
        original = snapshot()
        receipt.generation(original, binary["sha256"])

        def observe():
            current = snapshot()
            probe.require(current == original, "original daemon generation changed")
            return current

        yield observe


def read_configuration(reader, observe, selector, ready, deadline):
    probe.require(ready == READY, "original readiness response differs")
    probe.require(selector in (None, receipt.SELECTOR), "unexpected expected profile")
    before = observe()
    reader.settimeout(min(5, probe.seconds_left(deadline)))
    raw, ancillary, flags, _ = reader.recvmsg(
        MAX_BYTES + 1, socket.CMSG_SPACE(CREDENTIALS.size) + socket.CMSG_SPACE(4 * 4),
        socket.MSG_CMSG_CLOEXEC)
    for level, kind, data in ancillary:
        if level == socket.SOL_SOCKET and kind == socket.SCM_RIGHTS:
            received = array("i")
            received.frombytes(data[:len(data) - len(data) % received.itemsize])
            for descriptor in received:
                os.close(descriptor)
    probe.require(not flags & (socket.MSG_TRUNC | socket.MSG_CTRUNC) and 0 < len(raw) <= MAX_BYTES,
                  "truncated or oversized config readback")
    probe.require(len(ancillary) == 1 and ancillary[0][:2] == (socket.SOL_SOCKET, socket.SCM_CREDENTIALS)
                  and len(ancillary[0][2]) == CREDENTIALS.size, "config readback credential envelope")
    peer = CREDENTIALS.unpack(ancillary[0][2])
    probe.require(peer == (before["pid"], os.getuid(), os.getgid()), "config readback sender differs")
    configuration = receipt.build.decode_json(raw)
    probe.require(raw == receipt.canonical(configuration)
                  and raw == receipt.canonical(receipt.config(selector)),
                  "observed configuration differs from approved limits")
    probe.require(observe() == before, "daemon changed during config readback")
    pending = select.poll()
    pending.register(reader, select.POLLIN)
    probe.require(not pending.poll(0), "duplicate config readback")
    probe.seconds_left(deadline)
    return ConfigObservation(configuration, before, raw)
