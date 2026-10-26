"""Offline tamper controls. Re-pinning in tests does not approve changed data."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('hc074_roots_approved_verify', Path(__file__).with_name('verify.py'))
verify = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verify)


class ApprovedEvidence(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.raw = (verify.HERE / 'index.json').read_bytes()
        cls.index = json.loads(cls.raw)
        cls.bodies = {'objects/' + row['sha256']: (verify.HERE / 'objects' / row['sha256']).read_bytes()
                      for row in cls.index['members'].values()}

    def test_exact_packet(self):
        result = verify.check()
        self.assertTrue(result['approved_exact_public_catalog'])
        self.assertFalse(result['policy_pass'])

    def test_index_and_body_changes(self):
        for target in ('index.json', next(iter(self.bodies))):
            bodies = dict(self.bodies, **{'index.json': self.raw})
            bodies[target] += b'X'
            with self.assertRaises(ValueError):
                verify.check(bodies.__getitem__)

    def reject(self, name, mutate):
        index, bodies = copy.deepcopy(self.index), dict(self.bodies)
        row = index['members'][name]
        value = json.loads(bodies['objects/' + row['sha256']])
        mutate(value)
        raw = json.dumps(value).encode()
        digest = verify.sha(raw)
        index['members'][name] = dict(sha256=digest, bytes=len(raw))
        bodies['objects/' + digest] = raw
        raw = json.dumps(index).encode()
        bodies['index.json'] = raw
        with patch.object(verify, 'INDEX_SHA', verify.sha(raw)):
            with self.assertRaises(ValueError):
                verify.check(bodies.__getitem__)

    def test_fingerprint_and_constraint_changes(self):
        for key in ('spki_sha256', 'certificate_sha256', 'name_constraints_hex'):
            self.reject('implementation/approved-catalog.json', lambda v: v['added'][0].update({key: '00'}))

    def test_future_catalog_is_not_approved(self):
        self.reject('implementation/approved-catalog.json', lambda v: v['versions'].update({'webpki-roots': '1.0.11'}))

    def test_old_refusal_cannot_be_promoted(self):
        self.reject('historical/coordinator/roots-trust-inventory-20261026.json', lambda v: v.update(trust_scope_approved=True))

    def test_failed_policy_cannot_be_called_green(self):
        self.reject('checks/dependency-policy-first/result.json', lambda v: v.update(exit=0))

    def test_v7_refusal_cannot_dispatch(self):
        self.reject('v7-refusal/preflight-refusal.json', lambda v: v.update(package_dispatched=True))


if __name__ == '__main__':
    unittest.main()
