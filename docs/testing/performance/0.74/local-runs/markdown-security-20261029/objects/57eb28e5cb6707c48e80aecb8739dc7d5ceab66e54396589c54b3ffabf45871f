"""Offline exact-key, DER, constraints, registry and graph refusal controls."""
import copy
import json
from pathlib import Path
import tempfile
import tomllib
import unittest

import audit


class ExactPublicCatalog(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.read = staticmethod(audit.packet_reader())
        cls.policy = json.loads(audit.POLICY.read_bytes())
        cls.old = audit.parse_anchors(cls.read('upstream/webpki-roots-1.0.8/src/lib.rs'))
        cls.new = audit.parse_anchors(cls.read('upstream/webpki-roots-1.0.10/src/lib.rs'))
        cls.old_der = audit.parse_certificates(cls.read('upstream/webpki-root-certs-1.0.8/src/lib.rs'))
        cls.new_der = audit.parse_certificates(cls.read('upstream/webpki-root-certs-1.0.10/src/lib.rs'))

    def validate(self, old=None, new=None, old_der=None, new_der=None, policy=None):
        return audit.validate_catalogs(self.old if old is None else old, self.new if new is None else new,
                    self.old_der if old_der is None else old_der, self.new_der if new_der is None else new_der,
                    self.policy if policy is None else policy)

    def test_approved_exact_delta_not_release_admission(self):
        result = audit.audit(self.read, self.policy)
        self.assertTrue(result['approved_exact_public_catalog'])
        self.assertFalse(result['release_admitted'])
        self.assertFalse(result['production_enrollment_admitted'])
        self.assertEqual((len(result['removed']), len(result['added'])), (2, 4))

    def test_changed_unchanged_constraint_and_new_key_refuse(self):
        for select in ('unchanged', 'added'):
            new = self.new.copy()
            row = next(iter((self.old & new) if select == 'unchanged' else (new - self.old)))
            new.subtract({row: 1})
            new[(row[0], row[1] if select == 'unchanged' else row[1] + b'X', b'new-constraint')] += 1
            with self.assertRaises(ValueError):
                self.validate(new=new)

    def test_extra_dropped_duplicate_and_changed_der_refuse(self):
        row = next(iter(self.new))
        for count in (-1, 1):
            new = self.new.copy()
            new[row] += count
            with self.assertRaises(ValueError):
                self.validate(new=new)
        new = self.new_der.copy()
        row = next(iter(new - self.old_der))
        new.subtract({row: 1})
        new[row[:-1] + bytes([row[-1] ^ 1])] += 1
        with self.assertRaisesRegex(ValueError, 'unapproved added'):
            self.validate(new_der=new)

    def test_wrong_approved_fingerprint_or_version_or_production_scope_refuse(self):
        for key in ('spki_sha256', 'certificate_sha256', 'name_constraints_hex'):
            policy = copy.deepcopy(self.policy)
            policy['added'][0][key] = '00' * 32
            with self.assertRaises(ValueError):
                self.validate(policy=policy)
        for change in ({'release_admitted': True}, {'versions': {'webpki-roots': '1.0.11'}}):
            with self.assertRaises(ValueError):
                audit.audit(self.read, {**self.policy, **change})

    def test_der_rejects_truncation_indefinite_and_nonminimal_lengths(self):
        for raw in (b'0', b'0\x80', b'0\x81\x01X', b'0\x82\x00\x80', b'0\x03x'):
            with self.assertRaises(ValueError):
                audit.fields(raw)
        with self.assertRaises(ValueError):
            audit.certificate_identity(b'0\x00')

    def test_incomplete_source_parsers_refuse(self):
        with self.assertRaises(ValueError):
            audit.parse_anchors(b'subject_public_key_info: Der::from_slice(unknown)')
        with self.assertRaises(ValueError):
            audit.parse_certificates(b'CertificateDer::from_slice(unknown)')

    def test_graph_exact_pair_and_unrelated_change_refusal(self):
        before = tomllib.loads(self.read('original/Cargo.lock').decode())
        after = tomllib.loads(self.read('source/Cargo.lock').decode())
        self.assertTrue(audit.validate_graph(before, after)['exact_two_catalog_updates_and_three_test_edges'])
        after['package'][0]['version'] += '.unknown'
        with self.assertRaisesRegex(ValueError, 'unrelated lock'):
            audit.validate_graph(before, after)

    def test_registry_missing_files_refuse(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(FileNotFoundError):
                audit.registry_audit(Path(directory) / 'src/index', self.read)


if __name__ == '__main__':
    unittest.main()
