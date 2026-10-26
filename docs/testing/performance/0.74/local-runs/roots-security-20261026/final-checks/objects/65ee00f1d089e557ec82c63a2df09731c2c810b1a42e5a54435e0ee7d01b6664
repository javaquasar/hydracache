import copy
import importlib.util
import json
from pathlib import Path
import unittest

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location('roots_packet', HERE/'verify.py')
verify = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(verify)


class Integrity(unittest.TestCase):
    def test_actual_packet_is_not_admission(self):
        self.assertFalse(verify.check()['candidate_accepted'])

    def test_index_mutation_refused(self):
        with self.assertRaisesRegex(ValueError, 'index differs'):
            verify.check(lambda name: b'{}' if name == 'index.json' else (HERE/name).read_bytes())

    def test_changed_source_inventory_and_first_result_refused(self):
        index = json.loads((HERE/'index.json').read_bytes())
        for member in ('source/Cargo.lock', 'coordinator/roots-trust-inventory-20261026.json',
                       'checks/roots-catalog-delta/result.json'):
            with self.subTest(member=member):
                chosen = 'objects/'+index['members'][member]['sha256']
                def load(name):
                    raw = (HERE/name).read_bytes()
                    return raw+b' ' if name == chosen else raw
                with self.assertRaisesRegex(ValueError, 'object differs'):
                    verify.check(load)

    def test_semantic_promotion_refused(self):
        index = json.loads((HERE/'index.json').read_bytes())
        def read(name):
            return (HERE/'objects'/index['members'][name]['sha256']).read_bytes()
        for field in ('candidate_accepted', 'trust_scope_approved', 'release_admitted', 'policy_pass'):
            with self.subTest(field=field):
                changed = copy.deepcopy(index)
                changed[field] = True
                with self.assertRaisesRegex(ValueError, 'packet promoted'):
                    verify.validate(changed, read)

    def test_paired_oracle_substitution_refused(self):
        index = json.loads((HERE/'index.json').read_bytes())
        def read(name):
            raw = (HERE/'objects'/index['members'][name]['sha256']).read_bytes()
            return raw+b' ' if name == 'source/crates/hydracache-redis-compat/tests/tls_roots_security_074.rs' else raw
        with self.assertRaisesRegex(ValueError, 'paired oracle differs'):
            verify.validate(index, read)

    def test_lost_narrow_gate_refusal_refused(self):
        index = json.loads((HERE/'index.json').read_bytes())
        def read(name):
            raw = (HERE/'objects'/index['members'][name]['sha256']).read_bytes()
            if name == 'checks/roots-catalog-delta/result.json':
                changed = json.loads(raw)
                changed['exit'] = 0
                return json.dumps(changed).encode()
            return raw
        with self.assertRaisesRegex(ValueError, 'first phase verdict differs'):
            verify.validate(index, read)


if __name__ == '__main__':
    unittest.main()
