import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import check as c
import retain as r


class Controls(unittest.TestCase):
    def test_only_local_role_assertion_controls(self):
        for lane in ('host', '194', '188', 'metadata'):
            for _, argv in c.commands(lane):
                for forbidden in ('--ignored', 'sudo', 'root', 'systemctl', 'ssh', 'unshare', 'campaign'):
                    self.assertNotIn(forbidden, argv)
        with self.assertRaises(ValueError): c.commands('live')

    def test_existing_packet_never_reexecutes(self):
        with tempfile.TemporaryDirectory() as tmp, patch.object(r, 'PACKET', Path(tmp)), patch.object(c.logger.subprocess, 'run') as run:
            with self.assertRaises(ValueError): r.run('unused')
            run.assert_not_called()

    def test_failed_lane_cannot_be_promoted(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            c.publish(root / 'result.json', dict(failure='first failure', lane='host', steps=[], server=False, product=False, original_attempt=False))
            with self.assertRaises(ValueError): r.verify(root, 'host', {})


if __name__ == '__main__': unittest.main()
