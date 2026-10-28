"""Owning local subreaper: original manager/root creator dies by real SIGKILL."""
import argparse
import ctypes
import fcntl
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import socket
import stat
import subprocess
import time

from original_custody_owner_loss import inherited, locked, parent_pid, pidfd_pid, require, terminal
from prearmed_watchdog import credentials, stopped

SCHEMA = "hc074-owned-root-creator-v1"
TEST = "privileged_original_root_manager_creator_victim"
CASES = ("owner", "first-dead", "second-dead", "first-stopped", "second-stopped", "outside-alias")
SEED = 0x074C2025


def shape(value, phase):
    common = {"schema", "phase", "root", "root_device", "root_inode", "fence", "scopes", "relay", "test_issuers", "original_send", "completion", "product"}
    require(set(value) == common | ({"origins", "bounds"} if phase == "ready" else set()), "capsule fields")
    require(value["schema"] == SCHEMA and value["phase"] == phase, "capsule version/phase")
    require(value["test_issuers"] is True and all(value[k] is False for k in ("original_send", "completion", "product")), "scope")
    require(type(value["root_device"]) is int and type(value["root_inode"]) is int and value["root_device"] > 0 and value["root_inode"] > 0, "root identity")
    if phase == "ready":
        for key in ("origins", "bounds"):
            require(type(value[key]) is list and len(value[key]) == 4 and all(type(x) is int and x > 0 for x in value[key]), "original bounds")
        require(value["bounds"][3] == min(value["bounds"][:3]), "first original deadline")
    return value


def receive(peer, owner, phase, count):
    raw, ancillary, flags, _ = peer.recvmsg(16384, 4096, socket.MSG_CMSG_CLOEXEC)
    rights, creds = credentials(ancillary)
    try:
        require(flags & ~socket.MSG_CMSG_CLOEXEC == 0 and 0 < len(raw) < 16384, "capsule truncation")
        require(creds == [(owner.pid, 0, 0)], "actual creator credentials")
        require(len(rights) == count and len(set(rights)) == count and all(not os.get_inheritable(fd) for fd in rights), "capsule rights")
        return shape(json.loads(raw), phase), rights
    except BaseException:
        for fd in rights:
            os.close(fd)
        raise


def scope(entry, relay=False):
    require(set(entry) == {"path", "device", "inode"}, "scope fields")
    path = Path(entry["path"])
    prefix = "hc074-controller-relay-" if relay else "hc074-owned-root-"
    require(path.parent == Path("/tmp") and path.name.startswith(prefix) and path.resolve(strict=True) == path, "exact temporary scope")
    info = path.lstat()
    require(stat.S_ISDIR(info.st_mode) and (info.st_uid, info.st_gid, info.st_mode & 0o7777) == (0, 0, 0o755 if relay else 0o700), "root temporary owner/mode")
    require((info.st_dev, info.st_ino) == (entry["device"], entry["inode"]), "temporary identity changed")
    return path


def validate_preparation(value, before):
    require(type(value["scopes"]) is list and len(value["scopes"]) == 6, "owned scope count")
    paths = [scope(item) for item in value["scopes"]]
    require(len(set(paths)) == len(paths), "duplicate owned scopes")
    scope(value["relay"], relay=True)
    root = Path(value["root"])
    require(root.parent == Path("/sys/fs/cgroup") and root.name.startswith("hc074-owned-root-") and root.name not in before, "fresh exact cgroup")
    require(root.name in {p.name for p in paths}, "cgroup has no owned temporary origin")
    info = root.lstat()
    require(stat.S_ISDIR(info.st_mode) and info.st_uid == info.st_gid == 0 and info.st_mode & 0o022 == 0, "cgroup owner")
    require((info.st_dev, info.st_ino) == (value["root_device"], value["root_inode"]), "cgroup identity")
    fence = Path(value["fence"])
    require(any(fence == p / "var/lib/hydracache-performance/campaigns/.host-execution.lock" for p in paths), "original fence route")
    locked(fence, True)
    return root, fence


def drain(fd):
    while True:
        try:
            if not os.read(fd, 4096):
                return True
        except BlockingIOError:
            return False


def kill_owned(fd):
    if not terminal(fd):
        signal.pidfd_send_signal(fd, signal.SIGKILL)
    try:
        os.waitid(os.P_PIDFD, fd, os.WEXITED)
    except ChildProcessError:
        pass  # the exact Popen creator may already have been reaped


def cleanup_adopted():
    # Single-purpose driver: all unreaped direct/adopted children are ours.
    for raw in Path(f"/proc/self/task/{os.getpid()}/children").read_text().split():
        pid = int(raw)
        require(parent_pid(pid) == os.getpid(), "cleanup parent changed")
        fd = os.pidfd_open(pid)  # unreaped owned child cannot recycle its PID
        try:
            kill_owned(fd)
        finally:
            os.close(fd)


