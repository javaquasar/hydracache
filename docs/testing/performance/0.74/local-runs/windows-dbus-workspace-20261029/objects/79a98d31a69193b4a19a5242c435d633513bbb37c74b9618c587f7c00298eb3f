"""External root fixture subreaper; actual creator SIGKILL, never admission."""
import argparse
import ctypes
import fcntl
import hashlib
import os
from pathlib import Path
import select
import signal
import socket
import subprocess
import tempfile
import time

from prearmed_watchdog import cleanup, credentials, stopped
from original_custody_owner_loss import locked, parent_pid, pidfd_pid, require, terminal

TEST = "diagnostic_two_phase_launch::root_enrollment::kernel::guardian::creator::root_creator_victim"
CASES = [(f"cut-{cut}", cut, None) for cut in range(1, 6)] + [
    ("first-dead", 4, (0, signal.SIGKILL)),
    ("second-dead", 4, (1, signal.SIGKILL)),
    ("first-stopped", 4, (0, signal.SIGSTOP)),
    ("second-stopped", 4, (1, signal.SIGSTOP)),
    ("lost-second-ack", 1, None),
    ("both-dead", 4, signal.SIGKILL),
    ("both-stopped", 4, signal.SIGSTOP),
]
SEED = 0x074C2022


def packet(stage):
    return b"HCRGCUT1" + bytes([stage]) + bytes(95)


def receive(peer, owner, stage, count):
    raw, ancillary, flags, _ = peer.recvmsg(104, 4096, socket.MSG_CMSG_CLOEXEC)
    rights, creds = credentials(ancillary)
    try:
        require(flags & ~socket.MSG_CMSG_CLOEXEC == 0 and raw == packet(stage), "cut carrier shape")
        require(creds == [(owner.pid, os.getuid(), os.getgid())], "cut carrier creator")
        require(len(rights) == count and all(not os.get_inheritable(fd) for fd in rights), "cut carrier rights")
        return rights
    except BaseException:
        for fd in rights:
            os.close(fd)
        raise


def timer(libc, deadline):
    class Timespec(ctypes.Structure):
        _fields_ = [("seconds", ctypes.c_long), ("nanos", ctypes.c_long)]

    class Itimerspec(ctypes.Structure):
        _fields_ = [("interval", Timespec), ("value", Timespec)]

    fd = libc.timerfd_create(time.CLOCK_BOOTTIME, os.O_NONBLOCK | os.O_CLOEXEC)
    require(fd >= 0, "create fixture alarm")
    value = Itimerspec(Timespec(0, 0), Timespec(*divmod(deadline, 1_000_000_000)))
    if libc.timerfd_settime(fd, 1, ctypes.byref(value), None) != 0:
        os.close(fd)
        raise RuntimeError("arm fixture alarm")
    return fd


