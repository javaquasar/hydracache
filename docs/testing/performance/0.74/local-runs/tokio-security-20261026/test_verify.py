import hashlib
import importlib.util
import json
from pathlib import Path
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location('tokio_packet', HERE/'verify.py')
verify = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(verify)


class Integrity(unittest.TestCase):
    def test_actual_is_not_release_admission(self):
        self.assertFalse(verify.check()['release_admitted'])

    def test_index_mutation_refused(self):
        with self.assertRaisesRegex(ValueError, 'index differs'):
            verify.check(lambda name: b'{}' if name == 'index.json' else (HERE/name).read_bytes())

    def test_changed_raw_objects_refused(self):
        index = json.loads((HERE/'index.json').read_bytes())
        for member in ('source/Cargo.lock','source/tools/long-run-supervisor-074/src/diagnostic_manager_alarm_tests.rs',
                       'checks/policy/result.json'):
            with self.subTest(member=member):
                chosen = 'objects/'+index['members'][member]['sha256']
                with self.assertRaisesRegex(ValueError, 'object differs'):
                    verify.check(lambda name: (HERE/name).read_bytes()+(b' ' if name == chosen else b''))

    def test_semantic_promotion_refused_even_with_test_repin(self):
        index = json.loads((HERE/'index.json').read_bytes())
        index['release_admitted'] = True
        raw = json.dumps(index).encode()
        with patch.object(verify, 'INDEX_SHA256', hashlib.sha256(raw).hexdigest()):
            with self.assertRaisesRegex(ValueError, 'packet promoted'):
                verify.check(lambda name: raw if name == 'index.json' else (HERE/name).read_bytes())

    def changed_member(self, member, transform, reason):
        # A test-only re-pin bypasses the hash boundary to exercise semantic gates.
        # It cannot authorize a new archive or modify the committed verifier pin.
        index = json.loads((HERE/'index.json').read_bytes())
        old = (HERE/'objects'/index['members'][member]['sha256']).read_bytes()
        changed = transform(old)
        self.assertNotEqual(old, changed)
        digest = hashlib.sha256(changed).hexdigest()
        index['members'][member] = dict(bytes=len(changed), sha256=digest)
        raw = json.dumps(index).encode()
        def load(name):
            if name == 'index.json':
                return raw
            if name == 'objects/'+digest:
                return changed
            return (HERE/name).read_bytes()
        with patch.object(verify, 'INDEX_SHA256', hashlib.sha256(raw).hexdigest()):
            with self.assertRaisesRegex(ValueError, reason):
                verify.check(load)

    def test_changed_paired_oracle_refused_even_with_test_repin(self):
        self.changed_member('source/tools/long-run-supervisor-074/src/diagnostic_manager_alarm_tests.rs',
            lambda raw: raw+b' ', 'paired oracle differs')

    def test_extra_graph_delta_refused_even_with_test_repin(self):
        self.changed_member('source/Cargo.lock',
            lambda raw: raw.replace(b' "tokio-macros",',b' "tokio-macros-foreign",',1),
            'unrelated lock delta')


if __name__ == '__main__':
    unittest.main()
