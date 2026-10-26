"""Tamper controls. Test-only re-pinning exercises semantic rejection, not admission."""
import importlib.util
import json
from pathlib import Path
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location('evidence_verify',HERE/'verify.py')
verify = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(verify)


class Evidence(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.files = {'index.json':(HERE/'index.json').read_bytes()}
        index = json.loads(cls.files['index.json'])
        cls.files.update({'objects/'+r['sha256']:(HERE/'objects'/r['sha256']).read_bytes()
                         for r in index['members'].values()})

    def semantic_mutation(self, member, change):
        files = dict(self.files)
        index = json.loads(files['index.json'])
        if member is None:
            change(index)
        else:
            row = index['members'][member]
            value = json.loads(files['objects/'+row['sha256']])
            change(value)
            raw = json.dumps(value).encode()
            digest = verify.sha(raw)
            files['objects/'+digest] = raw
            index['members'][member] = dict(bytes=len(raw),sha256=digest)
        files['index.json'] = json.dumps(index).encode()
        with patch.object(verify,'INDEX_SHA256',verify.sha(files['index.json'])):
            with self.assertRaises(ValueError):
                verify.check(files.__getitem__)

    def test_exact_packet(self):
        self.assertEqual(verify.check(self.files.__getitem__)['members'],219)

    def test_index_and_raw_object_tamper(self):
        for name in ('index.json',next(n for n in self.files if n.startswith('objects/'))):
            with self.subTest(name=name):
                files = dict(self.files)
                files[name] += b' '
                with self.assertRaises(ValueError):
                    verify.check(files.__getitem__)

    def test_package_or_projection_promotion_refuses(self):
        for field in ('production_projection_installed','package_pass','release_admitted'):
            self.semantic_mutation(None,lambda v,f=field:v.update({f:True}))

    def test_count_and_first_outcome_changes_refuse(self):
        self.semantic_mutation(None,lambda v:v.update(controls=23))
        self.semantic_mutation('checks/old-lock-baseline/result.json',lambda v:v.update(exit=0))

    def test_syntax_and_security_scope_changes_refuse(self):
        self.semantic_mutation('checks/syntax-data-first/result.json',lambda v:v.update(executed_input_code=True))
        self.semantic_mutation('checks/audit-explicit-heartbeat/stdout.log',
            lambda v:v['security'].update(historical_checks_updated=True))

    def test_missing_heartbeat_and_uncaptured_producer_refuse(self):
        self.semantic_mutation('checks/audit-explicit-heartbeat/stdout.log',
            lambda v:v['heartbeat'].update(outside_previous_auxiliary=[]))
        self.semantic_mutation('checks/controls/intent.json',
            lambda v:v['source'].update({'tools/security-successor-audit-074/audit.py':'0'*64}))


if __name__ == '__main__':
    unittest.main()
