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
        with tempfile.TemporaryDirectory(prefix='hc074-budget-') as name:
            folder = Path(name)
            (folder / 'index.json').write_bytes(b'{}')
            with self.assertRaisesRegex(ValueError, 'fixed index'):
                verify.verify(folder)

    def test_deadline_widening(self):
        self.reject(lambda i, _: i.__setitem__('cleanup_bound', 6))

    def test_runtime_drift(self):
        self.reject(lambda _, m: m.__setitem__('source/scripts/ci/run-with-heartbeat.py', b'unknown'))

    def test_first_refusal_erased(self):
        self.reject(lambda _, m: m.__setitem__('checks/ci-watchdog-budget-red-first/stderr.log', b'OK'))

    def test_test_source_changed_after_red(self):
        self.reject(lambda _, m: m.__setitem__('first-red-test.py', b'unknown'))

    def test_reused_birth(self):
        key = 'native/test_owner_loss_terminates_running_root_and_writer/result.json'

        def edit(_, members):
            result = json.loads(members[key])
            result['terminal'][-1]['creation_100ns'] += 1
            members[key] = json.dumps(result).encode()
        self.reject(edit)


if __name__ == '__main__':
    unittest.main()
