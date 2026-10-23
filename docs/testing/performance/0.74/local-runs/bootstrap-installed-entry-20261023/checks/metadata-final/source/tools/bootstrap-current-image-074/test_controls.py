import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import check as c
import retain as r


class Controls(unittest.TestCase):
    def test_fixed_scopes_and_unknown_lane_refusal(self):
        for lane in c.LANES:
            for _, argv in c.commands(lane):
                for forbidden in ('--ignored', 'sudo', 'root', 'systemctl', 'ssh', 'unshare', 'campaign'):
                    self.assertNotIn(forbidden, argv)
        with self.assertRaises(ValueError): c.commands('live')

    def test_existing_packet_cannot_rerun_or_reseal(self):
        with tempfile.TemporaryDirectory() as tmp, patch.object(r, 'PACKET', Path(tmp)), patch.object(c.logger.subprocess, 'run') as run:
            with self.assertRaises(ValueError): r.run('unused', ['host', '194', '188', 'metadata'])
            run.assert_not_called()

    def test_failed_lane_cannot_be_accepted(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            c.publish(root / 'result.json', dict(lane='host', failure='first failure', steps=[], original_attempt=False, product=False, server=False))
            with self.assertRaises(ValueError): r.verify(root, 'host', {})


if __name__ == '__main__': unittest.main()
