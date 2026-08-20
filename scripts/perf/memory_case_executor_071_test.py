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
    def workload(self, distribution: str, count: int, pool: int | str) -> object:
        return executor.Workload(
            None,
            {
                "case_id": "M5-tags",
                "dimensions": {
                    "distribution": distribution,
                    "tags_per_entry": count,
                    "tag_pool": pool,
                    "keys": 10,
                },
            },
            False,
        )

    def test_m5_distributions_are_deterministic_and_distinct(self) -> None:
        uniform = self.workload("uniform", 4, "private-per-entry")
        one_hot = self.workload("one-hot", 1, 1)
        high_fanout = self.workload("high-fanout", 16, 16)
        self.assertEqual(uniform.tags_for(3), uniform.tags_for(3))
        self.assertTrue(set(uniform.tags_for(3)).isdisjoint(uniform.tags_for(4)))
        self.assertEqual(one_hot.tags_for(3), one_hot.tags_for(4))
        self.assertEqual(len(high_fanout.tags_for(3)), 16)
        self.assertEqual(set(high_fanout.tags_for(3)), set(high_fanout.tags_for(4)))

    def test_m5_logical_accounting_uses_verified_ledger(self) -> None:
        workload = self.workload("high-fanout", 16, 16)
        workload.live.update({0, 1})
        workload.live_tags = {0: workload.tags_for(0), 1: workload.tags_for(1)}
        snapshot = executor.logical_snapshot({}, workload)
        self.assertEqual(snapshot["tag_records"], 32)
        self.assertEqual(snapshot["tag_bytes"], workload.tag_bytes())

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
