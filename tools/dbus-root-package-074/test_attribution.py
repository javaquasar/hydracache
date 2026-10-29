"""Inert proc parsing and exact original-caller attribution controls."""
from pathlib import Path
import json
import tempfile
import unittest
from unittest import mock

import attribute_startup as a
import common as c


class Attribution(unittest.TestCase):
    def test_cpu_io_start_identity_and_candidate_children_from_fixture_only(self):
        with tempfile.TemporaryDirectory(dir=c.ROOT/'target') as name:
            root = Path(name)
            process = root/'12'
            (process/'task/12').mkdir(parents=True)
            fields = ['0']*50
            for at,value in ((0,'S'),(1,'10'),(11,'500'),(12,'7'),(19,'12345')):
                fields[at] = value
            (process/'stat').write_bytes(('12 (owned (fixture)) '+' '.join(fields)).encode())
            (process/'io').write_bytes(b'rchar: 131072\nsyscr: 3\n')
            (process/'cmdline').write_bytes(b'/tmp/owned-bootstrap\0--held\0')
            (process/'task/12/children').write_bytes(b'13 14\n')
            with mock.patch.object(a,'Path',return_value=root), \
                 mock.patch.object(a.os,'readlink',return_value='/tmp/owned-bootstrap'):
                value,children = a.proc(12)
            self.assertEqual((value['ppid'],value['start_ticks'],value['utime_ticks'],value['stime_ticks']),
                             (10,12345,500,7))
            self.assertEqual(value['io'],dict(rchar=131072,syscr=3))
            self.assertEqual(children,{13,14})
            self.assertEqual(value['argv'],['/tmp/owned-bootstrap','--held'])

    def test_nonqualifying_probe_preserves_outer_caller_and_actual_failure_exit(self):
        with tempfile.TemporaryDirectory(dir=c.ROOT/'target') as name:
            root = Path(name)
            (root/'freeze').mkdir()
            (root/'focus-194').mkdir()
            out = root/'out'
            out.mkdir()
            (root/'freeze/runtime-reviewed.json').write_bytes(b'{}')
            image = root/'not-executed'
            image.write_bytes(b'inert only')
            c.write(root/'focus-194','images.json',[
                dict(kind=['test'],name='bootstrap_current_image',path=str(image),sha256=c.sha(image.read_bytes()))])
            owner = mock.Mock()
            owner._owned.return_value = dict(exit=101,retired=True,errors=[])
            with mock.patch.object(c,'BASE',root),mock.patch.object(a,'OUT',out), \
                 mock.patch.object(c,'guard'),mock.patch.object(c,'load') as loader, \
                 mock.patch.object(c,'owned',return_value=owner), \
                 mock.patch.object(a.n,'environment',return_value=(None,{})), \
                 mock.patch.object(a.os,'environ'):
                with self.assertRaises(SystemExit) as stopped:
                    a.native_worker()
            self.assertEqual(stopped.exception.code,101)
            argv = owner._owned.call_args.args[1]
            self.assertEqual(argv,[str(image),'--exact',a.CASE,'--test-threads=1','--color=never'])
            self.assertEqual(owner._owned.call_args.kwargs['bound'],60)
            self.assertFalse(json.loads((out/'test-intent.json').read_bytes())['acceptance_pass'])
            self.assertEqual(json.loads((out/'linux-execution.json').read_bytes())['exit'],101)


if __name__ == '__main__':
    unittest.main()
