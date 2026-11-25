"""Join the isolated process proof, effective limits and batch observations."""

import os
from pathlib import Path
import socket
import sys

from . import performance_signed_role_probe_074 as probe
from . import w11_batch_boundary_074 as batch
from . import w11_config_readback_074 as readback
from . import w11_product_build_receipt_074 as receipt


def namespace_identity(pid):
    info = Path(f"/proc/{pid}/ns/net").stat()
    return info.st_dev, info.st_ino


def listen_inode(port):
    found = []
    for name in ("tcp", "tcp6"):
        with Path(f"/proc/thread-self/net/{name}").open() as file:
            probe.require(bool(file.readline()), "missing TCP table header")
            for row in file:
                fields = row.split()
                probe.require(len(fields) >= 10, "incomplete TCP row")
                address, number = fields[1].split(":")
                if fields[3] != "0A" or int(number, 16) != port:
                    continue
                probe.require(name == "tcp" and len(address) == 8 and
                              int(address, 16).to_bytes(4, sys.byteorder) == socket.inet_aton("127.0.0.1"),
                              "non-loopback or ambiguous batch listener")
                found.append(int(fields[9]))
    probe.require(len(found) == 1 and found[0] > 0, "expected a single owned batch listener")
    return found[0]


def holds_socket(pid, inode):
    expected = f"socket:[{inode}]"
    for path in Path(f"/proc/{pid}/fd").iterdir():
        try:
            if os.readlink(path) == expected:
                return True
        except FileNotFoundError:
            continue
    return False


def observe_listener(observe, expected_namespace, address):
    before = observe()
    host, port = address
    probe.require(host == "127.0.0.1" and 0 < port <= 65535, "invalid batch proof endpoint")
    pid = before["pid"]
    probe.require(pid != os.getpid() and namespace_identity(pid) == expected_namespace and
                  namespace_identity("thread-self") == expected_namespace,
                  "batch listener escaped the admitted namespace")
    inode = listen_inode(port)
    probe.require(holds_socket(pid, inode) and not holds_socket("self", inode),
                  "batch listener is not held solely by the original daemon in the process pair")
    probe.require(observe() == before and namespace_identity(pid) == expected_namespace,
                  "daemon or namespace changed while reading the listener")
    return {"network_namespace_inode": expected_namespace[1], "socket_inode": inode,
            "daemon_pid": pid, "port": port}


class OriginalSession(batch.RespSession):
    def __init__(self, session, check_original):
        super().__init__(session.material, session.address, session.token, session.deadline, session.version)
        self.check_original = check_original

    def verify_authentication(self, output):
        self.check_original()
        super().verify_authentication(output)
        self.check_original()

    def issue(self, command):
        self.check_original()
        reply = super().issue(command)
        self.check_original()
        return reply


def collect_generation(subject, child, image, expected_boot, expected_namespace,
                       channel, ready, selector, session, output):
    """Complete one profile in its already admitted isolated process."""
    with readback.observe_child(child, image, subject["binary"], expected_boot) as observe:
        limits = readback.read_configuration(channel, observe, selector, ready, session.deadline)
        listener = observe_listener(observe, expected_namespace, session.address)

        def check_original():
            probe.seconds_left(session.deadline)
            probe.require(observe_listener(observe, expected_namespace, session.address) == listener,
                          "batch listener changed during the proof")

        bound = OriginalSession(session, check_original)
        result = batch.collect_profile(subject, selector, limits.configuration, observe, bound, output)
        check_original()
        probe.require(result["before"] == limits.generation == result["after"],
                      "configuration and batch cases have different process generations")
        batch.write_bytes(output / "effective-config.json", limits.raw)
        batch.write_bytes(output / "ready.json", ready)
        batch.write_bytes(output / "listener.json", receipt.canonical(listener) + b"\n")
        probe.seconds_left(session.deadline)
        return result