def run(binary, out, case):
    before = {p.name for p in Path("/sys/fs/cgroup").iterdir() if p.name.startswith("hc074-owned-root-")}
    directory = out / case
    directory.mkdir()
    peer, child = socket.socketpair(socket.AF_UNIX, socket.SOCK_SEQPACKET | socket.SOCK_CLOEXEC)
    peer.setsockopt(socket.SOL_SOCKET, socket.SO_PASSCRED, 1)
    peer.settimeout(30)  # pre-admission harness bound, never a runtime extension
    owner = None
    owner_pin = None
    rights = []
    preparation = None
    root = None
    try:
        env = os.environ.copy()
        env.update(HC074_OWNED_ROOT_FIXTURE="local-only-v1", HC074_OWNED_CREATOR_PEER=str(child.fileno()), HC074_OWNED_CREATOR_CASE=case)
        with (directory / "creator.log").open("wb") as log:
            owner = subprocess.Popen([str(binary), TEST, "--ignored", "--nocapture", "--test-threads=1"],
                                     stdout=log, stderr=log, env=env, pass_fds=(child.fileno(),))
            owner_pin = os.pidfd_open(owner.pid)
            child.close()
            preparation, empty = receive(peer, owner, "preparing", 0)
            require(not empty, "unexpected preparation alias")
            (directory / "preparing.json").write_text(json.dumps(preparation, indent=2) + "\n")
            root, fence = validate_preparation(preparation, before)
            ready, rights = receive(peer, owner, "ready", 14 if case == "outside-alias" else 13)
            require({k: v for k, v in ready.items() if k not in {"phase", "origins", "bounds"}} ==
                    {k: v for k, v in preparation.items() if k != "phase"}, "original preparation changed")
            (directory / "ready.json").write_text(json.dumps(ready, indent=2) + "\n")
            for fd in rights[:2]:
                require(stat.S_ISFIFO(os.fstat(fd).st_mode) and fcntl.fcntl(fd, fcntl.F_GETFL) & os.O_ACCMODE == os.O_RDONLY, "read endpoint")
            require((os.fstat(rights[0]).st_dev, os.fstat(rights[0]).st_ino) != (os.fstat(rights[1]).st_dev, os.fstat(rights[1]).st_ino), "stdout/stderr alias")
            pids = [pidfd_pid(fd) for fd in rights[2:10]]
            require(len(set(pids)) == 8 and all(parent_pid(pid) == owner.pid for pid in pids), "exact original children")
            require(all(not terminal(fd) for fd in [owner_pin, *rights[2:10]]), "not live before cut")
            for pid, fd in zip(pids[5:8], rights[10:13]):
                info = os.fstat(fd)
                original = Path(f"/proc/{pid}/fd/0").stat()
                require(stat.S_ISFIFO(info.st_mode) and (info.st_dev, info.st_ino) == (original.st_dev, original.st_ino), "original dummy stdin")
                require(fcntl.fcntl(fd, fcntl.F_GETFL) & os.O_ACCMODE == os.O_WRONLY, "held dummy stdin writer")
            deadline = ready["bounds"][3]
            require(time.clock_gettime_ns(time.CLOCK_BOOTTIME) < deadline, "original deadline already expired")
            if case.startswith("first-") or case.startswith("second-"):
                index = 0 if case.startswith("first-") else 1
                sig = signal.SIGKILL if case.endswith("dead") else signal.SIGSTOP
                signal.pidfd_send_signal(rights[2 + index], sig)
                until = time.monotonic() + 2
                while not (terminal(rights[2 + index]) if sig == signal.SIGKILL else stopped(pids[index])):
                    require(time.monotonic() < until, "guardian fault not observed")
                    time.sleep(.001)
            signal.pidfd_send_signal(owner_pin, signal.SIGKILL)
            require(owner.wait(timeout=5) == -signal.SIGKILL, "creator did not die by SIGKILL")
            for fd in rights[7:10]:
                remaining = (deadline - time.clock_gettime_ns(time.CLOCK_BOOTTIME)) / 1e9
                require(remaining > 0 and terminal(fd, min(remaining, 5)), "writer not stopped before original expiry")
                status = os.waitid(os.P_PIDFD, fd, os.WEXITED)
                require(status.si_code == os.CLD_KILLED and status.si_status == signal.SIGKILL, "writer wait is not SIGKILL")
            require(time.clock_gettime_ns(time.CLOCK_BOOTTIME) < deadline, "expiry could explain revocation")
            require((root / "cgroup.events").read_bytes() == b"populated 0\nfrozen 0\n", "writers still populate root")
            until = time.monotonic() + 5
            while True:
                try:
                    inherited(pids[2], fence)
                    inherited(pids[3], fence)
                    break
                except RuntimeError:
                    require(time.monotonic() < until, "original sink holders did not shed writers")
                    time.sleep(.001)
            require(drain(rights[1]), "stderr not EOF")
            if case == "outside-alias":
                require(fcntl.fcntl(rights[13], fcntl.F_GETFL) & os.O_ACCMODE == os.O_WRONLY, "outside writer access")
                require(not drain(rights[0]), "outside alias incorrectly certified EOF")
                require(os.write(rights[13], b"outside") == 7, "outside alias cannot demonstrate remaining writing")
                locked(fence, True)
                os.close(rights.pop())
            require(drain(rights[0]), "stdout not EOF")
            locked(fence, True)
            print(f"HC074-OWNED-CREATOR case={case} seed=0x{SEED:08x} creator_sigkill=true writers_sigkill=3 before_original_deadline=true sinks_shed=true drained_eof=true fence_held=true test_issuers=true original_send=false completion=false product=false", flush=True)
    except BaseException:
        print((directory / "creator.log").read_text(errors="replace")[-16384:], flush=True)
        raise
    finally:
        if owner is not None and owner.poll() is None:
            signal.pidfd_send_signal(owner_pin, signal.SIGKILL)
            owner.wait(timeout=5)
        # Writers first. Only then may fixture-only keeper cleanup release locks.
        for fd in rights[7:10] + rights[4:7] + rights[2:4]:
            kill_owned(fd)
        for fd in rights:
            os.close(fd)
        if owner_pin is not None:
            os.close(owner_pin)
        if root is not None and root.exists():
            require(root.parent == Path("/sys/fs/cgroup") and root.name not in before, "cleanup cgroup scope")
            info = root.lstat()
            require((info.st_dev, info.st_ino) == (preparation["root_device"], preparation["root_inode"]), "cleanup cgroup replacement")
            # Failure cleanup only, not an EOF or completion receipt.
            (root / "cgroup.kill").write_text("1")
        cleanup_adopted()
        if root is not None and root.exists():
            (root / "nested").rmdir()
            root.rmdir()
        if preparation is not None:
            for entry, relay in [(x, False) for x in preparation["scopes"]] + [(preparation["relay"], True)]:
                path = Path(entry["path"])
                if path.exists():
                    scope(entry, relay)
                    shutil.rmtree(path)
        child.close()
        peer.close()


