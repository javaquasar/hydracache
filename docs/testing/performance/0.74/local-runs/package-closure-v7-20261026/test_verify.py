"""Pure archive mutations. Test-only re-pinning is never archive approval."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('hc074_v7_refusal_verify', Path(__file__).with_name('verify.py'))
verify = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verify)


class RefusalEvidence(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.raw = (verify.HERE / 'index.json').read_bytes()
        cls.index = json.loads(cls.raw)
        cls.bodies = {'objects/' + row['sha256']: (verify.HERE / 'objects' / row['sha256']).read_bytes()
                      for row in cls.index['members'].values()}

    def test_original(self):
        value = verify.check()
        self.assertTrue(value['verified'])
        self.assertFalse(value['package_pass'])

    def test_index_and_object_mutations(self):
        for target in ('index.json', next(iter(self.bodies))):
            original = dict(self.bodies, **{'index.json': self.raw})
            original[target] += b' '
            with self.assertRaises(ValueError):
                verify.check(original.__getitem__)

    def reject_semantic_change(self, member, change):
        index = copy.deepcopy(self.index)
        bodies = dict(self.bodies)
        pointer = index['members'][member]
        value = json.loads(bodies['objects/' + pointer['sha256']])
        change(value)
        raw = json.dumps(value).encode()
        digest = verify.sha(raw)
        index['members'][member] = dict(sha256=digest, bytes=len(raw))
        bodies['objects/' + digest] = raw
        raw = json.dumps(index).encode()
        bodies['index.json'] = raw
        with patch.object(verify, 'INDEX_SHA256', verify.sha(raw)):
            with self.assertRaises(ValueError):
                verify.check(bodies.__getitem__)

    def test_original_limits_not_widened(self):
        self.reject_semantic_change('package/intent.json', lambda v: v.update(bound_seconds=900))

    def test_no_invented_passed_or_failed_cases(self):
        self.reject_semantic_change('derived/accounting.json', lambda v: v.update(passed=0))

    def test_source_preservation_is_not_just_a_boolean(self):
        self.reject_semantic_change('checks/full-package-first/result.json',
                                    lambda v: v['source_after'].update({'unknown': 'a' * 64}))

    def test_missing_inner_result_stays_missing(self):
        self.reject_semantic_change('derived/accounting.json', lambda v: v.update(original_inner_result_missing=False))

    def test_postflight_scope_stays_exact(self):
        self.reject_semantic_change('checks/postflight-first/stdout.log',
                                    lambda v: v['source_review'].update(package_pass=True))


if __name__ == '__main__':
    unittest.main()
