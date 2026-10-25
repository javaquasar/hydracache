import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('packet_reader', HERE/'verify.py')
reader = importlib.util.module_from_spec(spec)
spec.loader.exec_module(reader)


class Controls(unittest.TestCase):
    def setUp(self):
        self.index = json.loads((HERE/'index.json').read_bytes())
        self.objects = {key: (HERE/'objects'/key).read_bytes() for key in {v['sha256'] for v in self.index['members'].values()}}

    def read(self, name):
        return json.dumps(self.index).encode() if name == 'index.json' else self.objects[name.split('/')[1]]

    def change(self, member, mutate):
        old = self.index['members'][member]
        value = json.loads(self.objects[old['sha256']])
        mutate(value)
        raw = json.dumps(value).encode()
        digest = hashlib.sha256(raw).hexdigest()
        self.objects[digest] = raw
        self.index['members'][member] = dict(sha256=digest, bytes=len(raw))

    def test_original_packet(self):
        self.assertEqual(reader.verify(self.read)['windows_standalone_off_on'], [89,89])

    def test_raw_mutation(self):
        digest = self.index['members']['server/get-owner-d3-build-first/result.json']['sha256']
        self.objects[digest] += b' '
        with self.assertRaises(ValueError): reader.verify(self.read)

    def test_rehashed_first_refusal_cannot_become_success(self):
        self.change('server/get-owner-material-checks-first/result.json', lambda d: d.update(passed=True))
        with self.assertRaises(ValueError): reader.verify(self.read)

    def test_rehashed_provider_without_coverage_cannot_pass(self):
        self.change('server/get-owner-provider-control-first/control.stdout.log', lambda d: d['samples'][1].update(covered=False))
        with self.assertRaises(ValueError): reader.verify(self.read)

    def test_cannot_promote_preparation_into_full_d3(self):
        self.index = copy.deepcopy(self.index)
        self.index['d3_complete'] = True
        with self.assertRaises(ValueError): reader.verify(self.read)


if __name__ == '__main__': unittest.main()
