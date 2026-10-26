"""Tamper controls; test-only re-pinning grants no evidence approval."""
import importlib.util
import json
from pathlib import Path
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location('view_verify',HERE/'verify.py')
verify = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(verify)

class Packet(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.files = {'index.json':(HERE/'index.json').read_bytes()}
        index = json.loads(cls.files['index.json'])
        cls.files.update({'objects/'+row['sha256']:(HERE/'objects'/row['sha256']).read_bytes()
                         for row in index['members'].values()})

    def mutate(self, member, change, is_json=True):
        files = dict(self.files)
        index = json.loads(files['index.json'])
        if member is None:
            change(index)
        else:
            row = index['members'][member]
            raw = files['objects/'+row['sha256']]
            if is_json:
                value = json.loads(raw)
                change(value)
                raw = json.dumps(value).encode()
            else:
                raw = change(raw)
            digest = verify.sha(raw)
            files['objects/'+digest] = raw
            index['members'][member] = dict(bytes=len(raw),sha256=digest)
        files['index.json'] = json.dumps(index).encode()
        with patch.object(verify,'INDEX_SHA256',verify.sha(files['index.json'])):
            with self.assertRaises(ValueError):
                verify.check(files.__getitem__)

    def test_exact_packet(self):
        self.assertEqual(verify.check(self.files.__getitem__)['members'],284)

    def test_index_and_object_tamper_refuse(self):
        for name in ('index.json',next(n for n in self.files if n.startswith('objects/'))):
            files = dict(self.files)
            files[name] += b' '
            with self.assertRaises(ValueError):
                verify.check(files.__getitem__)

    def test_promotion_and_executing_edit_scope_refuse(self):
        for field in ('package_pass','historical_source_is_executed','release_admitted'):
            self.mutate(None,lambda v,f=field:v.update({f:True}))
        self.mutate(None,lambda v:v.update(current_root_executing_edits=2))

    def test_first_failure_and_count_changes_refuse(self):
        self.mutate('checks/manager-guards/result.json',lambda v:v.update(exit=0))
        self.mutate(None,lambda v:v['counts']['manager-guards'].update(totals=[14,0,0,0,307]))

    def test_current_root_and_diagnostic_text_changes_refuse(self):
        self.mutate('current/'+verify.PATHS[-1],lambda raw:raw+b'\n// weakened\n',False)
        self.mutate('current/tools/source-successor-view-074/diagnostic-additions.txt',
                    lambda raw:raw.replace(b'refuse',b'accept',1),False)

    def test_linux_context_cannot_disappear(self):
        self.mutate('checks/gnu-focused-explicit-git/intent.json',
                    lambda v:v.update(argv=[a for a in v['argv'] if not a.startswith('GIT_DIR=')]))

if __name__ == '__main__':
    unittest.main()
