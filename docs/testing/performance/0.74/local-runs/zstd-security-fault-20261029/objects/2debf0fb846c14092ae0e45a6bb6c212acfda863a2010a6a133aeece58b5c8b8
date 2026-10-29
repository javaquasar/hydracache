from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

from observation_coordinator import Coordinator
from owned_process_observer import Refusal


class CoordinatorTests(unittest.TestCase):
    def monitor(self):
        owner = Mock()
        owner.poll.return_value = None
        root = SimpleNamespace(pid=1, sample=Mock())
        api = SimpleNamespace(snapshot=Mock(return_value=[]))
        return Coordinator(owner, root, api, [(r"C:\fixture\leaf.exe",)])

    def chain(self, terminal=False):
        return SimpleNamespace(samples=[dict(terminal=terminal)], pids=(1, 2),
                               identities=[dict(pid=1), dict(pid=2)],
                               sample=Mock(return_value=dict(terminal=True)), close=Mock())

    def test_absence_is_waiting_until_original_discovery_origin(self):
        c = self.monitor()
        with patch("observation_coordinator.candidates", return_value=[]):
            c.tick(0)
            c.tick(29)
            self.assertIsNone(c.first_refusal)
            c.tick(30)
        self.assertIn("discovery exhausted", c.first_refusal)
        self.assertEqual(c.api.snapshot.call_count, 2)

    def test_no_retry_or_kill_after_snapshot_failure_owner_still_drains(self):
        c = self.monitor()
        c.api.snapshot.side_effect = Refusal("first census failure")
        c.tick(0)
        c.tick(1)
        self.assertEqual(c.api.snapshot.call_count, 1)
        self.assertEqual(c.first_refusal, "first census failure")
        self.assertFalse(c.result()["owner_exit_observed"])
        c.owner.poll.return_value = 0
        c.tick(2)
        c.owner.kill.assert_not_called()
        c.owner.terminate.assert_not_called()
        self.assertTrue(c.result()["owner_exit_observed"])
        self.assertFalse(c.result()["observations_complete"])
        c.close()

    def test_single_attachment_failure_never_reacquired(self):
        c = self.monitor()
        with patch("observation_coordinator.candidates", return_value=["found"]), \
                patch("observation_coordinator.HeldLineage", side_effect=Refusal("attach failed")) as attach:
            c.tick(0)
            c.tick(1)
            attach.assert_called_once()
        self.assertEqual(c.api.snapshot.call_count, 1)

    def test_held_leaf_terminal_sample_once_then_owner_verdict_separate(self):
        c, chain = self.monitor(), self.chain()
        with patch("observation_coordinator.candidates", return_value=["found"]), \
                patch("observation_coordinator.HeldLineage", return_value=chain):
            c.tick(0)
            c.tick(0.5)
            chain.sample.assert_not_called()
            c.tick(1)
            c.tick(2)
            chain.sample.assert_called_once()
            c.owner.poll.return_value = 1
            c.tick(3)
        self.assertTrue(c.result()["observations_complete"])
        self.assertEqual(c.result()["owner_returncode"], 1)
        self.assertFalse(c.result()["selected_target_pass"])
        self.assertEqual(c.api.snapshot.call_count, 1)
        c.close()
        c.close()
        chain.close.assert_called_once()

    def test_terminal_on_first_attachment_does_not_sample_again(self):
        c, chain = self.monitor(), self.chain(True)
        with patch("observation_coordinator.candidates", return_value=["found"]), \
                patch("observation_coordinator.HeldLineage", return_value=chain):
            c.tick(0)
            c.owner.poll.return_value = 0
            c.tick(1)
        chain.sample.assert_not_called()
        self.assertTrue(c.leaf_terminal)

    def test_owner_exit_before_leaf_or_live_leaf_at_exit_refuses(self):
        c = self.monitor()
        c.owner.poll.return_value = 0
        c.tick(0)
        self.assertIn("before leaf attachment", c.first_refusal)
        c = self.monitor()
        c.chain = self.chain()
        c.chain.sample.return_value = dict(terminal=False)
        c.owner.poll.return_value = 0
        c.tick(0)
        self.assertIn("still live", c.first_refusal)

    def test_sampling_error_latched_and_no_future_sampling(self):
        c = self.monitor()
        c.chain = self.chain()
        c.chain.sample.side_effect = Refusal("sample failure")
        c.next_sample = 0
        c.tick(0)
        c.tick(1)
        c.chain.sample.assert_called_once()
        self.assertEqual(c.first_refusal, "sample failure")

    def test_snapshot_budget_without_reset(self):
        c = self.monitor()
        with patch("observation_coordinator.candidates", return_value=[]):
            for _ in range(301):
                c.tick(0)
        self.assertEqual(c.discovery_snapshots, 300)
        self.assertIn("discovery exhausted", c.first_refusal)

    def test_outer_expiry_retains_live_custody_and_blocks_close(self):
        c = self.monitor()
        c.tick(1200)
        self.assertTrue(c.result()["outer_expired"])
        self.assertTrue(c.result()["live_owner_custody_retained"])
        with self.assertRaisesRegex(Refusal, "retain custody"):
            c.close()
        c.owner.kill.assert_not_called()
        with self.assertRaises(Refusal):
            c.tick(1201)

    def test_clock_faults_first_reason_and_no_post_completion_ticks(self):
        for bad in (True, float("nan"), float("inf"), -1, "0"):
            c = self.monitor()
            with self.subTest(bad=bad), self.assertRaises(Refusal):
                c.tick(bad)
        c = self.monitor()
        c.refuse("original")
        c.tick(1)
        with self.assertRaisesRegex(Refusal, "original"):
            c.tick(0)
        c.owner.poll.return_value = 0
        c.tick(2)
        with self.assertRaises(Refusal):
            c.tick(3)

    def test_missed_slots_not_burst_replayed_and_root_error_retained(self):
        c = self.monitor()
        c.chain = self.chain()
        c.chain.sample.return_value = dict(terminal=False)
        c.next_sample = 1
        c.tick(5.5)
        self.assertEqual(c.next_sample, 6)
        c.chain.sample.assert_called_once()
        c.chain.sample.return_value = dict(terminal=True)
        c.root.sample.side_effect = Refusal("terminal root query")
        c.owner.poll.return_value = 0
        c.tick(6)
        self.assertEqual(c.first_refusal, "terminal root query")


if __name__ == "__main__":
    unittest.main()
