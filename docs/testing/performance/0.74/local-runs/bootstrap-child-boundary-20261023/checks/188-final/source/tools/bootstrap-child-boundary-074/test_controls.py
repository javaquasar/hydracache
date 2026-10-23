import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import check as c
import retain as r


class Controls(unittest.TestCase):
    def test_only_fixed_static_models_and_no_infrastructure(self):
        for lane in c.LANES:
            for _, argv in c.commands(lane):
                for token in ('--ignored', 'sudo', 'root', 'ssh', 'systemctl', 'unshare', 'campaign'):
                    self.assertNotIn(token, argv)
        with self.assertRaises(ValueError): c.commands('live')

    def test_existing_packet_is_never_overwritten_or_executed(self):
        with tempfile.TemporaryDirectory() as tmp, patch.object(r, 'PACKET', Path(tmp)), patch.object(r.subprocess, 'run') as run:
            with self.assertRaises(ValueError): r.run('unused')
            run.assert_not_called()

    def test_explicit_failures_are_not_boolean_passes(self):
        for bad in (False, None, 0, ''):
            with self.assertRaises(ValueError): r.need(bad, 'refused')
        r.need(True, 'accepted')


if __name__ == '__main__': unittest.main()
