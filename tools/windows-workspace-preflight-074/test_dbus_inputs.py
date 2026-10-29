"""Whole-input, historical-chain and complete vendor-custody refusal controls."""
import copy
import json
import os
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch

import custody
import dbus_inputs as d
import security_lock
import swagger_inputs as s
import yaml_inputs as y
from test_yaml_inputs import data

ROOT = Path(__file__).resolve().parents[2]


class DbusInputs(unittest.TestCase):
    def test_complete_original_predecessors_and_every_old_chain(self):
        for name, (_, expected, _) in d.BODIES.items():
            prior = d.predecessor(name, (ROOT/name).read_bytes())
            self.assertEqual(s.sha(prior), expected)
            if name == 'Cargo.lock':
                prior = y.predecessor(name, prior)
                prior = s.predecessor(name, prior)
                self.assertEqual(s.sha(security_lock.predecessor(prior)), security_lock.PREVIOUS)
            else:
                self.assertEqual(s.sha(s.predecessor(name, prior)), s.BODIES[name][1])

    def test_original_normalization_handles_mixed_lf_and_crlf(self):
        for name in d.BODIES:
            raw = (ROOT/name).read_bytes().replace(b'\r\n', b'\n')
            self.assertEqual(d.predecessor(name, raw), d.predecessor(name, raw.replace(b'\n', b'\r\n')))

    def test_entire_current_body_and_unknown_names_refuse(self):
        for name in d.BODIES:
            for raw in ((ROOT/name).read_bytes()+b'\n', b'unknown'):
                with self.subTest(name=name), self.assertRaises(ValueError):
                    d.predecessor(name, raw)
        with self.assertRaises(ValueError):
            d.predecessor('deny.toml', b'unknown')

    def test_offset_content_order_digest_and_boolean_faults_refuse(self):
        for mutation in ('offset','current','previous','order','digest','boolean'):
            rows = d.rules()
            row = rows['Cargo.lock']
            if mutation == 'offset': row['patches'][-1]['offset'] += 1
            elif mutation == 'current': row['patches'][-1]['current'] += 'unknown'
            elif mutation == 'previous': row['patches'][-1]['previous'] += 'unknown'
            elif mutation == 'order': row['patches'].reverse()
            elif mutation == 'digest': row['previous_sha256'] = '0'*64
            else: row['patches'][-1]['offset'] = True
            with self.subTest(mutation=mutation), patch.object(d, 'rules', return_value=rows), self.assertRaises(ValueError):
                d.predecessor('Cargo.lock', (ROOT/'Cargo.lock').read_bytes())

    def test_rules_hash_schema_source_pins_and_artifact_set_refuse(self):
        with patch.object(Path, 'read_bytes', return_value=b'{}'), self.assertRaises(ValueError):
            d.rules()
        original = json.loads((ROOT/'tools/windows-workspace-preflight-074/dbus-inputs.json').read_bytes())
        for key in ('schema','previous_revision','source_review_sha256','package_review_sha256','artifacts','extra'):
            value = copy.deepcopy(original)
            value[key] = {} if key == 'artifacts' else 'unreviewed'
            raw = json.dumps(value).encode()
            with self.subTest(key=key), patch.object(Path, 'read_bytes', return_value=raw), \
                 patch.object(d, 'RULES_SHA', s.sha(raw)), self.assertRaises(ValueError):
                d.rules()

    def test_exact_three_final_catalog_entries_without_extra_git_reads(self):
        original = data(); before = copy.deepcopy(original)
        calls = []
        def trace(label, function):
            def observed(*args):
                calls.append(label+':'+args[0] if label != 'sys' else label)
                return function(*args)
            return observed
        with patch.object(subprocess, 'check_output', side_effect=AssertionError('no extra Git reads')), \
             patch.object(y, 'predecessor', side_effect=trace('yaml', y.predecessor)), \
             patch.object(s, 'predecessor', side_effect=trace('swagger', s.predecessor)), \
             patch.object(security_lock, 'predecessor', side_effect=trace('sys', security_lock.predecessor)):
            result = d.reviewed_inputs(ROOT, original)
        expected = copy.deepcopy(before)
        expected['auxiliary'].update({n:v[0] for n,v in d.BODIES.items()})
        expected['auxiliary']['crates/xtask/Cargo.toml'] = y.BODIES['crates/xtask/Cargo.toml'][0]
        self.assertEqual(result, expected)
        self.assertEqual(original, before)
        self.assertEqual(calls, ['yaml:Cargo.lock','swagger:Cargo.lock','sys',
                                 'swagger:Cargo.toml','yaml:crates/xtask/Cargo.toml'])

    def test_bad_original_catalog_and_reparse_refuse_before_read(self):
        for name in ('Cargo.lock','Cargo.toml','crates/xtask/Cargo.toml'):
            bad = data(); bad['auxiliary'][name] = 'unknown'
            with patch.object(Path, 'read_bytes', side_effect=AssertionError('catalog')), self.assertRaises(ValueError):
                d.reviewed_inputs(ROOT, bad)
        with patch.object(Path, 'is_junction', return_value=True), \
             patch.object(Path, 'read_bytes', side_effect=AssertionError('junction')), self.assertRaises(ValueError):
            d.reviewed_inputs(ROOT, data())

    def test_original_swagger_override_refuses_before_disk(self):
        for name in ('SWAGGER_UI_DOWNLOAD_URL','SWAGGER_UI_OVERWRITE_FOLDER','SWAGGER_UI_DOWNLOAD_CACHE_DIR'):
            with patch.dict(os.environ, {name:'unknown'}), \
                 patch.object(Path, 'read_bytes', side_effect=AssertionError('override')), self.assertRaises(ValueError):
                d.reviewed_inputs(ROOT, data())

    def test_old_versioned_apis_still_refuse_new_root_inputs(self):
        with self.assertRaises(ValueError):
            y.predecessor('Cargo.lock', (ROOT/'Cargo.lock').read_bytes())
        with self.assertRaises(ValueError):
            s.predecessor('Cargo.toml', (ROOT/'Cargo.toml').read_bytes())


class VendorCustody(unittest.TestCase):
    def test_status_retains_every_original_path_and_adds_vendor(self):
        self.assertEqual(custody.STATUS_PATHS, ('Cargo.toml','Cargo.lock','deny.toml','.cargo',
            'crates','tools','scripts','console','.github','.gitattributes','third_party'))

    def test_source_order_and_all_old_prefixes_preserved_with_vendor(self):
        names = ['tools/tool.py','third_party/hc074-dbus/zbus/src/lib.rs','Cargo.lock',
            'crates/a/lib.rs','scripts/x.py','console/src/x.ts','.cargo/config.toml',
            '.github/workflows/ci.yml','docs/plans/V0_74.md','docs/COMPAT.md','docs/RULES.md',
            'docs/GATES.md','.gitattributes','Cargo.toml','deny.toml',
            'docs/testing/performance/0.74/qualification-manifest.toml']
        self.assertEqual(custody.selected_sources(names), names)
        self.assertEqual(custody.selected_sources(['target/binary.exe','console/node_modules/x',
            'tools/node_modules/x','unrelated.txt','docs/testing/unrelated.md']), [])
