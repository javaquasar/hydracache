"""Independently verify held bytes, accounting and negative scenario completeness."""
from pathlib import Path
import importlib.util
import json
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('native_retention_review', Path(__file__).with_name('retain.py'))
r = importlib.util.module_from_spec(spec)
spec.loader.exec_module(r)
INDEX = '5b52a78c032b622f401079ce2b0d895e47ba93504c4cffcf71bcdf024c87f701'


class Retained(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        raw = (r.OUT / 'index.json').read_bytes()
        if r.sha(raw) != INDEX:
            raise ValueError('whole held native transport index')
        cls.members = json.loads(raw)['files']

    def member(self, name):
        row = self.members[name]
        raw = (r.OUT / 'objects' / row['sha256']).read_bytes()
        self.assertEqual(r.sha(raw), row['sha256'])
        self.assertEqual(len(raw), row['bytes'])
        return raw

    def hydrate(self, folder):
        prefix = 'f2/units/'
        for name in self.members:
            if name.startswith(prefix):
                dest = folder / name[len(prefix):]
                dest.parent.mkdir(parents=True, exist_ok=True)
                dest.write_bytes(self.member(name))

    def test_all_packet_bytes_and_original_failures_are_retained(self):
        self.assertEqual(len(self.members), 472)
        for name in self.members:
            self.member(name)
        first = json.loads(self.member('r/private-bus/native-result.json'))
        self.assertFalse(first['passed'])
        self.assertTrue(first['source_unchanged'] and first['boot_unchanged'])
        floor = json.loads(self.member('c/session/execution.json'))
        self.assertEqual(floor['classification'], 'resource-floor')
        self.assertTrue(floor['retired'])
        self.assertNotIn('c/session/native-result.json', self.members)

    def test_exact_actual_cohort_and_fault_markers_then_missing_case_refuse(self):
        with tempfile.TemporaryDirectory(dir=r.ROOT / 'target') as name:
            folder = Path(name)
            self.hydrate(folder)
            r.verify_cohort(folder, 65)
            path = folder / 'fd-188-test/stdout.log'
            raw = path.read_text()
            line = next(s for s in raw.splitlines(True) if s.startswith('HC074-NEGOTIATED case=lost-ack '))
            path.write_text(raw.replace(line, '', 1))
            with self.assertRaises(ValueError):
                r.verify_cohort(folder, 65)

    def test_success_total_cannot_conceal_duplicate_or_wrong_case(self):
        with tempfile.TemporaryDirectory(dir=r.ROOT / 'target') as name:
            folder = Path(name)
            self.hydrate(folder)
            path = folder / 'fd-194-test/result.json'
            value = json.loads(path.read_bytes())
            value['rows'][0] = value['rows'][1]
            path.write_text(json.dumps(value))
            with self.assertRaises(ValueError):
                r.verify_cohort(folder, 65)


if __name__ == '__main__':
    unittest.main()
