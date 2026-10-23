import unittest
from unittest.mock import patch
import check as c
class Controls(unittest.TestCase):
    def test_scope_and_native_filters(self):
        for lane in c.LANES:
            for _, argv in c.commands(lane):
                for forbidden in ('--ignored', 'sudo', 'root', 'ssh', 'systemctl', 'unshare', 'campaign'):
                    self.assertNotIn(forbidden, argv)
        for lane in ('194', '188'):
            argv = dict(c.commands(lane))['held-entry']
            start = argv.index('diagnostic_held_bootstrap') + 1
            self.assertEqual(argv[start:start+2], ['held_entry_', '--'])
            self.assertEqual(argv.count('--test-threads=1'), 1)
            self.assertIn('image-cli', dict(c.commands(lane)))
    def test_unknown_lane_and_historical_baseline_refuse(self):
        for lane in ('live', 'server', 'retry'):
            with self.assertRaises(ValueError): c.commands(lane)
        with self.assertRaises(ValueError): c.run('baseline')
    def test_existing_lane_never_overwrites(self):
        import tempfile
        from pathlib import Path
        with tempfile.TemporaryDirectory() as tmp, patch.object(c.snapshot, 'OUTPUT', Path(tmp)), patch.object(c.snapshot, 'FILES', []), patch.object(c.snapshot, 'commands', lambda _: []):
            c.snapshot.run('fixture')
            with self.assertRaises(FileExistsError): c.snapshot.run('fixture')
if __name__ == '__main__': unittest.main()
