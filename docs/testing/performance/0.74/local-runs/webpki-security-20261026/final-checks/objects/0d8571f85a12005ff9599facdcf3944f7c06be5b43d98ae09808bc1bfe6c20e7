import importlib.util
import json
from pathlib import Path
import unittest

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location('webpki_security_packet', HERE/'verify.py')
verify = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(verify)


class Integrity(unittest.TestCase):
    def test_actual_packet_is_not_release_admission(self):
        self.assertFalse(verify.check()['release_admitted'])

    def test_changed_index_is_refused(self):
        with self.assertRaisesRegex(ValueError, 'index differs'):
            verify.check(lambda name: b'{}' if name == 'index.json' else (HERE/name).read_bytes())

    def test_changed_public_vector_is_refused(self):
        index = json.loads((HERE/'index.json').read_bytes())
        chosen = 'objects/'+index['members']['upstream/vectors.json']['sha256']
        def load(name):
            raw = (HERE/name).read_bytes()
            return raw+b' ' if name == chosen else raw
        with self.assertRaisesRegex(ValueError, 'object differs'):
            verify.check(load)


if __name__ == '__main__':
    unittest.main()
