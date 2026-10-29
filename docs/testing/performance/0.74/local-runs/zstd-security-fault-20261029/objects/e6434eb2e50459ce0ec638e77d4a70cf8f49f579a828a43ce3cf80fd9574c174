import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import check as c
import retain as r


class Controls(unittest.TestCase):
    def test_commands_do_not_dispatch_original_or_privileged_controls(self):
        for lane in ('host', '194', '188', 'metadata'):
            commands = c.commands(lane)
            for _, argv in commands:
                for token in ('--ignored', 'sudo', 'root', 'systemctl', 'ssh', 'unshare', 'original-live', 'campaign'):
                    self.assertNotIn(token, argv)
        with self.assertRaises(ValueError): c.commands('live')

    def test_existing_packet_refuses_before_commands(self):
        with tempfile.TemporaryDirectory() as tmp, patch.object(r, 'PACKET', Path(tmp)), patch.object(c.logger.subprocess, 'run') as run:
            with self.assertRaises(ValueError): r.run('unused')
            run.assert_not_called()

    def test_failed_or_changed_lane_cannot_be_promoted(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            result = dict(failure='first failure', lane='host', steps=[], server=False, product=False, original_attempt=False)
            c.publish(root / 'result.json', result)
            with self.assertRaises(ValueError): r.verify(root, 'host', {})


if __name__ == '__main__': unittest.main()
