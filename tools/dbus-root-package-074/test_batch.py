"""Inert complete-catalog accounting and per-case refusal controls; no execution."""
from pathlib import Path
import json
import tempfile
import unittest
from unittest import mock

import batch as b
import common as c
import native as n


class Batch(unittest.TestCase):
    def test_expiry_prevents_even_creation_of_a_case_output(self):
        with tempfile.TemporaryDirectory(dir=c.ROOT/'target') as name:
            out = Path(name)
            with mock.patch.object(b.time, 'monotonic', return_value=2), \
                 mock.patch.object(b.subprocess, 'Popen') as spawn:
                with self.assertRaises(ValueError):
                    b.invoke(out, 'never', ['never'], {}, 1)
                spawn.assert_not_called()
                self.assertFalse((out/'never').exists())

    def test_full_catalog_and_zero_case_target_remain_unfiltered(self):
        with tempfile.TemporaryDirectory(dir=c.ROOT/'target') as name:
            out = Path(name)
            build = out/'builds/194'
            build.mkdir(parents=True)
            images = []
            for kind in ('lib', 'bin'):
                path = build/kind
                path.write_bytes(b'not executed: synthetic '+kind.encode())
                images.append(dict(path=str(path), sha256=c.sha(path.read_bytes()), kind=[kind], name=kind))
            c.write(out, 'images.json', images)
            selected = 'original::dispatch_negotiated_private_bus_fault_matrix'
            catalog = sorted(['case_'+str(i) for i in range(1592)]+[selected])
            calls = []
            def synthetic(folder, label, argv, env, deadline):
                calls.append(argv)
                child = folder/label
                child.mkdir()
                if '--ignored' in argv:
                    cases = ['bare-receiver','healthy','atomic-adapter','fragmented','lost-ack',
                             'foreign-sender','wrong-serial','disconnect','manager-error','no-capability']
                    text = ''.join('HC074-NEGOTIATED case='+v+' result=ok synthetic\n' for v in cases)
                    text += 'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1592 filtered out;\n'
                elif '--list' in argv:
                    text = ''.join(v+': test\n' for v in catalog) if argv[0] == str(build/'lib') else ''
                elif argv[0] == str(build/'lib'):
                    text = ''.join('test '+v+' ... '+('ignored' if v == selected else 'ok')+'\n' for v in catalog)
                    text += 'test result: ok. 1592 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out;\n'
                else:
                    text = 'test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;\n'
                (child/'stdout.log').write_bytes(text.encode())
                return child
            with mock.patch.object(n, 'NATIVE', out), mock.patch.object(n, 'environment', return_value=(None, {})), \
                 mock.patch.object(b, 'invoke', side_effect=synthetic):
                b.run('194', out, 100)
            self.assertEqual(json.loads((out/'totals.json').read_bytes()),
                dict(passed=1592, ignored=1, catalog=1593, targets=2))
            self.assertEqual(len(calls), 5)
            self.assertEqual(calls[1][-1], '--test-threads=1')
            self.assertEqual(calls[3][-1], '--test-threads=1')
            self.assertEqual(calls[4][1:], ['--ignored','--exact',selected,'--show-output'])


if __name__ == '__main__':
    unittest.main()
