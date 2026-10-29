import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import check as c
import retain as r
class Retention(unittest.TestCase):
    def test_existing_packet_cannot_be_resealed(self):
        with tempfile.TemporaryDirectory() as tmp, patch.object(r,'PACKET',Path(tmp)), patch.object(r.subprocess,'run') as run:
            with self.assertRaises(ValueError): r.run('0'*40,['host','194','188','metadata'])
            run.assert_not_called()
    def test_failed_lane_cannot_be_promoted(self):
        with tempfile.TemporaryDirectory() as tmp:
            folder=Path(tmp)
            c.publish(folder/'result.json',dict(lane='host',failure='first refusal',steps=[],original_attempt=False,server=False,product=False))
            c.publish(folder/'source-before.json',{})
            with self.assertRaises(ValueError): r.verify(folder,'host',{})
    def test_exact_commit_and_distinct_lane_kinds_required(self):
        with tempfile.TemporaryDirectory() as tmp, patch.object(r,'PACKET',Path(tmp)/'new'):
            with self.assertRaises(ValueError): r.run('HEAD',['host','194','188','metadata'])
            with self.assertRaises(ValueError): r.run('0'*40,['host']*4)
if __name__=='__main__': unittest.main()
