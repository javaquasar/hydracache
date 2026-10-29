import copy
import json
from pathlib import Path
import tempfile
import unittest
from types import SimpleNamespace
from unittest.mock import patch

import owned_process_observer as subject


def row(terminal=False):
    return {"terminal": terminal, "creation_100ns": 100,
            "exit_100ns": 200 if terminal else None,
            "kernel_100ns": 3, "user_100ns": 4,
            "io": {name: 1 for name, _ in subject.IoCounters._fields_},
            "memory": None, "terminal_memory_error": 5 if terminal else None}


class Child:
    pid = 123

    def __init__(self, states=(None, None)):
        self.states = iter(states)

    def poll(self):
        return next(self.states)


class Api:
    def __init__(self, rows=None, identity=(123, "fixture.exe")):
        self.rows = iter(rows or [row(), row(True)])
        self.opened = []
        self.closed = []
        self.process_identity = identity

    def open(self, pid):
        self.opened.append(pid)
        return "original-handle"

    def identity(self, handle):
        assert handle == "original-handle"
        return self.process_identity

    def read(self, handle):
        assert handle == "original-handle"
        return copy.deepcopy(next(self.rows))

    def close(self, handle):
        self.closed.append(handle)


class ObserverTests(unittest.TestCase):
    def test_terminal_uses_same_handle_no_pid_reopen(self):
        api = Api()
        observer = subject.OwnedObserver(Child(), "fixture.exe", api)
        self.assertTrue(observer.sample()["terminal"])
        self.assertEqual(api.opened, [123])
        self.assertIsNone(observer.samples[-1]["memory"])
        self.assertEqual(observer.samples[-1]["terminal_memory_error"], 5)
        observer.close()
        observer.close()
        self.assertEqual(api.closed, ["original-handle"])

    def test_reject_exited_before_open(self):
        api = Api()
        with self.assertRaisesRegex(subject.Refusal, "already exited"):
            subject.OwnedObserver(Child((0,)), "fixture.exe", api)
        self.assertEqual(api.opened, [])

    def test_reject_exit_during_open_and_close(self):
        api = Api()
        with self.assertRaisesRegex(subject.Refusal, "during attachment"):
            subject.OwnedObserver(Child((None, 0)), "fixture.exe", api)
        self.assertEqual(api.closed, ["original-handle"])

    def test_reject_wrong_pid_or_image(self):
        for identity in ((456, "fixture.exe"), (123, "foreign.exe")):
            with self.subTest(identity=identity):
                api = Api(identity=identity)
                with self.assertRaisesRegex(subject.Refusal, "mismatch"):
                    subject.OwnedObserver(Child(), "fixture.exe", api)
                self.assertEqual(api.closed, ["original-handle"])

    def test_attachment_cleanup_error_does_not_replace_first_refusal(self):
        api = Api(identity=(456, "fixture.exe"))
        with patch.object(api, "close", side_effect=RuntimeError("close failed")):
            with self.assertRaisesRegex(subject.Refusal, "mismatch") as caught:
                subject.OwnedObserver(Child(), "fixture.exe", api)
        self.assertIn("close failed", caught.exception.__notes__[0])

    def test_read_error_latched_no_reopen(self):
        api = Api()
        observer = subject.OwnedObserver(Child(), "fixture.exe", api)
        with patch.object(api, "read", side_effect=subject.Refusal("read failed")):
            with self.assertRaisesRegex(subject.Refusal, "read failed"):
                observer.sample()
        with self.assertRaisesRegex(subject.Refusal, "already refused"):
            observer.sample()
        self.assertEqual(api.opened, [123])
        self.assertEqual(len(observer.samples), 1)
        observer.close()

    def test_sample_budget_and_first_refusal_latched(self):
        api = Api()
        observer = subject.OwnedObserver(Child(), "fixture.exe", api, max_samples=1)
        with self.assertRaisesRegex(subject.Refusal, "budget exhausted"):
            observer.sample()
        first = observer.refusal
        with self.assertRaisesRegex(subject.Refusal, "already refused"):
            observer.sample()
        self.assertEqual(observer.refusal, first)
        observer.close()

    def test_invalid_budgets_do_not_open(self):
        for budget in (0, 801, -1, True, 1.5):
            api = Api()
            with self.assertRaises(subject.Refusal):
                subject.OwnedObserver(Child(), "fixture.exe", api, budget)
            self.assertEqual(api.opened, [])

    def test_changed_creation_and_cpu_refuse(self):
        for field, value in (("creation_100ns", 101), ("kernel_100ns", 2), ("user_100ns", 3)):
            after = row(True)
            after[field] = value
            with self.subTest(field=field), self.assertRaises(subject.Refusal):
                subject.validate(row(), after)

    def test_every_io_counter_monotonic(self):
        for name, _ in subject.IoCounters._fields_:
            after = row(True)
            after["io"][name] = 0
            with self.subTest(name=name), self.assertRaisesRegex(subject.Refusal, "I/O counter"):
                subject.validate(row(), after)

    def test_live_exit_time_mask_and_terminal_identity(self):
        for after in (dict(row(), exit_100ns=999), dict(row(True), exit_100ns=99),
                      dict(row(), creation_100ns=0)):
            with self.assertRaises(subject.Refusal):
                subject.validate(None, after)
        with self.assertRaisesRegex(subject.Refusal, "after terminal"):
            subject.validate(row(True), row(True))

    def test_cpu_can_exceed_wall_no_fake_wait_subtraction(self):
        after = row(True)
        after["kernel_100ns"] = 100000
        subject.validate(row(), after)

    def test_closed_handle_refuses(self):
        observer = subject.OwnedObserver(Child(), "fixture.exe", Api())
        observer.close()
        with self.assertRaisesRegex(subject.Refusal, "closed"):
            observer.sample()

    def test_platform_refusal_retained_before_dispatch(self):
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / "first"
            with patch.object(subject, "WindowsApi", side_effect=subject.Refusal("platform")), \
                    patch.object(subject.subprocess, "Popen") as spawn:
                self.assertFalse(subject.fixture(output))
                spawn.assert_not_called()
            result = json.loads((output / "result.json").read_text())
            self.assertIn("platform", result["refusal"])
            self.assertTrue((output / "intent.json").is_file())
            with self.assertRaises(FileExistsError):
                subject.fixture(output)
            self.assertEqual(json.loads((output / "result.json").read_text()), result)

    def test_cleanup_error_retains_initial_fixture_refusal(self):
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / "first"
            child = SimpleNamespace(poll=lambda: None,
                                    kill=lambda: (_ for _ in ()).throw(RuntimeError("cleanup failed")))
            with patch.object(subject, "WindowsApi"), \
                    patch.object(subject.subprocess, "Popen", return_value=child), \
                    patch.object(subject, "OwnedObserver", side_effect=subject.Refusal("attach failed")):
                self.assertFalse(subject.fixture(output))
            result = json.loads((output / "result.json").read_text())
            self.assertEqual(result["refusal"], "Refusal: attach failed")
            self.assertEqual(result["cleanup_error"], "cleanup failed")

    def test_timeout_stops_only_owned_synthetic_child_and_retains_samples(self):
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / "first"
            child = unittest.mock.Mock()
            child.poll.return_value = None
            observer = SimpleNamespace(samples=[row()], close=unittest.mock.Mock())
            with patch.object(subject, "WindowsApi"), \
                    patch.object(subject.subprocess, "Popen", return_value=child), \
                    patch.object(subject, "OwnedObserver", return_value=observer), \
                    patch.object(subject.time, "monotonic", side_effect=[0, 11]):
                self.assertFalse(subject.fixture(output))
            child.kill.assert_called_once_with()
            child.wait.assert_called_once_with(timeout=5)
            observer.close.assert_called_once_with()
            result = json.loads((output / "result.json").read_text())
            self.assertIn("timeout", result["refusal"])
            self.assertEqual(len(result["samples"]), 1)

    def test_bad_status_and_streams_refuse_without_retry(self):
        for status, stdout in ((1, b"synthetic-child-complete\n"), (0, b"wrong\n")):
            with tempfile.TemporaryDirectory() as temp, self.subTest(status=status):
                output = Path(temp) / "first"
                child = unittest.mock.Mock()
                child.poll.return_value = status
                child.wait.return_value = status
                observer = SimpleNamespace(samples=[row(True)], pid=123,
                                           image="fixture.exe", close=lambda: None)

                def spawn(argv, **kwargs):
                    Path(argv[-1]).write_bytes(b"x" * 262144)
                    kwargs["stdout"].write(stdout)
                    return child

                with patch.object(subject, "WindowsApi"), \
                        patch.object(subject.subprocess, "Popen", side_effect=spawn) as start, \
                        patch.object(subject, "OwnedObserver", return_value=observer):
                    self.assertFalse(subject.fixture(output))
                start.assert_called_once()
                child.kill.assert_not_called()
                self.assertFalse(json.loads((output / "result.json").read_text())["passed"])


