"""Manual two-child barred faults; only the supplied libtest image is executed."""
import argparse
import array
import ctypes
import hashlib
import json
import os
from pathlib import Path
import signal
import socket
import struct
import subprocess
import tempfile
import time

import original_custody_owner_loss as base

PREFIX = base.PREFIX.replace("barred_custody_tests::loss_tests::",
                             "barred_custody_tests::collector_tests::enrollment_tests::faults::")
SEED = 0x0740E418
ROLES = ["collector", "custodian"]


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        base.require(key not in result, "duplicate packet field")
        result[key] = value
    return result


def decode(raw, owner, cut):
    packet = json.loads(raw, object_pairs_hook=unique_object)
    base.require(type(packet) is dict and set(packet) == {
        "schema", "owner", "roles", "cut", "deadline", "origins", "frame_closed"}, "packet fields")
    base.require(packet["schema"] == "hc074-enrollment-owner-test-v1" and type(packet["owner"]) is int
                 and packet["owner"] == owner and packet["cut"] == cut, "packet binding")
    base.require(packet["roles"] == ROLES, "pidfd role order")
    base.require(type(packet["frame_closed"]) is bool and
                 packet["frame_closed"] == (cut == "after-frame"), "frame cut")
    base.require(type(packet["origins"]) is list and len(packet["origins"]) == 6 and
                 all(type(n) is int and n > 0 for n in packet["origins"]), "origin shape")
    base.require(type(packet["deadline"]) is int and packet["deadline"] > 0, "deadline shape")
    return packet


def ancillary_closed(flags, credentials, received, rights_headers, owner):
    base.require(flags & ~socket.MSG_CMSG_CLOEXEC == 0, "truncated ancillary data")
    base.require(credentials == [(owner, os.getuid(), os.getgid())], "sender credentials")
    base.require(rights_headers == 1 and len(received) == 2 and len(set(received)) == 2,
                 "ambiguous pidfd rights")


def receive(peer, owner, cut):
    received = []
    try:
        raw, ancillary, flags, _ = peer.recvmsg(2048, socket.CMSG_SPACE(8) + socket.CMSG_SPACE(12),
                                              socket.MSG_CMSG_CLOEXEC)
        credentials = []
        headers = 0
        # Gather ALL installed rights before validating other ancillary fields,
        # so a preceding unknown field cannot leak a later received descriptor.
        for level, kind, data in ancillary:
            if level == socket.SOL_SOCKET and kind == socket.SCM_RIGHTS:
                headers += 1
                rights = array.array("i")
                rights.frombytes(data)
                received.extend(rights)
        for level, kind, data in ancillary:
            base.require(level == socket.SOL_SOCKET, "ancillary level")
            if kind == socket.SCM_CREDENTIALS:
                base.require(len(data) == 12, "credential length")
                credentials.append(struct.unpack("3i", data))
            elif kind != socket.SCM_RIGHTS:
                raise RuntimeError("ancillary kind")
        ancillary_closed(flags, credentials, received, headers, owner)
        base.require(all(not os.get_inheritable(fd) for fd in received), "inheritable pidfd")
        return decode(raw, owner, cut), received
    except BaseException:
        for fd in received:
            os.close(fd)
        raise


def command(binary):
    return [str(binary), "--exact", PREFIX + "original_enrollment_owner_victim", "--ignored",
            "--nocapture", "--test-threads=1"]


def original_collector(pid, lock, origin):
    targets = [os.readlink(path) for path in Path(f"/proc/{pid}/fd").iterdir()]
    base.require(len(targets) == 7 and str(lock) in targets and
                 "anon_inode:[timerfd]" in targets, "collector inheritance")
    base.require(sum(p.startswith("pipe:[") for p in targets) == 2 and
                 sum(p.startswith("socket:[") for p in targets) == 1, "collector roles")
    for name in ["collector-stdout.bin", "collector-stderr.bin"]:
        paths = list(origin.rglob(name))
        base.require(len(paths) == 1 and str(paths[0]) in targets and paths[0].stat().st_size == 0,
                     "original empty sink")


