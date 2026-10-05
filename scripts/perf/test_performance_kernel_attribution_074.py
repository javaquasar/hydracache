import importlib.util
from pathlib import Path
import tempfile
import unittest


MODULE_PATH = Path(__file__).with_name("performance_kernel_attribution_074.py")
SPEC = importlib.util.spec_from_file_location("performance_kernel_attribution_074", MODULE_PATH)
assert SPEC and SPEC.loader
kernel = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(kernel)


class KernelAttributionTests(unittest.TestCase):
    def test_schedule_is_complete_counterbalanced_and_stable(self) -> None:
        schedule = kernel.schedule_for(5)
        self.assertEqual(len(schedule), 30)
        for cell in kernel.CELLS:
            self.assertEqual(schedule.count(cell), 5)
        self.assertEqual(schedule, kernel.schedule_for(5))
        self.assertNotEqual(schedule[:6], schedule[6:12])

    def test_trace_parser_owns_tcp_and_starts_after_marker(self) -> None:
        trace = """\
100.000000 listen(9<TCP:[127.0.0.1:43123]>, 1024) = 0
100.010000 write(4</tmp/ready>, "w9c-ready-v1 pid=42\\n", 21) = 21
100.015000 listen(12<TCP:[127.0.0.1:43124]>, 1024) = 0
100.020000 write(10<TCP:[127.0.0.1:43124->127.0.0.1:51000]>, "x", 8) = 8
100.030000 read(11<TCP:[127.0.0.1:51000->127.0.0.1:43124]>, "x", 8) = 8
100.040000 write(10<TCP:[127.0.0.1:43124->127.0.0.1:51000]>, "x", 8) = -1 EAGAIN (Resource temporarily unavailable)
100.050000 epoll_wait(3<anon_inode:[eventpoll]>, [], 1024, 0) = 2
100.060000 futex(0x1, FUTEX_WAKE_PRIVATE, 1) = 1
"""
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "trace.42"
            path.write_text(trace, encoding="utf-8")
            result = kernel.parse_strace_files([path])
        self.assertEqual(result["server_ports"], [43124])
        self.assertEqual(result["tcp"]["server"]["write_calls"], 2)
        self.assertEqual(result["tcp"]["server"]["write_bytes"], 8)
        self.assertEqual(result["tcp"]["server"]["write_eagain"], 1)
        self.assertEqual(result["tcp"]["client"]["read_calls"], 1)
        self.assertEqual(result["epoll_calls"], 1)
        self.assertEqual(result["epoll_events"], 2)
        self.assertEqual(result["futex_calls"], 1)

    def test_trace_parser_fails_closed_without_unique_marker(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "trace.42"
            path.write_text("100.0 listen(9<TCP:[127.0.0.1:1]>, 1) = 0\n", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "trace-ready marker"):
                kernel.parse_strace_files([path])

    def test_ss_parser_retains_queues_and_skmem(self) -> None:
        rows = kernel.parse_ss_snapshot(
            "ESTAB 12 34 127.0.0.1:43123 127.0.0.1:51000\n"
            "\t skmem:(r1,rb2,t3,tb4,f5,w6,o7,bl8,d9) cubic\n"
        )
        self.assertEqual(rows[0]["recv_q_bytes"], 12)
        self.assertEqual(rows[0]["send_q_bytes"], 34)
        self.assertEqual(rows[0]["skmem"]["bl"], 8)

    def test_receipt_rejects_workload_drift(self) -> None:
        receipt = {
            "schema_version": 1,
            "release": "0.74",
            "profile_id": kernel.PROFILER_ID,
            "source_commit": "a" * 40,
            "promotable": False,
            "surface": "resp-api-loopback-tcp",
            "operation": "get",
            "operations": kernel.OPERATIONS,
            "warmup_operations": kernel.WARMUP_OPERATIONS,
            "concurrency": 1,
            "pipeline": 1,
            "batch_size": 1,
            "payload_bytes": kernel.PAYLOAD_BYTES,
            "key_space": kernel.KEY_SPACE,
            "seed": kernel.SEED,
            "instrumentation_enabled": True,
            "transport": "tcp",
            "exact_response_validation": True,
            "workload_sha256": "sha256:test",
            "resp": {"decoded_commands": kernel.OPERATIONS, "output_frames": kernel.OPERATIONS, "output_bytes": 5},
            "client_surface": {"dispatches": kernel.OPERATIONS},
            "socket_io": {"available": True, "written_bytes": 5},
        }
        kernel.validate_receipt(receipt, "a" * 40, "get", 1, 1)
        receipt["payload_bytes"] += 1
        with self.assertRaisesRegex(ValueError, "payload changed"):
            kernel.validate_receipt(receipt, "a" * 40, "get", 1, 1)


if __name__ == "__main__":
    unittest.main()
