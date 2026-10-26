import hashlib
import tempfile
from pathlib import Path
import unittest

import caller_pins


class CallerPinControls(unittest.TestCase):
    def test_existing_base_anchor_precedes_raw_oid_derivation(self):
        body = b'canonical\n'
        value = caller_pins.verify_body(body, hashlib.sha256(body).hexdigest(), len(body))
        self.assertEqual(value['oid'], hashlib.sha1(b'blob 10\0' + body).hexdigest())

    def test_stale_body_pin_and_length_never_refresh(self):
        body = b'baseline\n'
        digest = hashlib.sha256(body).hexdigest()
        for raw, size in ((b'changed\n', None), (body, 1), (body + b'\0', None)):
            with self.subTest(size=size):
                with self.assertRaisesRegex(ValueError, 'anchor mismatch'):
                    caller_pins.verify_body(raw, digest, size)

    def test_normalized_anchor_is_not_silently_promoted_to_crlf_pin(self):
        body = b'canonical\r\n'
        with self.assertRaises(ValueError):
            caller_pins.verify_body(body, hashlib.sha256(body).hexdigest())

    def test_non_utf8_cannot_be_a_census_caller_value(self):
        with self.assertRaises(UnicodeDecodeError):
            caller_pins.verify_body(b'\xff', hashlib.sha256(b'\xff').hexdigest())

    def test_duplicate_reference_keys_refuse(self):
        with self.assertRaises(ValueError):
            caller_pins.decode(b'{"a":1,"a":2}')

    def test_packet_tamper_and_object_name_escape_refuse(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            packet = root / caller_pins.PACKET
            packet.mkdir(parents=True)
            (packet / 'index.json').write_bytes(b'{}')
            with self.assertRaisesRegex(ValueError, 'packet drift'):
                caller_pins.prepare(root)
            with self.assertRaisesRegex(ValueError, 'object name'):
                caller_pins.member(root, {'members':{'x':{'sha256':'../escape','bytes':0}}}, 'x')


if __name__ == '__main__':
    unittest.main()