def quarantined(pid, fd, lock):
    # A bounded post-owner-failure observation, not an authority deadline.
    limit = time.monotonic() + 5
    while True:
        base.require(not base.terminal(fd), "collector died instead of retaining custody")
        if len(list(Path(f"/proc/{pid}/fd").iterdir())) == 3:
            base.inherited(pid, lock)
            return
        base.require(time.monotonic() < limit, "original I/O not quarantined")
        time.sleep(0.001)


def cleanup_adopted(binary):
    # Failure-only cleanup of this driver's unreaped children. A terminal child
    # may be reaped without /proc/exe; no signal is sent to an unverified image.
    for raw in Path(f"/proc/self/task/{os.getpid()}/children").read_text().split():
        pid = int(raw)
        base.require(base.parent_pid(pid) == os.getpid(), "cleanup child parent")
        fd = os.pidfd_open(pid)
        try:
            if not base.terminal(fd):
                base.require(os.path.samefile(f"/proc/{pid}/exe", binary), "cleanup executable")
                signal.pidfd_send_signal(fd, signal.SIGKILL)
                base.require(base.terminal(fd, 10), "cleanup child termination")
            os.waitid(os.P_PIDFD, fd, os.WEXITED)
        finally:
            os.close(fd)


def run_case(binary, cut, order, repeat):
    with tempfile.TemporaryDirectory(prefix="hc074-enrollment-loss-", dir="/tmp") as name, \
            tempfile.TemporaryDirectory(prefix="hydracache-authority-", dir="/tmp") as authority:
        root = Path(name)
        for part in ["context", "origin"]:
            (root / part).mkdir(mode=0o700)
        env = os.environ.copy()
        env.update(HC074_PRIVATE_ENROLLMENT_LOSS_ROOT=name, HC074_PRIVATE_ENROLLMENT_LOSS_CUT=cut,
                   HC074_PRIVATE_ENROLLMENT_LOSS_AUTHORITY=authority, TMPDIR=str(root / "context"))
        with socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM | socket.SOCK_CLOEXEC) as peer, \
                (root / "victim.log").open("wb") as log:
            peer.setsockopt(socket.SOL_SOCKET, socket.SO_PASSCRED, 1)
            peer.bind(str(root / "notice.sock"))
            peer.settimeout(60)
            child = subprocess.Popen(command(binary), stdin=subprocess.PIPE, stdout=log, stderr=log, env=env)
            owner = None
            received = []
            try:
                # Test-only pin of our exact still-unreaped Popen child.
                owner = os.pidfd_open(child.pid)
                packet, received = receive(peer, child.pid, cut)
                deadline = packet["deadline"]
                base.require(time.clock_gettime_ns(time.CLOCK_BOOTTIME) < deadline, "checkpoint expired")
                pids = [base.pidfd_pid(fd) for fd in received]
                base.require(len(set(pids)) == 2, "duplicate child identity")
                for pid in pids:
                    base.require(base.parent_pid(pid) == child.pid, "not the fixture's direct child")
                    base.require(os.path.samefile(f"/proc/{pid}/exe", binary), "child executable")
                campaigns = root / "origin/var/lib/hydracache-performance/campaigns"
                locks = list(campaigns.iterdir())
                base.require(len(locks) == 1 and locks[0].is_file(), "campaign directory")
                lock = locks[0]
                original_collector(pids[0], lock, root / "origin")
                base.inherited(pids[1], lock)
                before = base.snapshot(root / "origin")
                authority_before = base.snapshot(Path(authority))
                base.locked(lock, True)
                base.require(not base.terminal(owner) and all(not base.terminal(fd) for fd in received),
                             "checkpoint process terminal")
                print("HC074-ENROLLMENT-LOSS-CHECKPOINT " + json.dumps(
                    dict(packet, collector=pids[0], custodian=pids[1], order=order, repeat=repeat),
                    sort_keys=True), flush=True)
                signal.pidfd_send_signal(owner, signal.SIGKILL)
                base.require(child.wait(timeout=10) == -signal.SIGKILL, "owner not killed")
                base.require(all(base.parent_pid(pid) == os.getpid() for pid in pids), "children not adopted")
                quarantined(pids[0], received[0], lock)
                base.inherited(pids[1], lock)
                base.locked(lock, True)
                indices = [1, 0] if order == "custodian-first" else [0, 1]
                first, last = indices
                signal.pidfd_send_signal(received[first], signal.SIGKILL)
                base.wait_killed(received[first])
                base.require(not base.terminal(received[last]), "survivor died with sibling")
                base.inherited(pids[last], lock)
                base.locked(lock, True)
                remaining = (deadline - time.clock_gettime_ns(time.CLOCK_BOOTTIME)) / 1e9
                if remaining > 0:
                    base.require(not base.terminal(received[last], remaining), "survivor died before original bound")
                base.require(time.clock_gettime_ns(time.CLOCK_BOOTTIME) >= deadline, "original bound not reached")
                base.locked(lock, True)
                base.require(base.snapshot(root / "origin") == before, "records changed after owner death")
                base.require(base.snapshot(Path(authority)) == authority_before, "authority changed after owner death")
                signal.pidfd_send_signal(received[last], signal.SIGKILL)
                base.wait_killed(received[last])
                base.locked(lock, False)
                env["HC074_PRIVATE_ENROLLMENT_LOSS_CUT"] = "inspect"
                replay = subprocess.run(command(binary), env=env, capture_output=True, timeout=60, check=True)
                base.require(b"HC074-ENROLLMENT-LOSS-REPLAY refused=true" in replay.stdout, "history admitted")
                base.require(base.snapshot(root / "origin") == before, "replay changed records")
                base.require(base.snapshot(Path(authority)) == authority_before, "authority changed after replay")
                print(f"HC074-ENROLLMENT-LOSS-OWNER seed=0x{SEED:08x} repeat={repeat} cut={cut} order={order} "
                      "owner_signal=9 first_signal=9 last_signal=9 io_closed=4 survivor_fds=3 "
                      "retained_past_original_bound=true unchanged=true replay_refused=true start=false", flush=True)
            except BaseException:
                log.flush()
                print((root / "victim.log").read_text(errors="replace"), flush=True)
                raise
            finally:
                # Do not signal received rights until credential/role/image
                # validation. Cleanup finds only our adopted, unreaped children.
                if owner is not None:
                    if child.returncode is None:
                        if not base.terminal(owner):
                            signal.pidfd_send_signal(owner, signal.SIGKILL)
                        child.wait(timeout=10)
                    os.close(owner)
                for fd in received:
                    os.close(fd)
                cleanup_adopted(binary)
                child.stdin.close()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--repeats", type=int, choices=[1, 3], default=3)
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    base.require(os.getuid() != 0 and os.geteuid() == os.getuid(), "nonroot only")
    base.require(binary.is_file() and os.access(binary, os.X_OK), "libtest executable required")
    libc = ctypes.CDLL(None, use_errno=True)
    previous = ctypes.c_int()
    base.require(libc.prctl(37, ctypes.byref(previous), 0, 0, 0) == 0, "get subreaper")
    base.require(libc.prctl(36, 1, 0, 0, 0) == 0, "set subreaper")
    try:
        print(f"HC074-ENROLLMENT-LOSS-DRIVER binary_sha256={hashlib.sha256(binary.read_bytes()).hexdigest()}", flush=True)
        for repeat in range(1, args.repeats + 1):
            for cut in ["before-frame", "after-frame"]:
                for order in ["custodian-first", "collector-first"]:
                    run_case(binary, cut, order, repeat)
        print(f"HC074-ENROLLMENT-LOSS-DRIVER completed=true cases={4 * args.repeats} product=false", flush=True)
    finally:
        try:
            cleanup_adopted(binary)
        finally:
            base.require(libc.prctl(36, previous.value, 0, 0, 0) == 0, "restore subreaper")


if __name__ == "__main__":
    main()
