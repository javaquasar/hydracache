"""Finite offline integrity controls; no captured command is executed."""
import copy
import json
from pathlib import Path
import tempfile
import unittest

import verify


class Integrity(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.index, cls.members = verify.verify(Path(__file__).resolve().parent)

    def reject(self, edit):
        index, members = copy.deepcopy(self.index), dict(self.members)
        edit(index, members)
        with self.assertRaises(ValueError):
            verify.check(index, members)

    def test_unchanged(self):
        verify.check(self.index, self.members)

    def test_index_pin(self):
        with tempfile.TemporaryDirectory(prefix='hc074-index-') as name:
            folder = Path(name)
            (folder / 'index.json').write_bytes(b'{}\n')
            with self.assertRaisesRegex(ValueError, 'fixed index'):
                verify.verify(folder)

    def test_source_receipt_drift(self):
        self.reject(lambda _, m: m.__setitem__(
            'source-final/tools/history-status-readers-074/inputs.rs', b'unknown'))

    def test_lost_case(self):
        key = 'checks/history-status-msrv-shared/stdout.log'
        self.reject(lambda _, m: m.__setitem__(key, m[key].replace(b'16 passed;', b'15 passed;', 1)))

    def test_changed_first_refusal(self):
        key = 'checks/history-status-clippy-final/result.json'

        def edit(_, members):
            result = json.loads(members[key])
            result['exit'] = 0
            members[key] = json.dumps(result).encode()
        self.reject(edit)

    def test_reused_pid_birth(self):
        def edit(_, members):
            result = json.loads(members['cleanup/result.json'])
            result['terminal_objects'][1]['creation_100ns'] += 1
            members['cleanup/result.json'] = json.dumps(result).encode()
        self.reject(edit)

    def test_diagnostic_promoted_to_pass(self):
        def edit(_, members):
            result = json.loads(members['cleanup/result.json'])
            result['original_test_pass'] = True
            members['cleanup/result.json'] = json.dumps(result).encode()
        self.reject(edit)


if __name__ == '__main__':
    unittest.main()
