import unittest
from unittest.mock import patch
import check as c
class Controls(unittest.TestCase):
    def test_local_scope(self):
        for lane in c.LANES:
            for _, argv in c.commands(lane):
                for forbidden in ('--ignored', 'sudo', 'root', 'ssh', 'systemctl', 'unshare', 'campaign'):
                    self.assertNotIn(forbidden, argv)
    def test_unknown_lane(self):
        for lane in ('live', 'server', 'baseline'):
            with self.assertRaises(ValueError): c.commands(lane)
    def test_existing_lane_cannot_overwrite_first_results(self):
        import tempfile
        from pathlib import Path
        with tempfile.TemporaryDirectory() as tmp, patch.object(c.snapshot, 'OUTPUT', Path(tmp)), patch.object(c.snapshot, 'FILES', []), patch.object(c.snapshot, 'commands', lambda _: []):
            c.snapshot.run('fixture')
            with self.assertRaises(FileExistsError): c.snapshot.run('fixture')
if __name__ == '__main__': unittest.main()
