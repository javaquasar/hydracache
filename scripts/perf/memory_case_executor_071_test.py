#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
import socket
import sys
import tempfile
import threading
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("memory_case_executor_071.py")
SPEC = importlib.util.spec_from_file_location("memory_case_executor_071", SCRIPT)
assert SPEC and SPEC.loader
executor = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = executor
SPEC.loader.exec_module(executor)


class MemoryCaseExecutor071Tests(unittest.TestCase):
    def test_resp_round_trip_parser(self) -> None:
        client, server = socket.socketpair()

        def answer() -> None:
            request = server.recv(4096)
            self.assertIn(b"PING", request)
            server.sendall(b"+PONG\r\n")

        worker = threading.Thread(target=answer)
        worker.start()
        try:
            self.assertEqual(executor.resp_command(client, b"PING"), b"PONG")
        finally:
            client.close()
            server.close()
            worker.join()

    def test_percentiles_are_nearest_rank(self) -> None:
        values = list(range(1, 101))
        self.assertEqual(executor.percentile(values, 0.50), 50)
        self.assertEqual(executor.percentile(values, 0.95), 95)
        self.assertEqual(executor.percentile(values, 0.99), 99)

    def test_unavailable_is_explicit(self) -> None:
        self.assertEqual(
            executor.available(None, "not exposed"),
            {"value": None, "unavailable_reason": "not exposed"},
        )
        self.assertEqual(
            executor.available(7, "ignored"),
            {"value": 7, "unavailable_reason": None},
        )

    def test_binary_digest_is_prefixed_and_stable(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "binary"
            path.write_bytes(b"exact-binary")
            self.assertEqual(executor.sha256(path), executor.sha256(path))
            self.assertRegex(executor.sha256(path), r"^sha256:[0-9a-f]{64}$")


if __name__ == "__main__":
    unittest.main()
