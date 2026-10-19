"""Bounded cut carrier refusal controls; no cgroup or process launch."""
import array
import os
from pathlib import Path
import socket
import sys
from types import SimpleNamespace

sys.path.insert(0, str(Path(__file__).resolve().parents[6] / "tools/long-run-supervisor-074/tests"))
from root_creator_sigkill import packet, receive


def run():
    cases = ["valid", "magic", "reserved", "short", "truncated", "stage", "creator", "extra-right", "missing-right"]
    for case in cases:
        sender, receiver = socket.socketpair(socket.AF_UNIX, socket.SOCK_SEQPACKET | socket.SOCK_CLOEXEC)
        try:
            receiver.setsockopt(socket.SOL_SOCKET, socket.SO_PASSCRED, 1)
            raw = bytearray(packet(4))
            if case == "magic": raw[0] ^= 1
            if case == "reserved": raw[103] = 1
            if case == "short": raw = raw[:103]
            if case == "truncated": raw += b"x"
            if case == "stage": raw[8] = 6
            ancillary = []
            if case == "extra-right":
                ancillary = [(socket.SOL_SOCKET, socket.SCM_RIGHTS, array.array("i", [sender.fileno()]))]
            before = len(os.listdir("/proc/self/fd"))
            sender.sendmsg([raw], ancillary)
            owner = SimpleNamespace(pid=os.getpid() + (1 if case == "creator" else 0))
            rejected = False
            try:
                rights = receive(receiver, owner, 4, 1 if case == "missing-right" else 0)
                assert not rights
            except RuntimeError:
                rejected = True
            assert rejected == (case != "valid"), case
            assert len(os.listdir("/proc/self/fd")) == before, case
            print(f"HC074-CUT-CARRIER case={case} expected_refusal={rejected} installed_rights_closed=true")
        finally:
            sender.close()
            receiver.close()
    print("HC074-CUT-CARRIER completed=true cases=9")


if __name__ == "__main__":
    run()