class Win32ReadTests(unittest.TestCase):
    def backend(self, status=258, memory_ok=True, io_ok=True):
        api = object.__new__(subject.WindowsApi)

        def times(handle, created, exited, kernel, user):
            for pointer, value in zip((created, exited, kernel, user), (100, 999, 3, 4)):
                pointer._obj.low = value
            return True

        api.dll = SimpleNamespace(
            WaitForSingleObject=lambda handle, timeout: status,
            GetProcessTimes=times,
            GetProcessIoCounters=lambda handle, io: io_ok,
            K32GetProcessMemoryInfo=lambda handle, memory, size: memory_ok)
        return api

    def test_live_exit_undefined_value_masked(self):
        self.assertIsNone(self.backend().read("held")["exit_100ns"])

    def test_terminal_exit_and_memory_failure_explicit(self):
        with patch.object(subject.ctypes, "get_last_error", return_value=5, create=True):
            result = self.backend(0, False).read("held")
        self.assertEqual(result["exit_100ns"], 999)
        self.assertIsNone(result["memory"])
        self.assertEqual(result["terminal_memory_error"], 5)

    def test_live_memory_failure_refuses(self):
        with patch.object(subject.ctypes, "get_last_error", return_value=5, create=True):
            with self.assertRaisesRegex(subject.Refusal, "live K32"):
                self.backend(memory_ok=False).read("held")

    def test_io_and_wait_failure_refuse(self):
        with patch.object(subject.ctypes, "get_last_error", return_value=5, create=True):
            with self.assertRaisesRegex(subject.Refusal, "GetProcessIoCounters"):
                self.backend(io_ok=False).read("held")
        with self.assertRaisesRegex(subject.Refusal, "WaitForSingleObject"):
            self.backend(status=0xffffffff).read("held")


if __name__ == "__main__":
    unittest.main()
