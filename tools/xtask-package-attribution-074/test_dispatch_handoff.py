import ctypes
from pathlib import Path
from types import SimpleNamespace
import tempfile
import unittest
from unittest.mock import Mock, patch

from dispatch_handoff import DispatchOnce, ExitApi, ObservationCustody
from owned_process_observer import Refusal


def custody():
    owner = Mock()
    owner.poll.return_value = None
    root = SimpleNamespace(handle=1, pid=1, samples=[dict(creation_100ns=100)], close=Mock())
    monitor = Mock(owner=owner, root=root, first_refusal=None, owner_exit_observed=False, outer_expired=False, chain=None)
    state = dict(first_refusal=None, observations_complete=False, owner_exit_observed=False)
    monitor.result.side_effect = lambda: dict(state)

    def refused(reason):
        if monitor.first_refusal is None:
            monitor.first_refusal = str(reason)
            state["first_refusal"] = str(reason)

    monitor.refuse.side_effect = refused
    api = SimpleNamespace(exit_witness=Mock(return_value=dict(pid=1, creation_100ns=100, exit_100ns=200)))
    return ObservationCustody(owner, root, monitor, api)


class CustodyTests(unittest.TestCase):
    def test_late_exit_after_outer_handoff_does_not_resume_telemetry(self):
        c = custody()
        c.monitor.first_refusal = "outer bound"
        c.monitor.outer_expired = True
        c.tick(1200)
        c.tick(1201)
        self.assertIsNone(c.exit_proof)
        c.owner.poll.return_value = 0
        c.tick(1202)
        self.assertEqual(c.monitor.tick.call_count, 1)
        self.assertEqual(c.first_refusal, "outer bound")
        self.assertIsNotNone(c.exit_proof)
        c.finalize_observation()
        c.root.close.assert_called_once()
        self.assertFalse(c.result()["all_writers_proven_stopped"])
        c.owner.kill.assert_not_called()

    def test_live_owner_cannot_finalize(self):
        c = custody()
        c.freeze("telemetry failure")
        c.tick(1)
        with self.assertRaises(Refusal):
            c.finalize_observation()
        c.api.exit_witness.assert_not_called()
        c.root.close.assert_not_called()

    def test_live_held_descendant_keeps_query_custody(self):
        c = custody()
        c.monitor.chain = SimpleNamespace(handles=[2], identities=[dict(pid=1, creation_100ns=100),
            dict(pid=2, creation_100ns=101)], close=Mock())
        c.freeze("sample failed")
        c.owner.poll.return_value = 0
        c.api.exit_witness.side_effect = [dict(pid=1, creation_100ns=100, exit_100ns=200), None]
        c.tick(1)
        self.assertIsNone(c.exit_proof)
        with self.assertRaises(Refusal):
            c.finalize_observation()
        c.api.exit_witness.side_effect = [dict(pid=1, creation_100ns=100, exit_100ns=200),
                                         dict(pid=2, creation_100ns=101, exit_100ns=201)]
        c.tick(2)
        c.finalize_observation()
        c.monitor.chain.close.assert_called_once()

    def test_wrong_pid_birth_or_time_latches_no_witness_retry(self):
        for witness in (dict(pid=2, creation_100ns=100, exit_100ns=200),
                        dict(pid=1, creation_100ns=101, exit_100ns=200),
                        dict(pid=1, creation_100ns=100, exit_100ns=99)):
            c = custody()
            c.freeze("first")
            c.owner.poll.return_value = 0
            c.api.exit_witness.return_value = witness
            c.tick(1)
            with self.assertRaises(Refusal):
                c.tick(2)
            c.api.exit_witness.assert_called_once()
            self.assertEqual(c.first_refusal, "first")

    def test_exit_api_error_and_cleanup_error_never_retry_to_pass(self):
        c = custody()
        c.freeze("first")
        c.owner.poll.return_value = 0
        c.api.exit_witness.side_effect = Refusal("kernel query")
        c.tick(1)
        self.assertEqual(c.custody_refusal, "kernel query")
        c = custody()
        c.freeze("first")
        c.owner.poll.return_value = 0
        c.tick(1)
        c.root.close.side_effect = RuntimeError("close refused")
        with self.assertRaises(Refusal):
            c.finalize_observation()
        with self.assertRaises(Refusal):
            c.finalize_observation()
        c.root.close.assert_called_once()

    def test_poll_and_tick_exceptions_are_separate_first_refusals(self):
        c = custody()
        c.monitor.tick.side_effect = Refusal("observer tick")
        c.owner.poll.side_effect = OSError("owner status")
        c.tick(1)
        self.assertEqual(c.first_refusal, "observer tick")
        self.assertEqual(c.custody_refusal, "owner status")

    def test_original_root_required_and_snapshots_not_mutable(self):
        c = custody()
        with self.assertRaises(Refusal):
            ObservationCustody(Mock(), c.root, c.monitor, c.api)
        c.freeze("first")
        snapshot = c.result()
        snapshot["observation"]["first_refusal"] = "changed"
        self.assertEqual(c.result()["observation"]["first_refusal"], "first")


