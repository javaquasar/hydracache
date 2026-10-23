import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import check as c
import retain as r


class Controls(unittest.TestCase):
    def test_fixed_local_commands_refuse_infrastructure_and_unknown_lanes(self):
        for lane in c.LANES:
            for _, argv in c.commands(lane):
                for token in ('--ignored', 'sudo', 'root', 'ssh', 'systemctl', 'unshare', 'campaign'):
                    self.assertNotIn(token, argv)
        with self.assertRaises(ValueError): c.commands('live')

    def test_historical_baseline_cannot_be_reexecuted(self):
        with self.assertRaises(ValueError): c.run('baseline')

    def test_existing_packet_never_overwrites_or_executes_checks(self):
        with tempfile.TemporaryDirectory() as tmp, patch.object(r, 'PACKET', Path(tmp)), patch.object(r.subprocess, 'run') as run:
            with self.assertRaises(ValueError): r.run('unused')
            run.assert_not_called()

    def test_failed_lane_cannot_be_accepted(self):
        with tempfile.TemporaryDirectory() as tmp:
            folder = Path(tmp)
            c.publish(folder / 'result.json', dict(lane='host', failure='first failure', steps=[], original_attempt=False, server=False, product=False))
            with self.assertRaises(ValueError): r.verify(folder, 'host', {})


if __name__ == '__main__': unittest.main()
