"""Owned local single-guardian-failure experiment; no original/server authority."""
import argparse
import ctypes
import hashlib
import json
import os
from pathlib import Path
import select
import signal
import socket
import stat
import subprocess
import sys
import tempfile
import time

from prearmed_watchdog import cleanup, credentials, stopped
from original_custody_owner_loss import locked, parent_pid, pidfd_pid, require, terminal

PREFIX = "diagnostic_two_phase_launch::held_barrier::prearmed::tests::process::redundant::"
CASES = ["healthy-expiry", "owner-loss", "first-dead-owner-loss", "second-dead-owner-loss",
         "first-stopped-owner-loss", "second-stopped-owner-loss", "first-dead-expiry",
         "second-dead-expiry", "first-stopped-expiry", "second-stopped-expiry",
         "lost-second-ack-first-dead", "lost-second-ack-first-stopped", "partial-enrollment-loss",
         "duplicate-global", "hidden-writer-first-dead", "both-dead", "both-stopped"]
SEED = 0x0740D019


def run(binary, case):
    with tempfile.TemporaryDirectory(prefix="hc074-redundant-", dir="/tmp") as directory:
        root = Path(directory)
        env = os.environ.copy()
        env.update(HC074_REDUNDANT_ROOT=directory, HC074_REDUNDANT_CASE=case)
        with socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM | socket.SOCK_CLOEXEC) as peer, \
                (root / "victim.log").open("wb") as log:
            peer.setsockopt(socket.SOL_SOCKET, socket.SO_PASSCRED, 1)
            peer.bind(str(root / "notice.sock"))
            peer.settimeout(15)  # Harness bound, never authority time.
            owner = subprocess.Popen([str(binary), "--exact", PREFIX + "redundant_watchdog_owner_victim",
                                      "--ignored", "--nocapture", "--test-threads=1"],
                                     stdin=subprocess.PIPE, stdout=log, stderr=log, env=env)
            owner_pin = os.pidfd_open(owner.pid)  # exact still-unreaped Popen child
            received = []
            try:
                raw, ancillary, flags, _ = peer.recvmsg(2048, 128, socket.MSG_CMSG_CLOEXEC)
                received, creds = credentials(ancillary)
                require(flags & ~socket.MSG_CMSG_CLOEXEC == 0, "carrier flags/truncation")
                require(creds == [(owner.pid, os.getuid(), os.getgid())], "carrier credentials")
                packet = json.loads(raw)
                early = case.startswith("lost-second-ack") or case == "partial-enrollment-loss"
                require(set(packet) == {"schema", "case", "owner", "deadline", "pending", "armed"}, "carrier shape")
                require(packet["schema"] == "hc074-redundant-owner-test-v1" and packet["case"] == case
                        and packet["owner"] == owner.pid and packet["pending"] is True
                        and packet["armed"] is (not early), "carrier binding")
                expected = 5 if case == "hidden-writer-first-dead" else 4
                require(len(received) == expected and all(not os.get_inheritable(fd) for fd in received), "rights")
                guardians = received[:2]
                target, reader = received[2:4]
                pids = [pidfd_pid(fd) for fd in [*guardians, target]]
                require(len(set(pids)) == 3 and owner.pid not in pids, "duplicate process roles")
                for pid in pids:
                    require(parent_pid(pid) == owner.pid and os.path.samefile(f"/proc/{pid}/exe", binary), "process origin")
                require(all(not terminal(fd) for fd in [*guardians, target]), "premature terminal process")
                require(stat.S_ISFIFO(os.fstat(reader).st_mode) and not os.get_blocking(reader), "reader shape")
                hidden = received[4] if expected == 5 else None
                if hidden is not None:
                    require(os.fstat(hidden).st_ino == os.fstat(reader).st_ino, "hidden writer identity")
                deadline = packet["deadline"]
                require(type(deadline) is int and time.clock_gettime_ns(time.CLOCK_BOOTTIME) < deadline, "original bound")
                fence = root / "fence"
                before = fence.stat()
                require(stat.S_ISREG(before.st_mode) and before.st_size == 0 and before.st_nlink == 1
                        and before.st_mode & 0o7777 == 0o600 and before.st_uid == os.getuid(), "fence shape")
                locked(fence, True)
                if early:
                    require(not terminal(reader), "effect before both Armed")
                else:
                    require(terminal(reader, 5) and os.read(reader, 16) == b"E", "missing released effect")

                def kill_owner():
                    if owner.returncode is None:
                        signal.pidfd_send_signal(owner_pin, signal.SIGKILL)
                        require(owner.wait(timeout=5) == -signal.SIGKILL, "owner not actual SIGKILL")

                def kill_guardian(index):
                    signal.pidfd_send_signal(guardians[index], signal.SIGKILL)
                    require(terminal(guardians[index], 5), "guardian not killed")

                def await_stop(index):
                    until = time.monotonic() + 5
                    while not stopped(pids[index]):
                        require(time.monotonic() < until and not terminal(guardians[index]), "missing actual stop")
                        select.select([], [], [], 0.005)

                def stop_guardian(index):
                    signal.pidfd_send_signal(guardians[index], signal.SIGSTOP)
                    await_stop(index)

                def send_command():
                    owner.stdin.write(b"G")
                    owner.stdin.flush()

                def done():
                    raw, ancillary, flags, _ = peer.recvmsg(1, 128, socket.MSG_CMSG_CLOEXEC)
                    extra, creds = credentials(ancillary)
                    try:
                        require(not extra and raw == b"D" and flags & ~socket.MSG_CMSG_CLOEXEC == 0
                                and creds == [(owner.pid, os.getuid(), os.getgid())], "command completion")
                    finally:
                        for fd in extra:
                            os.close(fd)

                if case == "both-dead":
                    kill_guardian(0)
                    kill_guardian(1)
                    require(not terminal(target) and not terminal(reader), "expected negative missing live writer")
                    locked(fence, False)
                elif case == "both-stopped":
                    stop_guardian(0)
                    stop_guardian(1)
                    kill_owner()
                    remaining = (deadline - time.clock_gettime_ns(time.CLOCK_BOOTTIME)) / 1e9
                    if remaining > 0:
                        require(not terminal(target, remaining), "both stopped unexpectedly revoked")
                    require(time.clock_gettime_ns(time.CLOCK_BOOTTIME) >= deadline
                            and not terminal(target) and not terminal(reader), "negative at original expiry missing")
                    locked(fence, True)
                    signal.pidfd_send_signal(guardians[1], signal.SIGCONT)
                elif early:
                    send_command()
                    await_stop(1)  # target retained by second guardian, before Armed ACK
                    if case == "partial-enrollment-loss":
                        kill_guardian(1)
                        done()  # fixture asserts global uncertainty, sticky refusal and no resend
                        require(not terminal(target) and not terminal(reader), "partial retry released/revoked target")
                        kill_owner()
                    else:
                        if case.endswith("first-dead"):
                            kill_guardian(0)
                        else:
                            stop_guardian(0)
                        kill_owner()
                        state = os.waitid(os.P_PIDFD, guardians[1], os.WSTOPPED | os.WNOWAIT)
                        require(state.si_code == os.CLD_STOPPED and state.si_status == signal.SIGSTOP, "ACK cut lost")
                        signal.pidfd_send_signal(guardians[1], signal.SIGCONT)
                else:
                    if case.startswith("first-dead") or case == "hidden-writer-first-dead":
                        kill_guardian(0)
                    elif case.startswith("second-dead"):
                        kill_guardian(1)
                    elif case.startswith("first-stopped"):
                        stop_guardian(0)
                    elif case.startswith("second-stopped"):
                        stop_guardian(1)
                    if case == "duplicate-global":
                        send_command()
                        done()
                        require(not terminal(target) and not terminal(reader), "retry sent another enrollment")
                    if "owner-loss" in case or case in ["hidden-writer-first-dead", "duplicate-global"]:
                        kill_owner()

                if case != "both-dead":
                    require(terminal(target, 12), "survivor failed to terminate target")
                    if case.endswith("expiry"):
                        require(time.clock_gettime_ns(time.CLOCK_BOOTTIME) >= deadline, "early/refreshed expiry")
                    if not early:
                        kill_owner()  # adopt, but do not reap target before checking actual status
                        status = os.waitid(os.P_PIDFD, target, os.WEXITED | os.WNOWAIT)
                        require(status.si_code == os.CLD_KILLED and status.si_status == signal.SIGKILL, "target not SIGKILL")
                    if hidden is not None:
                        require(not terminal(reader), "hidden writer mistaken for EOF")
                        os.close(hidden)
                        received.remove(hidden)
                    require(terminal(reader, 5) and os.read(reader, 16) == b"", "EOF or held-effect refusal missing")
                    locked(fence, True)
                    live = [pid for fd, pid in zip(guardians, pids[:2]) if not terminal(fd)]
                    require(live, "no retained custodian")
                    for pid in live:
                        require(str(fence) in [os.readlink(p) for p in Path(f"/proc/{pid}/fd").iterdir()], "custody alias lost")
                after = fence.stat()
                require((before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns, before.st_ctime_ns)
                        == (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns, after.st_ctime_ns), "fence modified")
                print(f"HC074-REDUNDANT case={case} seed=0x{SEED:08x} deadline={deadline} "
                      f"fence_retained={case != 'both-dead'} expected_negative={case in ['both-dead', 'both-stopped']} "
                      "product=false original_authority=false all_writers=false", flush=True)
            except BaseException:
                log.flush()
                print((root / "victim.log").read_text(errors="replace")[:8192], file=sys.stderr, flush=True)
                raise
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
    group.add_argument("--binary-env", choices=["HC074_REDUNDANT_TEST_BINARY"])
    parser.add_argument("--case", choices=CASES)
    args = parser.parse_args()
    binary = (args.binary if args.binary is not None else Path(os.environ[args.binary_env])).resolve(strict=True)
    require(binary.name.startswith("hydracache_long_run_supervisor_074-") and os.access(binary, os.X_OK), "libtest required")
    require(os.getuid() != 0 and os.geteuid() == os.getuid(), "nonroot only")
    libc = ctypes.CDLL(None, use_errno=True)
    previous = ctypes.c_int()
    require(libc.prctl(37, ctypes.byref(previous), 0, 0, 0) == 0, "get subreaper")
    require(libc.prctl(36, 1, 0, 0, 0) == 0, "set subreaper")
    try:
        print("HC074-REDUNDANT image_sha256=" + hashlib.sha256(binary.read_bytes()).hexdigest(), flush=True)
        for case in ([args.case] if args.case else CASES):
            run(binary, case)
        print("HC074-REDUNDANT completed=true cases=" + str(1 if args.case else len(CASES)), flush=True)
    finally:
        try:
            cleanup(binary)
        finally:
            require(libc.prctl(36, previous.value, 0, 0, 0) == 0, "restore subreaper")


if __name__ == "__main__":
    main()