class DispatchTests(unittest.TestCase):
    def test_lost_ack_retains_same_owner_and_marker_blocks_restart(self):
        with tempfile.TemporaryDirectory() as temp:
            c = custody()
            launch = DispatchOnce("fixed-attempt", temp)
            spawn, attach = Mock(return_value=c.owner), Mock(return_value=c)
            result = launch.start(spawn, attach, Mock(side_effect=OSError("lost ACK")))
            self.assertFalse(result["acknowledged"])
            self.assertIs(launch.owner, c.owner)
            self.assertIn("lost ACK", c.first_refusal)
            with self.assertRaises(Refusal):
                launch.start(spawn, attach, Mock())
            again = DispatchOnce("fixed-attempt", temp)
            again.start(spawn, attach, Mock())
            self.assertIsNone(again.owner)
            spawn.assert_called_once()
            self.assertTrue((Path(temp) / "owner-attached.json").exists())

    def test_spawn_or_attach_error_consumes_attempt_without_second_spawn(self):
        for at_spawn in (True, False):
            with tempfile.TemporaryDirectory() as temp:
                launch = DispatchOnce("fixed-attempt", temp)
                spawn = Mock(side_effect=OSError("spawn") if at_spawn else None, return_value=object())
                attach = Mock(side_effect=OSError("attach"))
                result = launch.start(spawn, attach, Mock())
                self.assertEqual(result["owner_object_retained"], not at_spawn)
                with self.assertRaises(Refusal):
                    launch.start(spawn, attach, Mock())
                spawn.assert_called_once()

    def test_success_ack_once_and_invalid_id(self):
        with tempfile.TemporaryDirectory() as temp:
            c = custody()
            ack = Mock()
            result = DispatchOnce("fixed-attempt", temp).start(Mock(return_value=c.owner), Mock(return_value=c), ack)
            self.assertTrue(result["acknowledged"])
            ack.assert_called_once_with("fixed-attempt")
        with self.assertRaises(Refusal):
            DispatchOnce("../escape", "unused")

    def test_fsync_failure_before_spawn_and_publish_failure_after_attach(self):
        with tempfile.TemporaryDirectory() as temp:
            spawn = Mock()
            first = DispatchOnce("fixed-attempt", temp)
            with patch("dispatch_handoff.os.fsync", side_effect=OSError("fsync refused")):
                first.start(spawn, Mock(), Mock())
            again = DispatchOnce("fixed-attempt", temp)
            again.start(spawn, Mock(), Mock())
            spawn.assert_not_called()
        with tempfile.TemporaryDirectory() as temp:
            c = custody()
            first = DispatchOnce("fixed-attempt", temp)
            original = first.publish

            def publish(name, value):
                if name == "owner-attached.json":
                    raise OSError("attachment receipt lost")
                original(name, value)

            with patch.object(first, "publish", side_effect=publish):
                first.start(Mock(return_value=c.owner), Mock(return_value=c), Mock())
            self.assertIs(first.owner, c.owner)
            self.assertIn("attachment receipt lost", c.first_refusal)


class ExitApiTests(unittest.TestCase):
    def api(self, status=0, times=True):
        api = object.__new__(ExitApi)

        def get_times(handle, created, exited, kernel, user):
            created._obj.low, exited._obj.low = 100, 200
            return times

        api.dll = SimpleNamespace(WaitForSingleObject=Mock(return_value=status), GetProcessId=Mock(return_value=1),
                                  GetProcessTimes=Mock(side_effect=get_times))
        return api

    def test_terminal_identity_only_and_live_no_times(self):
        self.assertEqual(self.api().exit_witness("original"), dict(pid=1, creation_100ns=100, exit_100ns=200))
        api = self.api(258)
        self.assertIsNone(api.exit_witness("original"))
        api.dll.GetProcessTimes.assert_not_called()

    def test_wait_or_times_failure_refuses(self):
        with self.assertRaises(Refusal):
            self.api(0xffffffff).exit_witness("original")
        with patch.object(ctypes, "get_last_error", return_value=5, create=True):
            with self.assertRaises(Refusal):
                self.api(times=False).exit_witness("original")


if __name__ == "__main__":
    unittest.main()
