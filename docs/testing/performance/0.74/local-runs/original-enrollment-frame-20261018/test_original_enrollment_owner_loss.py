"""Fast parser/ancillary refusal controls; no external process is signalled."""
import json
import array
import os
from pathlib import Path
import socket
import unittest
from unittest.mock import Mock

import original_enrollment_owner_loss as driver


class CollectorPacketTests(unittest.TestCase):
    def packet(self, **changes):
        packet = dict(schema="hc074-enrollment-owner-test-v1", owner=123, roles=["collector", "custodian"],
                      cut="after-frame", deadline=100, origins=[1, 2, 3, 4, 5, 6], frame_closed=True)
        packet.update(changes)
        return json.dumps(packet).encode()

    def test_matched_packet(self):
        self.assertEqual(driver.decode(self.packet(), 123, "after-frame")["roles"], driver.ROLES)

    def test_unknown_extra_missing_and_duplicate_fields(self):
        for raw in [self.packet(schema="v2"), self.packet(extra=1), b"{}",
                    self.packet()[:-1] + b',"owner":123}']:
            with self.assertRaises(RuntimeError):
                driver.decode(raw, 123, "after-frame")

    def test_role_cut_owner_and_frame_binding(self):
        for changes in [dict(roles=["custodian", "collector"]), dict(roles=["collector", "collector"]),
                        dict(owner=124), dict(cut="before-frame"), dict(frame_closed=False)]:
            with self.assertRaises(RuntimeError):
                driver.decode(self.packet(**changes), 123, "after-frame")

    def test_bool_float_and_malformed_origins_refuse(self):
        for changes in [dict(owner=True), dict(deadline=True), dict(deadline=1.0), dict(deadline=0),
                        dict(frame_closed=1), dict(origins=[True]*6), dict(origins=[1]*5),
                        dict(origins=[1, 2, 3, 4, 5, 0])]:
            with self.assertRaises(RuntimeError):
                driver.decode(self.packet(**changes), 123, "after-frame")

    def test_before_frame_requires_false(self):
        raw = self.packet(cut="before-frame", frame_closed=False)
        self.assertEqual(driver.decode(raw, 123, "before-frame")["frame_closed"], False)

    def test_matched_ancillary(self):
        driver.ancillary_closed(socket.MSG_CMSG_CLOEXEC, [(123, os.getuid(), os.getgid())], [3, 4], 1, 123)

    def test_truncation_rights_and_credentials_refuse(self):
        creds = [(123, os.getuid(), os.getgid())]
        for flags, credentials, rights, headers in [
            (socket.MSG_CTRUNC, creds, [3, 4], 1), (socket.MSG_TRUNC, creds, [3, 4], 1),
            (0, [], [3, 4], 1), (0, creds*2, [3, 4], 1), (0, [(124, 0, 0)], [3, 4], 1),
            (0, creds, [3], 1), (0, creds, [3, 4, 5], 1), (0, creds, [3, 3], 1),
            (0, creds, [3, 4], 2)]:
            with self.assertRaises(RuntimeError):
                driver.ancillary_closed(flags, credentials, rights, headers, 123)

    def test_exact_fixture_only(self):
        self.assertEqual(driver.command(Path("/tmp/libtest")), ["/tmp/libtest", "--exact",
                         driver.PREFIX+"original_enrollment_owner_victim", "--ignored", "--nocapture", "--test-threads=1"])

    def test_receive_rejects_unknown_ancillary_and_closes_later_rights(self):
        fds = [os.open(os.devnull, os.O_RDONLY) for _ in range(2)]
        peer = Mock()
        peer.recvmsg.return_value = (self.packet(), [(socket.SOL_SOCKET, -1, b""),
            (socket.SOL_SOCKET, socket.SCM_RIGHTS, array.array("i", fds).tobytes())], 0, None)
        with self.assertRaisesRegex(RuntimeError, "ancillary kind"):
            driver.receive(peer, 123, "after-frame")
        for fd in fds:
            with self.assertRaises(OSError):
                os.fstat(fd)


if __name__ == "__main__":
    unittest.main()

