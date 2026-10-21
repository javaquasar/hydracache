"""Deterministic controls plus Linux subprocess containment (no real fixtures)."""
import copy
import importlib.util
import json
import os
from pathlib import Path
import signal
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("session_runner", Path(__file__).with_name("runner.py"))
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class SessionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.output = Path(self.temp.name) / "packet"
        self.pins = dict(source="a" * 40, bundle=[{"name": "synthetic"}], images={"194": "b", "188": "c"})
        self.context = dict(source="a" * 40, tracked_dirty="", uid=0, fixtures=[],
                            boot="aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee", bundle=self.pins["bundle"],
                            images=self.pins["images"], qualification_sha256=runner.QUALIFICATION,
                            pid=71, start_ticks="99", config={"wsl": "fixed"})
        self.calls = []

    def observe(self):
        return copy.deepcopy(self.context)

    def execute(self, lane, log, deadline):
        self.calls.append(lane)
        Path(log).write_bytes(b"synthetic unit control only\n")
        return dict(exit=0)

    def run_test(self, observe=None, execute=None, clock=lambda: 0):
        return runner.run_session(self.output, self.pins, observe or self.observe,
                                  execute or self.execute, clock)

    def test_six_serial_samples_fixed_order_and_postflight(self):
        result = self.run_test()
        self.assertEqual(self.calls, ["194"] * 3 + ["188"] * 3)
        self.assertTrue(result["schedule_completed"])
        self.assertEqual(result["measured_profile_invocations"], 6)
        self.assertEqual(runner.read_json(self.output / "metadata.json"), runner.read_json(self.output / "postflight.json"))
        self.assertFalse(result["native_gate_open"])

    def test_first_refusal_stops_only_lane(self):
        def execute(lane, log, deadline):
            row = self.execute(lane, log, deadline)
            row["exit"] = 101 if lane == "194" else 0
            return row
        result = self.run_test(execute=execute)
        self.assertEqual(self.calls, ["194", "188", "188", "188"])
        self.assertFalse((self.output / "194-Profile-1.log").exists())
        self.assertFalse(result["schedule_completed"])
        self.assertIsNone(result["failure"])

    def test_later_refusal_keeps_earlier_success(self):
        def execute(lane, log, deadline):
            row = self.execute(lane, log, deadline)
            row["exit"] = 101 if self.calls == ["194", "194"] else 0
            return row
        self.run_test(execute=execute)
        self.assertEqual(self.calls, ["194", "194", "188", "188", "188"])

    def test_existing_packet_never_resumed(self):
        self.run_test()
        with self.assertRaises(FileExistsError):
            self.run_test()
        self.assertEqual(len(self.calls), 6)

    def test_new_write_does_not_overwrite(self):
        path = Path(self.temp.name) / "once.json"
        runner.write_new(path, [1])
        with self.assertRaises(FileExistsError):
            runner.write_new(path, [2])
        self.assertEqual(runner.read_json(path), [1])

    def test_initial_metadata_refusals_start_no_children(self):
        for key, value in (("source", "c" * 40), ("tracked_dirty", "M runtime"),
                           ("uid", 1000), ("fixtures", ["owned-residue"]), ("boot", "invalid"),
                           ("bundle", []), ("images", {}), ("qualification_sha256", "changed")):
            with self.subTest(key=key):
                self.output = Path(self.temp.name) / key
                original = self.context[key]
                self.context[key] = value
                result = self.run_test()
                self.context[key] = original
                self.assertEqual(self.calls, [])
                self.assertFalse(result["schedule_completed"])
                self.assertIsNotNone(result["failure"])
                self.assertTrue((self.output / "postflight.json").exists())

    def test_live_metadata_drift_stops_all_lanes(self):
        for key, value in (("boot", "bbbbbbbb-bbbb-cccc-dddd-eeeeeeeeeeee"), ("pid", 72),
                           ("start_ticks", "100"), ("config", {}), ("images", {}), ("bundle", [])):
            with self.subTest(key=key):
                self.output = Path(self.temp.name) / key
                self.calls = []
                def observe():
                    result = self.observe()
                    if self.calls:
                        result[key] = value
                    return result
                result = self.run_test(observe=observe)
                self.assertEqual(self.calls, ["194"])
                self.assertIsNotNone(result["failure"])

    def test_exception_and_interruption_keep_intent_and_postflight(self):
        for exception in (RuntimeError("broken"), InterruptedError("lost"), KeyboardInterrupt()):
            self.output = Path(self.temp.name) / type(exception).__name__
            def execute(*_):
                raise exception
            result = self.run_test(execute=execute)
            self.assertEqual(result["measured_profile_invocations"], 1)
            self.assertEqual(result["profiles"], [])
            self.assertTrue((self.output / "194-Profile-0.intent.json").exists())
            self.assertTrue((self.output / "postflight.json").exists())
            self.assertIsNotNone(result["failure"])

    def test_postflight_failure_cannot_be_success(self):
        count = 0
        def observe():
            nonlocal count
            count += 1
            if count == 14:
                raise OSError("postflight lost")
            return self.observe()
        result = self.run_test(observe=observe)
        self.assertEqual(len(self.calls), 6)
        self.assertFalse(result["schedule_completed"])
        self.assertIn("Postflight", result["failure"])

    def test_preparation_refusal_keeps_postflight_without_profiles(self):
        def prepare():
            raise RuntimeError("Wrong default image")
        result = runner.run_session(self.output, self.pins, self.observe, self.execute, prepare=prepare)
        self.assertEqual(self.calls, [])
        self.assertIsNotNone(result["failure"])
        self.assertTrue((self.output / "postflight.json").exists())

    def test_predispatch_drift_never_starts_child(self):
        count = 0
        def observe():
            nonlocal count
            count += 1
            result = self.observe()
            if count > 1:
                result["config"] = {}
            return result
        result = self.run_test(observe=observe)
        self.assertEqual(self.calls, [])
        self.assertIsNotNone(result["failure"])

    def test_session_budget_never_extends_admission(self):
        calls = iter([0, runner.SESSION_SECONDS])
        result = self.run_test(clock=lambda: next(calls, runner.SESSION_SECONDS))
        self.assertEqual(self.calls, [])
        self.assertIn("budget", result["failure"])
        self.assertEqual(runner.read_json(self.output / "intent.json")["original_windows_seconds"], [300, 60, 10])

    def test_exact_command_no_send_or_product(self):
        self.assertEqual(runner.command("194")[0:4], ["/usr/bin/timeout", "--signal=TERM", "--kill-after=5s", "1200s"])
        self.assertEqual(runner.command("188")[-4:], ["privileged_consuming_held_live_matrix", "--ignored", "--nocapture", "--test-threads=1"])
        with patch.dict(os.environ, {"HC074_CONSUMING_HELD_LIVE_CASE": "bad", "RUST_TEST_THREADS": "99"}):
            env = runner.child_env(Path("/synthetic"))
        self.assertEqual(env["HC074_CONSUMING_HELD_LIVE_CASE"], "healthy")
        self.assertNotIn("RUST_TEST_THREADS", env)
        self.assertEqual(env["RUSTUP_AUTO_INSTALL"], "0")


class PreparationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.repo = Path(self.temp.name)
        self.prior = self.repo / "docs/testing/performance/0.74/local-runs" / runner.PRIOR
        self.prior.mkdir(parents=True)
        self.preparation = self.repo / "preparation.json"
        self.pins = dict(source="a"*40, bundle=[{"name": "synthetic"}],
                         images={l: "b"*64 for l in runner.LANES}, preparation=str(self.preparation))
        runner.write_new(self.prior / "inventory.json", dict(files=[]))
        runner.write_new(self.prior / "context-Verified-Before.json", dict(bundle=self.pins["bundle"]))
        fresh = {}
        for lane in runner.LANES:
            artifact = dict(reason="compiler-artifact", target=dict(name="hydracache_long_run_supervisor_074"),
                            profile=dict(test=True), features=[], executable=runner.IMAGES[lane])
            log = self.repo / f"build-{lane}.log"
            log.write_text(json.dumps(artifact)+"\n", encoding="utf-8")
            (self.prior / f"check-Verified-{lane}-build.log").write_bytes(log.read_bytes())
            runner.write_new(self.prior / f"check-Verified-{lane}-exit.json",
                             dict(source="a"*40, failure=None, steps=[dict(exit=0)]*11))
            fresh[lane] = dict(source="a"*40, exit=0, image=runner.IMAGES[lane], sha256="b"*64,
                               build_log=log.name, build_log_sha256=runner.sha(log))
        runner.write_new(self.preparation, fresh)
        actual_sha = runner.sha
        self.sha = patch.object(runner, "sha", side_effect=lambda p:
                               runner.PRIOR_HASH if Path(p).name == "inventory.json" else actual_sha(p))
        self.sha.start()
        self.addCleanup(self.sha.stop)
        self.git = patch.object(runner, "git", return_value="")
        self.git.start()
        self.addCleanup(self.git.stop)

    def test_original_default_images_and_checks(self):
        runner.verify_preparation(self.repo, self.pins)

    def test_source_bundle_and_lane_mutations_refused(self):
        for key, value in (("source", "not-a-source"), ("bundle", []), ("images", {})):
            pins = copy.deepcopy(self.pins)
            pins[key] = value
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                runner.verify_preparation(self.repo, pins)

    def test_changed_build_log_refused(self):
        with (self.repo / "build-194.log").open("a") as stream:
            stream.write("changed\n")
        with self.assertRaisesRegex(RuntimeError, "build log"):
            runner.verify_preparation(self.repo, self.pins)

    def test_allocation_feature_or_wrong_executable_refused(self):
        log = self.prior / "check-Verified-194-build.log"
        original = runner.read_json(log)
        for key, value in (("features", ["allocation-probe"]), ("executable", "/other/image")):
            artifact = copy.deepcopy(original)
            artifact[key] = value
            log.write_text(json.dumps(artifact)+"\n", encoding="utf-8")
            with self.subTest(key=key), self.assertRaises(RuntimeError):
                runner.verify_preparation(self.repo, self.pins)

    def test_incomplete_checks_refused(self):
        path = self.prior / "check-Verified-194-exit.json"
        receipt = runner.read_json(path)
        receipt["steps"][0]["exit"] = 1
        path.write_text(json.dumps(receipt), encoding="utf-8")
        with self.assertRaisesRegex(RuntimeError, "Focused checks"):
            runner.verify_preparation(self.repo, self.pins)


