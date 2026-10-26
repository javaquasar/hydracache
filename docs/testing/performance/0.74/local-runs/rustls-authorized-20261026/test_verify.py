import importlib.util
import json
from pathlib import Path
import unittest

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location('authorized_rustls_packet', HERE/'verify.py')
verify = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(verify)


class Integrity(unittest.TestCase):
    def test_actual_packet_keeps_admission_closed(self):
        result = verify.check()
        self.assertTrue(result['integrity'])
        self.assertFalse(result['release_admitted'])

    def test_changed_index_is_refused(self):
        with self.assertRaisesRegex(ValueError, 'index differs'):
            verify.check(lambda name: b'{}' if name == 'index.json' else (HERE/name).read_bytes())

    def test_changed_object_is_refused(self):
        index = json.loads((HERE/'index.json').read_bytes())
        selected = 'objects/'+index['members']['source/Cargo.lock']['sha256']
        def load(name):
            raw = (HERE/name).read_bytes()
            return raw+b'\n' if name == selected else raw
        with self.assertRaisesRegex(ValueError, 'object differs'):
            verify.check(load)


if __name__ == '__main__':
    unittest.main()
