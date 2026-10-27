import copy
import json
from pathlib import Path
import tempfile
import unittest

import analysis as a


class OfflineControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data = a.inputs()

    def test_real_first_result_stays_incomplete(self):
        report = a.analyze(*self.data)
        self.assertEqual(len(report['selected_loop_contracts']), 7)
        self.assertEqual(report['selected_loop_contracts'][-1]['static_loop_members'], 466)
        self.assertIsNone(report['selected_loop_contracts'][-1]['verdict'])
        self.assertEqual([r['verdict'] for r in report['selected_loop_contracts'][:-1]], ['ok'] * 6)
        for key in ('measured_cpu_attribution', 'measured_git_or_filesystem_attribution',
                    'summed_case_times', 'tests_dispatched', 'input_code_executed',
                    'windows_package_pass', 'release_admitted', 'new_hypothesis'):
            self.assertFalse(report[key])

    def test_accounting_cannot_promote_or_fill_missing_coverage(self):
        for key, value in [('package_pass', True), ('complete', True), ('passed', 491), ('completed_targets', 153)]:
            data = copy.deepcopy(self.data)
            data[0][key] = value
            with self.assertRaisesRegex(ValueError, 'accounting drift'):
                a.analyze(*data)

    def test_body_and_missing_source_drift_refuse(self):
        for mode in ('body', 'missing'):
            data = copy.deepcopy(self.data)
            name = a.PREFIX + a.LOOPS[0][0] + '.rs'
            if mode == 'body':
                data[3][name] += b'\n// change'
            else:
                del data[3][name]
            with self.assertRaises(ValueError):
                a.analyze(*data)

    def test_syntax_scope_and_census_cannot_change(self):
        for edit in (lambda d: d[2].update(compiler_expansion=True),
                     lambda d: d[2]['sources'].pop(),
                     lambda d: d[1]['sources'].pop(next(iter(d[1]['sources']))),
                     lambda d: d[1]['targets'].pop()):
            data = copy.deepcopy(self.data)
            edit(data)
            with self.assertRaises(ValueError):
                a.analyze(*data)

    def test_object_identity_precedes_read_and_never_executes(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            objects = root / 'packet/objects'
            objects.mkdir(parents=True)
            raw = b'raise RuntimeError("inert archive")'
            digest = a.sha(raw)
            (objects / digest).write_bytes(raw)
            doc = {'members': {'input': {'sha256': digest, 'bytes': len(raw)}}}
            self.assertEqual(a.member(root, 'packet', doc, 'input', digest), raw)
            doc['members']['input']['sha256'] = '../escape'
            with self.assertRaisesRegex(ValueError, 'unsafe object'):
                a.member(root, 'packet', doc, 'input')

    def test_index_and_duplicate_json_refuse(self):
        with self.assertRaisesRegex(ValueError, 'duplicate JSON'):
            a.decode(b'{"same":1,"same":2}')
        with self.assertRaisesRegex(ValueError, 'nonfinite'):
            a.decode(b'{"x":NaN}')
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / 'p').mkdir()
            (root / 'p/index.json').write_bytes(b'{}')
            with self.assertRaisesRegex(ValueError, 'index drift'):
                a.packet(root, 'p', '0' * 64)


if __name__ == '__main__':
    unittest.main()
