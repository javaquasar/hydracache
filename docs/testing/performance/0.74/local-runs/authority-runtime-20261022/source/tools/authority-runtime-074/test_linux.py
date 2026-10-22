"""Offline selector/receipt controls; no Linux image execution."""
import json
import unittest
from unittest.mock import patch
import linux as subject

class Controls(unittest.TestCase):
    def test_exact_frozen_membership_and_no_measurement_names(self):
        tests=subject.selection();self.assertEqual(len(tests),33)
        self.assertFalse(any('allocation' in name or 'attribution' in name or 'privileged' in name for name in tests))
    def test_build_requires_exactly_one_lib_test_image(self):
        row=dict(reason='compiler-artifact',executable='/cache/test',target=dict(name='hydracache_long_run_supervisor_074'),profile=dict(test=True))
        raw=json.dumps(row).encode()+b'\n';self.assertEqual(subject.image(raw),'/cache/test')
        for value in (b'',raw+raw):
            with self.assertRaises(ValueError):subject.image(value)
    def test_one_test_and_zero_ignored_are_mandatory(self):
        raw=b'running 1 test\ntest exact_name ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 900 filtered out\n'
        subject.test_passed(raw,'exact_name')
        for value in (raw.replace(b'1 passed',b'0 passed'),raw.replace(b'0 ignored',b'1 ignored'),raw.replace(b'exact_name',b'other'),raw.replace(b'running 1 test',b'running 34 tests')):
            with self.assertRaises(ValueError):subject.test_passed(value,'exact_name')
    def test_schema_membership_and_new_test_count_refuse(self):
        original=subject.Path.read_text
        def bad(path,*args,**kwargs):
            row=json.loads(original(path,*args,**kwargs));row['tests']=row['tests'][:-1];return json.dumps(row)
        with patch.object(subject.Path,'read_text',bad),self.assertRaises(ValueError):subject.selection()

if __name__=='__main__':unittest.main(verbosity=2)
