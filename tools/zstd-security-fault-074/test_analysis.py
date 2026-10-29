"""Missing rows and disabled vulnerable feature paths cannot appear green."""
import json
import unittest

import analyze as a
import common as c


class AnalysisControls(unittest.TestCase):
    def test_actual_offsets_and_missing_changed_reordered_rows_refuse(self):
        raw=(c.BASE/'windows-194/probe-test/stdout.log').read_text(encoding='utf8')
        self.assertEqual(len(a.offsets(raw)),64)
        for bad in (raw.replace('HC074-ZSTD-OFFSET','missing',1),raw.replace('written=0','written=1',1),raw+raw):
            with self.assertRaises(ValueError):a.offsets(bad)

    def test_actual_compiled_feature_profile(self):
        raw=(c.BASE/'windows-194/probe-build/stdout.log').read_text(encoding='utf8')
        actual=a.compiled_features(raw,'probe');self.assertIn('legacy',actual['zstd_sys'])

    def test_disabled_vulnerable_features_and_optional_activation_refuse(self):
        raw=(c.BASE/'windows-194/probe-build/stdout.log').read_text(encoding='utf8')
        for alteration in ('remove-legacy','remove-seekable','bindgen','cmake','with-rust-allocator'):
            rows=[json.loads(v) for v in raw.splitlines()]
            for row in rows:
                if row.get('reason')=='compiler-artifact' and row.get('target',{}).get('name')=='zstd_safe':
                    if alteration.startswith('remove-'):row['features']=[v for v in row['features'] if v!=alteration.removeprefix('remove-')]
                    else:row['features'].append(alteration)
            with self.subTest(alteration=alteration),self.assertRaises(ValueError):a.compiled_features('\n'.join(json.dumps(v) for v in rows),'probe')

    def test_missing_compiled_dependency_refuses(self):
        with self.assertRaises(ValueError):a.compiled_features('', 'probe')
