import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import check as c
import retain as r


class Controls(unittest.TestCase):
    def test_fixed_local_scopes_never_use_infrastructure_or_old_live_fixtures(self):
        for lane in c.LANES:
            for _, argv in c.commands(lane):
                for token in ('--ignored', 'sudo', 'root', 'ssh', 'systemctl', 'unshare', 'campaign'):
                    self.assertNotIn(token, argv)
        with self.assertRaises(ValueError): c.commands('live')

    def test_existing_packet_never_reexecutes_or_overwrites(self):
        with tempfile.TemporaryDirectory() as tmp, patch.object(r, 'PACKET', Path(tmp)), patch.object(r.subprocess, 'run') as run:
            with self.assertRaises(ValueError): r.run('unused', ['host', '194', '188', 'metadata'])
            run.assert_not_called()

    def test_failed_lane_cannot_be_accepted(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            c.publish(root / 'result.json', dict(lane='host', failure='first failure', steps=[], original_attempt=False, product=False, server=False))
            with self.assertRaises(ValueError): r.verify(root, 'host', {})


if __name__ == '__main__': unittest.main()
