"""Owned nonroot pre-armed mechanics; no systemd, product or original authority."""
import argparse
import array
import ctypes
import fcntl
import hashlib
import json
import os
from pathlib import Path
import select
import signal
import socket
import stat
import struct
import subprocess
import tempfile
import time

from original_custody_owner_loss import locked, parent_pid, pidfd_pid, require, terminal

PREFIX = "diagnostic_two_phase_launch::held_barrier::prearmed::tests::process::"
CASES = ["pending-owner-loss", "armed-owner-loss", "peer-loss", "expiry", "hidden-writer",
         "lost-ack", "foreign-sender", "wrong-right", "duplicate-wire", "guardian-death",
         "guardian-death-after-release", "guardian-stopped-after-release"]
SEED = 0x0740A019


def cleanup(binary):
    # Only still-unreaped direct/adopted children of this single driver.
    for raw in Path(f"/proc/self/task/{os.getpid()}/children").read_text().split():
        pid = int(raw)
        require(parent_pid(pid) == os.getpid(), "cleanup parent")
        fd = os.pidfd_open(pid)
        try:
            if not terminal(fd):
                require(os.path.samefile(f"/proc/{pid}/exe", binary), "unexpected adopted executable")
                signal.pidfd_send_signal(fd, signal.SIGKILL)
            os.waitid(os.P_PIDFD, fd, os.WEXITED)
        finally:
            os.close(fd)


def credentials(ancillary):
    rights, creds = [], []
    for level, kind, data in ancillary:
        require(level == socket.SOL_SOCKET, "ancillary level")
        if kind == socket.SCM_RIGHTS:
            values = array.array("i")
            values.frombytes(data)
            rights.extend(values)
        elif kind == socket.SCM_CREDENTIALS:
            require(len(data) == 12, "credential shape")
            creds.append(struct.unpack("3i", data))
        else:
            raise RuntimeError("unknown ancillary")
    return rights, creds


def stopped(pid):
    state = next(line.split()[1] for line in Path(f"/proc/{pid}/status").read_text().splitlines()
                 if line.startswith("State:"))
    return state == "T"


