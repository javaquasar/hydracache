import copy
import json
import os
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch

import security_lock
import swagger_inputs
import yaml_inputs as y

ROOT = Path(__file__).resolve().parents[2]


def data():
    return dict(auxiliary={'Cargo.lock': security_lock.PREVIOUS,
        'Cargo.toml': swagger_inputs.BODIES['Cargo.toml'][1],
        'crates/xtask/Cargo.toml': y.BODIES['crates/xtask/Cargo.toml'][1], 'deny.toml': 'original'},
        sources={'original.rs': 'original'}, targets=['original'])


class YamlInputs(unittest.TestCase):
    def test_complete_predecessors_and_both_older_layers(self):
        for name, (_, previous, _) in y.BODIES.items():
            raw = y.predecessor(name, (ROOT/name).read_bytes())
            self.assertEqual(swagger_inputs.sha(raw), previous)
            if name == 'Cargo.lock':
                raw = swagger_inputs.predecessor(name, raw)
                self.assertEqual(swagger_inputs.sha(security_lock.predecessor(raw)), security_lock.PREVIOUS)

    def test_same_inherited_lf_normalization(self):
        for name in y.BODIES:
            raw = (ROOT/name).read_bytes().replace(b'\r\n', b'\n')
            self.assertEqual(y.predecessor(name, raw), y.predecessor(name, raw.replace(b'\n', b'\r\n')))

    def test_complete_body_mutation_and_unknown_names_refuse(self):
        for name in y.BODIES:
            for raw in ((ROOT/name).read_bytes()+b'\n', b'unknown'):
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
                y.predecessor('Cargo.lock', (ROOT/'Cargo.lock').read_bytes())

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
        with patch.object(subprocess, 'check_output', side_effect=AssertionError('no extra Git reads')):
            result = y.reviewed_inputs(ROOT, original)
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