def self_test():
    value = dict(schema=SCHEMA, phase="ready", root="unused", root_device=1, root_inode=2,
                 fence="unused", scopes=[], relay={}, test_issuers=True, original_send=False,
                 completion=False, product=False, origins=[1, 2, 3, 4], bounds=[300, 60, 10, 10])
    shape(value, "ready")
    mutations = [dict(value, schema="future"), dict(value, completion=True), dict(value, product=True),
                 dict(value, original_send=True), dict(value, test_issuers=False), dict(value, bounds=[300, 60, 10, 60]),
                 dict(value, origins=[0, 2, 3, 4]), dict(value, extra=True)]
    for bad in mutations:
        try:
            shape(bad, "ready")
        except RuntimeError:
            continue
        raise RuntimeError("accepted capsule mutation")
    print("owned creator capsule mutation controls: 8 passed")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--binary-env")
    parser.add_argument("--out-env")
    parser.add_argument("--case", choices=CASES)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return
    if args.binary_env:
        require(not args.binary and args.binary_env in os.environ, "exact binary selector")
        args.binary = Path(os.environ[args.binary_env])
    if args.out_env:
        require(not args.out and args.out_env in os.environ, "exact evidence selector")
        args.out = Path(os.environ[args.out_env])
    require(os.getuid() == os.geteuid() == 0 and os.getgid() == 0, "local privileged fixture only")
    require(os.environ.get("HC074_OWNED_ROOT_FIXTURE") == "local-only-v1", "explicit fixture marker")
    require(args.binary and args.out and not args.out.exists(), "fresh evidence directory required")
    binary = args.binary.resolve(strict=True)
    args.out.mkdir(parents=True)
    libc = ctypes.CDLL(None, use_errno=True)
    require(not Path(f"/proc/self/task/{os.getpid()}/children").read_text().strip(), "driver already owns unrelated children")
    previous = ctypes.c_int()
    require(libc.prctl(37, ctypes.byref(previous), 0, 0, 0) == 0, "read subreaper")
    require(libc.prctl(36, 1, 0, 0, 0) == 0, "enable owning subreaper")
    digest = hashlib.sha256(binary.read_bytes()).hexdigest()
    try:
        print("HC074-OWNED-CREATOR image_sha256=" + digest, flush=True)
        selected = [args.case] if args.case else CASES
        for case in selected:
            run(binary, args.out, case)
        require(hashlib.sha256(binary.read_bytes()).hexdigest() == digest, "image drift")
        print(f"HC074-OWNED-CREATOR completed=true cases={len(selected)} server=false product=false", flush=True)
    finally:
        try:
            cleanup_adopted()
        finally:
            require(libc.prctl(36, previous.value, 0, 0, 0) == 0, "restore subreaper")


if __name__ == "__main__":
    main()