def run(binary, case):
    with tempfile.TemporaryDirectory(prefix="hc074-prearmed-", dir="/tmp") as directory:
        root = Path(directory)
        env = os.environ.copy()
        env.update(HC074_PREARMED_ROOT=directory, HC074_PREARMED_CASE=case)
        with socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM | socket.SOCK_CLOEXEC) as peer, \
                (root / "victim.log").open("wb") as log:
            peer.setsockopt(socket.SOL_SOCKET, socket.SO_PASSCRED, 1)
            peer.bind(str(root / "notice.sock"))
            peer.settimeout(15)  # Harness only, never an authority window.
            owner = subprocess.Popen([str(binary), "--exact", PREFIX + "prearmed_owner_victim",
                                      "--ignored", "--nocapture", "--test-threads=1"],
                                     stdin=subprocess.PIPE, stdout=log, stderr=log, env=env)
            owner_pin = os.pidfd_open(owner.pid)  # exact still-unreaped Popen child
            received = []
            guardian = target = reader = hidden = None
            try:
                raw, anc, flags, _ = peer.recvmsg(2048, socket.CMSG_SPACE(16) + socket.CMSG_SPACE(12),
                                                 socket.MSG_CMSG_CLOEXEC)
                received, creds = credentials(anc)
                require(flags & ~socket.MSG_CMSG_CLOEXEC == 0, "carrier truncation/flags")
                require(creds == [(owner.pid, os.getuid(), os.getgid())], "carrier owner credentials")
                packet = json.loads(raw)
                require(set(packet) == {"schema", "case", "owner", "deadline", "pending", "armed"}, "carrier fields")
                require(packet["schema"] == "hc074-prearmed-owner-test-v1" and packet["case"] == case
                        and packet["owner"] == owner.pid and packet["pending"] is True, "carrier binding")
                early = case in ["lost-ack", "foreign-sender", "wrong-right", "pending-owner-loss"]
                require(packet["armed"] is (not early), "armed cut")
                expected = 1 if case == "pending-owner-loss" else (4 if case == "hidden-writer" else 3)
                require(len(received) == expected and all(not os.get_inheritable(fd) for fd in received), "rights")
                guardian = received[0]
                gpid = pidfd_pid(guardian)
                require(parent_pid(gpid) == owner.pid and os.path.samefile(f"/proc/{gpid}/exe", binary), "guardian origin")
                deadline = packet["deadline"]
                require(type(deadline) is int and time.clock_gettime_ns(time.CLOCK_BOOTTIME) < deadline, "original bound")
                require(not terminal(guardian), "guardian already dead")
                lock = root / "fence"
                before = lock.stat()
                require(stat.S_ISREG(before.st_mode) and before.st_size == 0 and before.st_nlink == 1, "fence shape")
                locked(lock, True)
                if expected > 1:
                    target, reader = received[1:3]
                    tpid = pidfd_pid(target)
                    require(parent_pid(tpid) == owner.pid and os.path.samefile(f"/proc/{tpid}/exe", binary), "target origin")
                    require(stat.S_ISFIFO(os.fstat(reader).st_mode), "output not a pipe")
                    require(fcntl.fcntl(reader, fcntl.F_GETFL) & os.O_ACCMODE == os.O_RDONLY, "output mode")
                    require(fcntl.fcntl(reader, fcntl.F_GETFL) & os.O_NONBLOCK, "output blocking")
                    if expected == 4:
                        hidden = received[3]
                        require(os.fstat(hidden).st_ino == os.fstat(reader).st_ino, "foreign writer alias")
                    if not early and case != "guardian-death":
                        require(terminal(reader, 5) and os.read(reader, 16) == b"E", "released effect missing")
                    else:
                        require(not terminal(reader), "effect before release")

                def kill_owner():
                    if owner.returncode is None:
                        signal.pidfd_send_signal(owner_pin, signal.SIGKILL)
                        require(owner.wait(timeout=5) == -signal.SIGKILL, "owner not SIGKILL")

                guardian_dead = case in ["guardian-death", "guardian-death-after-release"]
                if case == "guardian-death-after-release":
                    signal.pidfd_send_signal(guardian, signal.SIGKILL)
                    require(terminal(guardian, 5), "released guardian not killed")
                    require(not terminal(target) and not terminal(reader), "negative lost live writer")
                    locked(lock, False)
                elif case == "guardian-stopped-after-release":
                    signal.pidfd_send_signal(guardian, signal.SIGSTOP)
                    until = time.monotonic() + 5
                    while not stopped(gpid):
                        require(time.monotonic() < until and not terminal(guardian), "missing active guardian stop")
                        select.select([], [], [], 0.005)
                    kill_owner()
                    state = os.waitid(os.P_PIDFD, guardian, os.WSTOPPED | os.WNOWAIT)
                    require(state.si_code == os.CLD_STOPPED and state.si_status == signal.SIGSTOP, "active stop cut")
                    remaining = (deadline - time.clock_gettime_ns(time.CLOCK_BOOTTIME)) / 1e9
                    if remaining > 0:
                        require(not terminal(target, remaining), "stopped guardian unexpectedly revoked")
                    require(time.clock_gettime_ns(time.CLOCK_BOOTTIME) >= deadline, "original expiry not reached")
                    require(not terminal(target) and not terminal(reader), "negative missing live writer at expiry")
                    locked(lock, True)
                    signal.pidfd_send_signal(guardian, signal.SIGCONT)
                elif case in ["pending-owner-loss", "armed-owner-loss", "hidden-writer"]:
                    kill_owner()
                else:
                    if case == "guardian-death":
                        signal.pidfd_send_signal(guardian, signal.SIGKILL)
                        require(terminal(guardian, 5), "guardian not killed")
                    owner.stdin.write(b"G")
                    owner.stdin.flush()
                    if case == "lost-ack":
                        until = time.monotonic() + 5
                        while not stopped(gpid):
                            require(time.monotonic() < until and not terminal(guardian), "missing pre-ACK stop")
                            select.select([], [], [], 0.005)
                        kill_owner()
                        state = os.waitid(os.P_PIDFD, guardian, os.WSTOPPED | os.WNOWAIT)
                        require(state.si_code == os.CLD_STOPPED and state.si_status == signal.SIGSTOP, "ACK cut")
                        signal.pidfd_send_signal(guardian, signal.SIGCONT)
                    else:
                        done, anc, flags, _ = peer.recvmsg(1, socket.CMSG_SPACE(12), socket.MSG_CMSG_CLOEXEC)
                        extra, creds = credentials(anc)
                        require(not extra and done == b"D" and flags & ~socket.MSG_CMSG_CLOEXEC == 0
                                and creds == [(owner.pid, os.getuid(), os.getgid())], "command completion")
                if case in ["foreign-sender", "wrong-right"]:
                    # Rejected received pidfd must NOT become signal authority.
                    require(not terminal(target) and not terminal(reader), "untrusted target signaled or released")
                    kill_owner()
                if case == "pending-owner-loss":
                    remaining = (deadline - time.clock_gettime_ns(time.CLOCK_BOOTTIME)) / 1e9
                    if remaining > 0:
                        require(not terminal(guardian, remaining), "pending custody expired by exit")
                    locked(lock, True)
                elif target is not None and case != "guardian-death-after-release":
                    require(terminal(target, 12), "guardian failed to terminate target")
                    if case in ["armed-owner-loss", "hidden-writer", "peer-loss", "duplicate-wire", "expiry",
                                "guardian-stopped-after-release"]:
                        # Inspect without reaping: owner may still be the parent.
                        kill_owner()
                        status = os.waitid(os.P_PIDFD, target, os.WEXITED | os.WNOWAIT)
                        require(status.si_code == os.CLD_KILLED and status.si_status == signal.SIGKILL, "target not actual SIGKILL")
                    if case == "hidden-writer":
                        require(not terminal(reader), "hidden alias incorrectly treated as EOF")
                        os.close(hidden)
                        received.remove(hidden)
                        hidden = None
                    require(terminal(reader, 5) and os.read(reader, 16) == b"", "actual EOF missing or unexpected effect")
                    locked(lock, case != "guardian-death")
                if case == "expiry":
                    require(time.clock_gettime_ns(time.CLOCK_BOOTTIME) >= deadline, "expiry was refreshed/early")
                if not guardian_dead:
                    require(not terminal(guardian), "guardian lost fence custody")
                    require(str(lock) in [os.readlink(p) for p in Path(f"/proc/{gpid}/fd").iterdir()], "fence no longer retained")
                after = lock.stat()
                require((before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns, before.st_ctime_ns)
                        == (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns, after.st_ctime_ns), "fence modified")
                print(f"HC074-PREARMED case={case} seed=0x{SEED:08x} deadline={deadline} "
                      f"fence_retained={not guardian_dead} product=false original_authority=false", flush=True)
            finally:
                if owner.returncode is None:
                    if not terminal(owner_pin):
                        signal.pidfd_send_signal(owner_pin, signal.SIGKILL)
                    owner.wait(timeout=5)
                owner.stdin.close()
                os.close(owner_pin)
                cleanup(binary)
                for fd in received:
                    os.close(fd)


