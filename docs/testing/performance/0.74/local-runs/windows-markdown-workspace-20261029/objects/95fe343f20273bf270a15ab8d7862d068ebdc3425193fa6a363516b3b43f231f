"""Inert exact-selection and full-catalog isolation controls; no executable runs."""
from pathlib import Path
import json
import tempfile
import threading
import unittest
from unittest import mock

import common as c
import isolated as i


class Isolated(unittest.TestCase):
    def test_selection_accounting_rejects_missing_duplicate_wrong_filtered_failed_or_foreign(self):
        good = (b'test alpha ... ok\n'
                b'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out;\n')
        self.assertEqual(i.account(good, 'alpha', 8)['passed'], 1)
        ignored = good.replace(b'alpha ... ok', b'alpha ... ignored, original manual case').replace(
            b'1 passed; 0 failed; 0 ignored', b'0 passed; 0 failed; 1 ignored')
        self.assertEqual(i.account(ignored, 'alpha', 8)['ignored'], 1)
        for raw in (b'', good+good, good.replace(b'7 filtered', b'6 filtered'),
                    good.replace(b'0 failed', b'1 failed'), good.replace(b'alpha', b'foreign')):
            with self.assertRaises(ValueError):
                i.account(raw, 'alpha', 8)

    def test_every_original_case_once_with_four_separate_commands(self):
        with tempfile.TemporaryDirectory(dir=c.ROOT/'target') as name:
            out = Path(name)
            path = out/'image-not-executed'
            path.write_bytes(b'synthetic data, never executed')
            image = dict(path=str(path), sha256=c.sha(path.read_bytes()))
            catalog = ['case'+str(v) for v in range(8)]
            calls = []
            barrier = threading.Barrier(4)
            lock = threading.Lock()
            def observed(folder, label, argv, env, deadline):
                self.assertEqual(argv[1], '--exact')
                self.assertEqual(argv[3], '--test-threads=1')
                with lock:
                    calls.append(argv[2])
                barrier.wait(timeout=5)
                child = folder/label
                child.mkdir()
                text = ('test '+argv[2]+' ... ok\n'
                        'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out;\n')
                (child/'stdout.log').write_bytes(text.encode())
                return child
            with mock.patch.object(i.b, 'invoke', side_effect=observed):
                result = i.cases(image, catalog, out/'cases', {}, 100)
            self.assertEqual(sorted(calls), catalog)
            self.assertEqual(result, dict(passed=8, ignored=0, catalog=8, global_omitted=0, concurrent_processes=4))
            self.assertEqual(len(list((out/'cases').glob('case-*/accounting.json'))), 8)


if __name__ == '__main__':
    unittest.main()
