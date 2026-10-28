import copy
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import security_lock
import swagger_inputs as s

ROOT = Path(__file__).resolve().parents[2]


def original_swagger_input(name):
    # Preserve this API against its immutable input, not a newer YAML lock.
    digest = {'Cargo.lock': s.BODIES['Cargo.lock'][0],
              'Cargo.toml': 'a494481b85fa84fd07e19a447c6d5fe7a49aa12557e0c0befc86fdea390d3bf2'}[name]
    raw = (ROOT/'docs/testing/performance/0.74/local-runs/swagger-security-20261028/objects'/digest).read_bytes()
    if s.sha(raw) != digest:
        raise ValueError('immutable original Swagger input')
    return raw


def data():
    return dict(auxiliary={'Cargo.lock': security_lock.PREVIOUS,
                           'Cargo.toml': s.BODIES['Cargo.toml'][1], 'deny.toml': 'unchanged'},
                sources={'original.rs': 'unchanged'}, targets=['original'])


class SwaggerInputs(unittest.TestCase):
    def test_complete_predecessors_and_original_sys_layer(self):
        for name, (_, previous, _) in s.BODIES.items():
            restored = s.predecessor(name, original_swagger_input(name))
            self.assertEqual(s.sha(restored), previous)
            if name == 'Cargo.lock':
                self.assertEqual(s.sha(security_lock.predecessor(restored)), security_lock.PREVIOUS)

    def test_same_normalization_for_both_crlf_and_lf(self):
        for name in s.BODIES:
            raw = original_swagger_input(name).replace(b'\r\n', b'\n')
            self.assertEqual(s.predecessor(name, raw), s.predecessor(name, raw.replace(b'\n', b'\r\n')))

    def test_mutated_complete_inputs_and_unknown_names_refuse(self):
        for name in s.BODIES:
            for raw in (original_swagger_input(name)+b'\n', b'unknown'):
                with self.subTest(name=name), self.assertRaises(ValueError):
                    s.predecessor(name, raw)
        with self.assertRaises(ValueError):
            s.predecessor('deny.toml', b'unknown')

    def test_bad_offsets_and_bad_inverse_content_refuse(self):
        for mutation in ('offset', 'previous'):
            rows = s.rules()
            if mutation == 'offset':
                rows['Cargo.lock']['patches'][-1]['offset'] += 1
            else:
                rows['Cargo.lock']['patches'][-1]['previous'] += 'unreviewed'
            with patch.object(s, 'rules', return_value=rows), self.assertRaises(ValueError):
                s.predecessor('Cargo.lock', original_swagger_input('Cargo.lock'))

    def test_inverse_requires_complete_predecessor_digest(self):
        rows = s.rules()
        rows['Cargo.lock']['previous_sha256'] = '0'*64
        with patch.object(s, 'rules', return_value=rows), self.assertRaises(ValueError):
            s.predecessor('Cargo.lock', original_swagger_input('Cargo.lock'))

    def test_rules_tampering_and_schema_mutation_refuse(self):
        with patch.object(Path, 'read_bytes', return_value=b'{}'), self.assertRaises(ValueError):
            s.rules()
        value = json.loads((ROOT/'tools/windows-workspace-preflight-074/swagger-inputs.json').read_bytes())
        value['unreviewed'] = True
        raw = json.dumps(value).encode()
        with patch.object(Path, 'read_bytes', return_value=raw), patch.object(s, 'RULES_SHA', s.sha(raw)), self.assertRaises(ValueError):
            s.rules()

    def test_only_two_reviewed_catalog_entries_change_without_git_reads(self):
        original = data()
        before = copy.deepcopy(original)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in s.BODIES:
                (root/name).write_bytes(original_swagger_input(name))
            with patch.object(subprocess, 'check_output', side_effect=AssertionError('no additional Git read')):
                result = s.reviewed_inputs(root, original)
        self.assertEqual(original, before)
        expected = copy.deepcopy(before)
        for name, (current, _, _) in s.BODIES.items():
            expected['auxiliary'][name] = current
        self.assertEqual(result, expected)

    def test_wrong_catalog_and_reparse_refuse_before_read(self):
        original = data()
        original['auxiliary']['Cargo.toml'] = 'unknown'
        with patch.object(Path, 'read_bytes', side_effect=AssertionError('wrong catalog')), self.assertRaises(ValueError):
            s.reviewed_inputs(ROOT, original)
        with patch.object(Path, 'is_junction', return_value=True), \
                patch.object(Path, 'read_bytes', side_effect=AssertionError('junction')), self.assertRaises(ValueError):
            s.reviewed_inputs(ROOT, data())

    def test_all_swagger_overrides_refuse_before_disk(self):
        for name in ('SWAGGER_UI_DOWNLOAD_URL', 'SWAGGER_UI_OVERWRITE_FOLDER', 'SWAGGER_UI_DOWNLOAD_CACHE_DIR'):
            with self.subTest(name=name), patch.dict(os.environ, {name: 'unreviewed'}), \
                    patch.object(Path, 'read_bytes', side_effect=AssertionError('override')), self.assertRaises(ValueError):
                s.reviewed_inputs(ROOT, data())
