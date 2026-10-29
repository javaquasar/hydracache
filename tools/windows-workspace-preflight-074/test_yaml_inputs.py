import copy
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import security_lock
import swagger_inputs
import yaml_inputs as y

ROOT = Path(__file__).resolve().parents[2]


def original_yaml_input(name):
    # The versioned YAML API still tests its immutable input after root D-Bus adoption.
    packet, digest = {
        'Cargo.lock': ('dbus-root-integration-20261028', y.BODIES['Cargo.lock'][0]),
        'Cargo.toml': ('dbus-root-integration-20261028', 'a494481b85fa84fd07e19a447c6d5fe7a49aa12557e0c0befc86fdea390d3bf2'),
        'crates/xtask/Cargo.toml': ('yaml-security-20261028', '5f29059c1f8dcc7b925876565090e8133697fcffc76499f01c0fbeffb3356d50'),
    }[name]
    raw = (ROOT/'docs/testing/performance/0.74/local-runs'/packet/'objects'/digest).read_bytes()
    if swagger_inputs.sha(raw) != digest:
        raise ValueError('immutable original YAML input')
    return raw


def data():
    return dict(auxiliary={'Cargo.lock': security_lock.PREVIOUS,
        'Cargo.toml': swagger_inputs.BODIES['Cargo.toml'][1],
        'crates/xtask/Cargo.toml': y.BODIES['crates/xtask/Cargo.toml'][1], 'deny.toml': 'original'},
        sources={'original.rs': 'original'}, targets=['original'])


class YamlInputs(unittest.TestCase):
    def test_complete_predecessors_and_both_older_layers(self):
        for name, (_, previous, _) in y.BODIES.items():
            raw = y.predecessor(name, original_yaml_input(name))
            self.assertEqual(swagger_inputs.sha(raw), previous)
            if name == 'Cargo.lock':
                raw = swagger_inputs.predecessor(name, raw)
                self.assertEqual(swagger_inputs.sha(security_lock.predecessor(raw)), security_lock.PREVIOUS)

    def test_same_inherited_lf_normalization(self):
        for name in y.BODIES:
            raw = original_yaml_input(name).replace(b'\r\n', b'\n')
            self.assertEqual(y.predecessor(name, raw), y.predecessor(name, raw.replace(b'\n', b'\r\n')))

    def test_complete_body_mutation_and_unknown_names_refuse(self):
        for name in y.BODIES:
            for raw in (original_yaml_input(name)+b'\n', b'unknown'):
                with self.subTest(name=name), self.assertRaises(ValueError):
                    y.predecessor(name, raw)
        with self.assertRaises(ValueError):
            y.predecessor('deny.toml', b'unknown')

    def test_offset_content_order_and_complete_predecessor_refuse(self):
        for mutation in ('offset', 'content', 'order', 'digest', 'boolean'):
            rows = y.rules()
            row = rows['Cargo.lock']
            if mutation == 'offset': row['patches'][-1]['offset'] += 1
            elif mutation == 'content': row['patches'][-1]['previous'] += 'unreviewed'
            elif mutation == 'order': row['patches'].reverse()
            elif mutation == 'digest': row['previous_sha256'] = '0'*64
            else: row['patches'][-1]['offset'] = True
            with self.subTest(mutation=mutation), patch.object(y, 'rules', return_value=rows), self.assertRaises(ValueError):
                y.predecessor('Cargo.lock', original_yaml_input('Cargo.lock'))

    def test_rules_tampering_and_schema_mutation_refuse(self):
        with patch.object(Path, 'read_bytes', return_value=b'{}'), self.assertRaises(ValueError):
            y.rules()
        value = json.loads((ROOT/'tools/windows-workspace-preflight-074/yaml-inputs.json').read_bytes())
        value['unknown'] = True
        raw = json.dumps(value).encode()
        with patch.object(Path, 'read_bytes', return_value=raw), patch.object(y, 'RULES_SHA', swagger_inputs.sha(raw)), self.assertRaises(ValueError):
            y.rules()

    def test_only_three_catalog_entries_change_without_git_reads(self):
        original = data(); before = copy.deepcopy(original)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in ('Cargo.lock','Cargo.toml','crates/xtask/Cargo.toml'):
                (root/name).parent.mkdir(parents=True, exist_ok=True)
                (root/name).write_bytes(original_yaml_input(name))
            with patch.object(subprocess, 'check_output', side_effect=AssertionError('no extra Git reads')):
                result = y.reviewed_inputs(root, original)
        expected = copy.deepcopy(before)
        for name, (current, _, _) in y.BODIES.items(): expected['auxiliary'][name] = current
        expected['auxiliary']['Cargo.toml'] = swagger_inputs.BODIES['Cargo.toml'][0]
        self.assertEqual(result, expected)
        self.assertEqual(original, before)

    def test_bad_catalog_and_reparse_refuse_before_read(self):
        for name in ('Cargo.lock', 'Cargo.toml', 'crates/xtask/Cargo.toml'):
            original = data(); original['auxiliary'][name] = 'unknown'
            with patch.object(Path, 'read_bytes', side_effect=AssertionError('wrong catalog')), self.assertRaises(ValueError):
                y.reviewed_inputs(ROOT, original)
        with patch.object(Path, 'is_junction', return_value=True), \
                patch.object(Path, 'read_bytes', side_effect=AssertionError('junction')), self.assertRaises(ValueError):
            y.reviewed_inputs(ROOT, data())

    def test_swagger_build_overrides_still_refuse_before_read(self):
        for name in ('SWAGGER_UI_DOWNLOAD_URL', 'SWAGGER_UI_OVERWRITE_FOLDER', 'SWAGGER_UI_DOWNLOAD_CACHE_DIR'):
            with patch.dict(os.environ, {name: 'unreviewed'}), \
                    patch.object(Path, 'read_bytes', side_effect=AssertionError('override')), self.assertRaises(ValueError):
                y.reviewed_inputs(ROOT, data())
