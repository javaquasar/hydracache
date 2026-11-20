"""Read only the W11 unit listing for the transient SSH observer."""

import os
from pathlib import Path
import re
import socket
import stat
import struct
import subprocess
import tempfile
import time

from .product_build_receipt_074 import require

ACCOUNT = "hydracache-w11-observer"
WORKER = "w11-build-observer-ssh-074.service"
BRIDGE = "w11-build-observer-status-074.service"
STATUS_ENV = "W11_OBSERVER_STATUS_SOCKET"
STATUS_COMMAND = (
    "/usr/bin/systemctl", "list-units", "--all", "--full", "--type=service",
    "--no-legend", "--no-pager", "hydracache-performance-074-*.service",
)
TIMEOUT = 5
MAX_LISTING = 64 * 1024


def protected_socket():
    value = os.environ.get(STATUS_ENV)
    require(value is not None and re.fullmatch(r"/run/w11-observer-status-[a-z0-9_]+/status\.sock", value),
            "missing protected observer status socket")
    path = Path(value)
    parent = path.parent.lstat()
    node = path.lstat()
    require(path.resolve(strict=True) == path and stat.S_ISDIR(parent.st_mode)
            and parent.st_uid == 0 and parent.st_mode & 0o022 == 0
            and node.st_uid == 0 and stat.S_ISSOCK(node.st_mode),
            "unexpected protected observer status socket")
    return path


def receive_exact(peer, length, deadline):
    result = bytearray()
    while len(result) < length:
        remaining = deadline - time.monotonic()
        require(remaining > 0, "observer status deadline expired")
        peer.settimeout(remaining)
        chunk = peer.recv(length - len(result))
        require(chunk, "observer status response was incomplete")
        result.extend(chunk)
    return result


def list_units():
    if STATUS_ENV not in os.environ:
        return subprocess.check_output(STATUS_COMMAND, text=True, timeout=TIMEOUT)
    path = protected_socket()
    deadline = time.monotonic() + TIMEOUT
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
        client.settimeout(TIMEOUT)
        client.connect(str(path))
        _, uid, _ = struct.unpack("3i", client.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
        require(uid == 0, "observer status peer was not the privileged launcher")
        (size,) = struct.unpack("!I", receive_exact(client, 4, deadline))
        require(size <= MAX_LISTING, "observer status listing exceeded its bound")
        return receive_exact(client, size, deadline).decode("utf-8")


def reply(peer):
    import pwd

    pid, uid, gid = struct.unpack("3i", peer.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
    try:
        account = pwd.getpwnam(ACCOUNT)
        scope = Path(f"/proc/{pid}/cgroup").read_text().strip()
    except (KeyError, OSError):
        return
    if not (61184 <= uid <= 65519 and uid == account.pw_uid and gid == account.pw_gid
            and scope == f"0::/system.slice/{WORKER}"):
        return
    raw = subprocess.check_output(STATUS_COMMAND, timeout=TIMEOUT)
    require(len(raw) <= MAX_LISTING, "observer status listing exceeded its bound")
    peer.settimeout(TIMEOUT)
    try:
        peer.sendall(struct.pack("!I", len(raw)) + raw)
    except (BrokenPipeError, ConnectionResetError):
        pass


def bridge():
    require(os.getuid() == 0, "the status bridge requires the trusted unit launcher")
    require(Path("/proc/self/cgroup").read_text().strip() == f"0::/system.slice/{BRIDGE}",
            "unexpected status bridge service")
    require(all(line.partition(":")[0] != ACCOUNT for line in Path("/etc/passwd").read_text().splitlines()),
            "observer identity must be transient")
    with tempfile.TemporaryDirectory(prefix="w11-observer-status-", dir="/run") as scratch:
        folder = Path(scratch)
        folder.chmod(0o711)
        address = folder / "status.sock"
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as listener:
            listener.bind(str(address))
            address.chmod(0o666)
            listener.listen(2)
            listener.settimeout(0.25)
            child = subprocess.Popen([
                "/usr/bin/systemd-run", "--quiet", "--wait", "--pipe", "--collect", f"--unit={WORKER}",
                "--property=Type=exec", "--property=DynamicUser=yes", f"--property=User={ACCOUNT}",
                f"--property=StateDirectory={ACCOUNT}", "--property=StateDirectoryMode=0700",
                "--property=WorkingDirectory=/opt/hydracache-w11-observer/tooling",
                "--property=CPUAffinity=0 1 2", "--property=RuntimeMaxSec=300", "--property=TimeoutStopSec=5",
                "--property=NoNewPrivileges=yes", "--property=CapabilityBoundingSet=",
                "--property=ProtectHome=yes", "--property=PrivateNetwork=yes",
                "--property=InaccessiblePaths=-/run/hydracache-perf", f"--property=BindsTo={BRIDGE}",
                f"--property=After={BRIDGE}", f"--setenv={STATUS_ENV}={address}", "--",
                "/bin/sh", "-c",
                'exec /usr/bin/env -i PATH=/usr/bin:/bin LANG=C.UTF-8 SSH_ORIGINAL_COMMAND=observe-v2 '
                '"STATE_DIRECTORY=${STATE_DIRECTORY:?}" "W11_OBSERVER_STATUS_SOCKET=${W11_OBSERVER_STATUS_SOCKET:?}" '
                '/usr/bin/python3 -B -m scripts.perf.w11_ssh_observer_074 serve',
            ])
            while child.poll() is None:
                try:
                    peer, _ = listener.accept()
                except TimeoutError:
                    continue
                with peer:
                    reply(peer)
            return child.returncode


if __name__ == "__main__":
    raise SystemExit(bridge())