def run(binary, libc, case, cut, fault):
    with tempfile.TemporaryDirectory(prefix="hc074-creator-", dir="/tmp") as directory:
        temporary = Path(directory)
        root = Path("/sys/fs/cgroup") / temporary.name
        root.mkdir()  # fresh single component; never adopt a pre-existing root
        fence_path = temporary / "fence"
        fence = os.open(fence_path, os.O_RDWR | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC, 0o600)
        fcntl.flock(fence, fcntl.LOCK_EX | fcntl.LOCK_NB)
        deadline = time.clock_gettime_ns(time.CLOCK_BOOTTIME) + 30_000_000_000
        alarm = timer(libc, deadline)
        peer, child = socket.socketpair(socket.AF_UNIX, socket.SOCK_SEQPACKET | socket.SOCK_CLOEXEC)
        peer.setsockopt(socket.SOL_SOCKET, socket.SO_PASSCRED, 1)
        peer.settimeout(10)  # harness only; never a new runtime admission window
        owner = None
        owner_pin = None
        guardians = []
        targets = []
        read_fd, hidden = os.pipe2(os.O_NONBLOCK | os.O_CLOEXEC)
        try:
            env = os.environ.copy()
            env.update(HC074_CREATOR_FIXTURE="local-only-v1", HC074_CREATOR_ROOT=root.name,
                       HC074_CREATOR_CUT=str(cut), HC074_CREATOR_FAULT="2" if case == "lost-second-ack" else "0",
                       HC074_CREATOR_FENCE=str(fence), HC074_CREATOR_ALARM=str(alarm),
                       HC074_CREATOR_PEER=str(child.fileno()), HC074_CREATOR_DEADLINE=str(deadline))
            with (temporary / "creator.log").open("wb") as log:
                owner = subprocess.Popen([str(binary), "--exact", TEST, "--ignored", "--nocapture", "--test-threads=1"],
                                         stdin=subprocess.PIPE, stdout=log, stderr=log, env=env,
                                         pass_fds=(fence, alarm, child.fileno()))
                owner_pin = os.pidfd_open(owner.pid)
                child.close()
                os.close(fence)
                fence = None
                os.close(alarm)
                alarm = None
                guardians = receive(peer, owner, 0, 2)
                pids = [pidfd_pid(fd) for fd in guardians]
                require(len(set(pids)) == 2 and owner.pid not in pids, "guardian roles")
                for pid in pids:
                    require(parent_pid(pid) == owner.pid and os.path.samefile(f"/proc/{pid}/exe", binary), "guardian origin")
                locked(fence_path, True)
                owner.stdin.write(b"G")
                owner.stdin.flush()
                receive(peer, owner, cut, 0)
                require(not terminal(owner_pin), "creator died before requested cut")
                if case == "lost-second-ack":
                    until = time.monotonic() + 5
                    while not stopped(pids[1]):
                        require(time.monotonic() < until, "missing retained-root pre-ACK stop")
                        select.select([], [], [], 0.001)
                selected = [fault] if isinstance(fault, tuple) else ([(0, fault), (1, fault)] if fault else [])
                for index, sig in selected:
                    signal.pidfd_send_signal(guardians[index], sig)
                    if sig == signal.SIGKILL:
                        require(terminal(guardians[index], 5), "guardian kill missing")
                    else:
                        until = time.monotonic() + 5
                        while not stopped(pids[index]):
                            require(time.monotonic() < until, "guardian stop missing")
                            select.select([], [], [], 0.001)
                before = fence_path.stat()
                def target(nested, output=False):
                    destination = root / "nested" if nested else root
                    if nested and not destination.exists():
                        destination.mkdir()
                    process = subprocess.Popen(["/bin/cat"], stdin=subprocess.PIPE,
                                               stdout=hidden if output else subprocess.DEVNULL,
                                               stderr=subprocess.DEVNULL, user=65534, group=65534)
                    pin = os.pidfd_open(process.pid)
                    targets.append((process, pin))  # own cleanup before migration can fail
                    with (destination / "cgroup.procs").open("wb", buffering=0) as procs:
                        require(procs.write(str(process.pid).encode()) == len(str(process.pid)), "migration")
                    return process, pin

                direct = target(False, True)
                nested = target(True)
                require(not terminal(direct[1]) and not terminal(nested[1]), "writers died before creator cut")
                require(time.clock_gettime_ns(time.CLOCK_BOOTTIME) < deadline, "creator cut reached expiry")
                signal.pidfd_send_signal(owner_pin, signal.SIGKILL)
                require(owner.wait(timeout=5) == -signal.SIGKILL, "creator not actual SIGKILL")
                for pid in pids:
                    require(parent_pid(pid) == os.getpid(), "guardian not adopted by external subreaper")
                negative = case in ("both-dead", "both-stopped")
                if negative:
                    require(all(not terminal(fd, 0.1) for _, fd in [direct, nested]), "common-mode control lost live writer")
                    locked(fence_path, case == "both-stopped")
                else:
                    for process, pin in [direct, nested]:
                        require(terminal(pin, 5) and process.wait(timeout=5) == -signal.SIGKILL, "target not actual SIGKILL")
                    # Owner-loss must revoke before the unchanged fixture alarm.
                    require(time.clock_gettime_ns(time.CLOCK_BOOTTIME) < deadline, "expiry substituted for creator death")
                    require(not terminal(read_fd), "external writer mistaken for cessation")
                    os.close(hidden)
                    hidden = None
                    require(terminal(read_fd, 5) and os.read(read_fd, 1) == b"", "missing EOF after last external alias closes")
                    locked(fence_path, True)
                    for wave in range(4):
                        process, pin = target(wave % 2 == 0)
                        require(terminal(pin, 5) and process.wait(timeout=5) == -signal.SIGKILL, "late writer survived")
                        locked(fence_path, True)
                after = fence_path.stat()
                require((before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns, before.st_ctime_ns)
                        == (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns, after.st_ctime_ns), "fence changed")
                print(f"HC074-ROOT-CREATOR case={case} seed=0x{SEED:08x} cut={cut} creator_sigkill=true "
                      f"expected_negative={negative} deadline={deadline} original_authority=false completion=false", flush=True)
        except BaseException:
            print((temporary / "creator.log").read_text(errors="replace")[:8192], flush=True)
            raise
        finally:
            if owner is not None:
                if owner.poll() is None:
                    signal.pidfd_send_signal(owner_pin, signal.SIGKILL)
                    owner.wait(timeout=5)
                owner.stdin.close()
            # Only exact owned targets and this dedicated subreaper's adopted
            # libtest children; never signal a service or another fixture.
            for process, pin in targets:
                if process.poll() is None:
                    signal.pidfd_send_signal(pin, signal.SIGKILL)
                process.wait(timeout=5)
                process.stdin.close()
                os.close(pin)
            cleanup(binary)
            for fd in [*guardians, owner_pin, fence, alarm, read_fd, hidden]:
                if fd is not None:
                    os.close(fd)
            child.close()
            peer.close()
            if (root / "nested").exists():
                (root / "nested").rmdir()
            root.rmdir()


def main():
    parser = argparse.ArgumentParser()
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--binary", type=Path)
    group.add_argument("--binary-env", choices=["HC074_CREATOR_TEST_BINARY"])
    parser.add_argument("--case", choices=[case[0] for case in CASES])
    args = parser.parse_args()
    require(os.getuid() == 0 and os.geteuid() == 0, "isolated local root fixture required")
    require(os.environ.get("HC074_CREATOR_FIXTURE") == "local-only-v1", "explicit local fixture marker")
    binary = (args.binary if args.binary else Path(os.environ[args.binary_env])).resolve(strict=True)
    require(binary.name.startswith("hydracache_long_run_supervisor_074-") and os.access(binary, os.X_OK), "libtest image")
    libc = ctypes.CDLL(None, use_errno=True)
    previous = ctypes.c_int()
    require(libc.prctl(37, ctypes.byref(previous), 0, 0, 0) == 0, "get subreaper")
    require(libc.prctl(36, 1, 0, 0, 0) == 0, "set subreaper")
    try:
        print("HC074-ROOT-CREATOR image_sha256=" + hashlib.sha256(binary.read_bytes()).hexdigest(), flush=True)
        selected = [case for case in CASES if args.case is None or case[0] == args.case]
        for case in selected:
            run(binary, libc, *case)
        print(f"HC074-ROOT-CREATOR completed=true cases={len(selected)} server=false product=false", flush=True)
    finally:
        try:
            cleanup(binary)
        finally:
            require(libc.prctl(36, previous.value, 0, 0, 0) == 0, "restore subreaper")


if __name__ == "__main__":
    main()