def main():
    parser = argparse.ArgumentParser()
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--binary", type=Path)
    group.add_argument("--binary-env", choices=["HC074_PREARMED_TEST_BINARY"])
    parser.add_argument("--case", choices=CASES)
    args = parser.parse_args()
    binary = (args.binary if args.binary is not None else Path(os.environ[args.binary_env])).resolve(strict=True)
    require(binary.name.startswith("hydracache_long_run_supervisor_074-") and os.access(binary, os.X_OK), "libtest image required")
    require(os.getuid() != 0 and os.geteuid() == os.getuid(), "nonroot only")
    libc = ctypes.CDLL(None, use_errno=True)
    previous = ctypes.c_int()
    require(libc.prctl(37, ctypes.byref(previous), 0, 0, 0) == 0, "get subreaper")
    require(libc.prctl(36, 1, 0, 0, 0) == 0, "set subreaper")
    try:
        print("HC074-PREARMED image_sha256=" + hashlib.sha256(binary.read_bytes()).hexdigest(), flush=True)
        for case in ([args.case] if args.case else CASES):
            run(binary, case)
        print("HC074-PREARMED completed=true cases=" + str(1 if args.case else len(CASES)), flush=True)
    finally:
        try:
            cleanup(binary)
        finally:
            require(libc.prctl(36, previous.value, 0, 0, 0) == 0, "restore subreaper")


if __name__ == "__main__":
    main()