@unittest.skipUnless(sys.platform == "linux", "Linux process-group and selector controls")
class CaptureTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.log = Path(self.temp.name) / "child.log"

    def capture(self, code, seconds=5, max_bytes=1024):
        return runner.capture([sys.executable, "-c", code], {"PATH": "/usr/bin:/bin"},
                              self.log, time.monotonic() + seconds, max_bytes)

    def test_success_and_raw_output(self):
        self.assertEqual(self.capture("import sys; sys.stdout.buffer.write(b'raw\\x00\\n')")["exit"], 0)
        self.assertEqual(self.log.read_bytes(), b"raw\x00\n")

    def test_nonzero_is_not_retried(self):
        self.assertEqual(self.capture("raise SystemExit(7)")["exit"], 7)

    def test_timeout_stops_only_own_child(self):
        with self.assertRaisesRegex(RuntimeError, "timeout"):
            self.capture("import time; time.sleep(30)", seconds=0.2)

    def test_log_cap_keeps_partial_bytes(self):
        with self.assertRaisesRegex(RuntimeError, "log limit"):
            self.capture("print('x'*5000)", max_bytes=64)
        self.assertLessEqual(self.log.stat().st_size, 64)

    def test_missing_binary_leaves_new_log(self):
        with self.assertRaises(FileNotFoundError):
            runner.capture(["/no-such-synthetic-test"], {}, self.log, time.monotonic()+1)
        self.assertTrue(self.log.exists())

    def test_descendant_pipe_holder_is_bounded(self):
        code = "import os,time; child=os.fork(); time.sleep(30) if child==0 else None"
        with self.assertRaisesRegex(RuntimeError, "timeout"):
            self.capture(code, seconds=0.2)


if __name__ == "__main__":
    unittest.main(verbosity=2)
