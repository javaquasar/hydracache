"""Offline integrity mutations; no captured process or command is executed."""
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
        with tempfile.TemporaryDirectory(prefix='hc074-packet-') as name:
            folder = Path(name)
            (folder / 'index.json').write_bytes(b'{}')
            with self.assertRaisesRegex(ValueError, 'fixed index'):
                verify.verify(folder)

    def test_runtime_drift(self):
        self.reject(lambda _, m: m.__setitem__('source-final/scripts/ci/run-with-heartbeat.py', b'unknown'))

    def test_removed_old_marker_verdict(self):
        key = 'checks/ci-watchdog-msrv-final/stdout.log'
        self.reject(lambda _, m: m.__setitem__(key, m[key].replace(
            b'test watchdog_times_out_and_kills_descendant_processes ... ok', b'missing verdict')))

    def test_reused_birth(self):
        key = 'native/test_owner_loss_terminates_running_root_and_writer/result.json'

        def edit(_, members):
            value = json.loads(members[key])
            value['terminal'][-1]['creation_100ns'] += 1
            members[key] = json.dumps(value).encode()
        self.reject(edit)

    def test_write_after_terminal(self):
        key = 'native/test_owner_loss_terminates_running_root_and_writer/result.json'

        def edit(_, members):
            value = json.loads(members[key])
            value['post_terminal_sizes'][1] += 1
            members[key] = json.dumps(value).encode()
        self.reject(edit)

    def test_wrong_injected_error(self):
        key = 'unsupported/test_unsupported_job_list_refuses_without_creating_child/result.json'

        def edit(_, members):
            value = json.loads(members[key])
            value['refusal'] = 'WinError 122'
            members[key] = json.dumps(value).encode()
        self.reject(edit)


if __name__ == '__main__':
    unittest.main()
