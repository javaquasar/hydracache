import json
from pathlib import Path
import unittest
from unittest.mock import patch

import package_result_v9 as result


class FirstV9Result(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        folder = result.ROOT / result.PACKET
        cls.index = (folder / 'index.json').read_bytes()
        cls.objects = {p.name: p.read_bytes() for p in (folder / 'objects').iterdir()}
        cls.members = result.load_packet(cls.index, cls.objects.__getitem__)
        cls.parser = staticmethod(result.current_parser())

    def test_first_timeout_is_integral_but_not_a_pass(self):
        self.assertEqual(result.verify()['failed'], 0)
        self.assertFalse(result.verify()['package_pass'])

    def test_index_drift_refuses_before_object_read(self):
        with self.assertRaisesRegex(ValueError, 'index drift'):
            result.load_packet(self.index + b' ', lambda _: self.fail('unexpected object read'))

    def test_missing_and_changed_objects_refuse(self):
        with self.assertRaises(KeyError):
            result.load_packet(self.index, {}.__getitem__)
        first = next(iter(json.loads(self.index)['members'].values()))['sha256']
        with self.assertRaisesRegex(ValueError, 'object drift'):
            result.load_packet(self.index, lambda key: b'changed' if key == first else self.objects[key])

    def changed(self, name, edit):
        members = self.members.copy()
        body = json.loads(members[name])
        edit(body)
        members[name] = json.dumps(body).encode()
        return members

    def test_unchanged_bound_and_no_retry_cannot_be_relabelled(self):
        for edit in (lambda d: d.update(bound_seconds=781),
                     lambda d: d.update(retry=True),
                     lambda d: d['argv'].append('filter')):
            with self.subTest(edit=edit), self.assertRaisesRegex(ValueError, 'command/window/retry'):
                result.assess(self.changed('package/intent.json', edit), self.parser)

    def test_source_environment_and_accounting_drift_refuse(self):
        for name, edit in (
                ('package/result.json', lambda d: d.update(source_unchanged=False)),
                ('derived/postflight.json', lambda d: d.update(environment={})),
                ('derived/accounting.json', lambda d: d.update(package_pass=True))):
            with self.subTest(name=name), self.assertRaises(ValueError):
                result.assess(self.changed(name, edit), self.parser)

    def test_archived_parser_never_imported_and_current_drift_refuses_before_import(self):
        with patch.object(Path, 'read_bytes', return_value=b'changed'), \
                patch.object(result.importlib.util, 'spec_from_file_location',
                             side_effect=AssertionError('must check before import')):
            with self.assertRaisesRegex(ValueError, 'current transcript parser drift'):
                result.current_parser()
        # assess reads only named data; the archived Python member is inert.
        members = self.members.copy()
        members['parser/review.py'] = b'raise AssertionError("not executable evidence")'
        self.assertFalse(result.assess(members, self.parser)['package_pass'])


if __name__ == '__main__':
    unittest.main()
