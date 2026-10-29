"""Inert refusal/accounting controls; never call an unsafe old native API."""
from pathlib import Path
import os
import unittest
from unittest.mock import patch

import common as c
import run as r


class Controls(unittest.TestCase):
    def test_complete_probe_catalog_and_all_foreign_missing_duplicate_refusals(self):
        text='\n'.join(n+': test' for n in c.PROBE_TESTS)+'\n\n8 tests, 0 benchmarks\n'
        self.assertEqual(c.inventory(text,'probe'),c.PROBE_TESTS)
        for bad in ('',text.replace(c.PROBE_TESTS[0],c.PROBE_TESTS[1]),text.replace('8 tests','7 tests'),text.replace(c.PROBE_TESTS[0],'foreign')):
            with self.assertRaises(ValueError):c.inventory(bad,'probe')
        with self.assertRaises(ValueError):c.inventory(text,'unknown')

    def test_upstream_requires_original_security_cases(self):
        names=sorted(c.UPSTREAM_REQUIRED)
        raw='\n'.join(n+': test' for n in names)+'\n\n4 tests, 0 benchmarks\n'
        self.assertEqual(c.inventory(raw,'upstream'),names)
        with self.assertRaises(ValueError):c.inventory(raw.replace(names[0],'tests::test_foreign'),'upstream')

    def test_filtered_ignored_failed_zero_duplicate_and_missing_totals_refuse(self):
        ok='test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;'
        self.assertEqual(c.accounting(ok,c.PROBE_TESTS)['passed'],8)
        for bad in ('',ok+ok,ok.replace('8 passed','0 passed'),ok.replace('0 ignored','1 ignored'),
                    ok.replace('0 failed','1 failed'),ok.replace('0 filtered out','1 filtered out')):
            with self.assertRaises(ValueError):c.accounting(bad,c.PROBE_TESTS)

    def test_fixed_profile_argv_and_no_unrequested_dependency_features(self):
        for kind in ('probe','upstream'):
            for action in ('test','check','clippy'):
                argv,target=r.selected(kind,action,Path('held-tool'),'.exe','194')
                self.assertIn('--locked',argv);self.assertIn('--offline',argv)
                if kind=='upstream':self.assertIn('std,seekable',argv);self.assertNotIn('--all-features',argv)
                elif action!='test':self.assertIn('--all-features',argv);self.assertIn('--all-targets',argv)
                if action=='clippy':self.assertEqual(argv[-3:],['--','-D','warnings'])
                self.assertEqual(target,c.BASE/'targets'/(kind+'-194'))
        with self.assertRaises(ValueError):r.selected('unknown','test',Path('tool'),'','194')

    def test_frozen_fixture_code_lock_and_probe_changes_refuse(self):
        original=c.source();c.frozen_inputs()
        names=['tools/zstd-security-fault-074/probe.rs','target/zstd-security-fault-20261029/fixtures/upstream/Cargo.toml',
               'target/zstd-security-fault-20261029/fixtures/probe/Cargo.lock']
        for name in names:
            altered=dict(original);altered[name]='0'*64
            with patch.object(c,'source',return_value=altered),self.assertRaises(ValueError):c.frozen_inputs()
        with patch.object(c,'source',return_value={**original,'target/zstd-security-fault-20261029/fixtures/probe/extra.rs':'new'}),self.assertRaises(ValueError):c.frozen_inputs()

    def test_root_and_original_owner_pins_cannot_silently_advance(self):
        with patch.dict(c.PINNED,{'Cargo.lock':'0'*64}),self.assertRaises(ValueError):c.source()

    def test_private_vendor_configuration_changes_refuse(self):
        c.vendor_configuration()
        with patch.object(Path,'read_text',return_value='unreviewed source'),self.assertRaises(ValueError):c.vendor_configuration()

    def test_native_build_and_compiler_overrides_refuse(self):
        for name in ('RUSTFLAGS','CC','CFLAGS_x86_64_pc_windows_msvc','PKG_CONFIG_PATH','ZSTD_SYS_USE_PKG_CONFIG','CARGO_PROFILE_DEV_OPT_LEVEL','RUSTC_WRAPPER'):
            with patch.dict(os.environ,{name:'changed'}),self.assertRaises(ValueError):c.environment_review()

    def test_standalone_policy_keeps_root_configuration_and_all_graph_dimensions(self):
        for kind in ('probe','upstream','probe-licensed'):
            argv=r.policy_command(kind,Path('tool'),'.exe')
            for value in ('--offline','--locked','--all-features'):self.assertIn(value,argv)
            self.assertEqual(argv[-2:],['--config',str(c.ROOT/'deny.toml')])
            self.assertFalse(any('exclude' in value or 'ignore' in value for value in argv))
        with self.assertRaises(ValueError):r.policy_command('foreign',Path('tool'),'')

    def test_license_successor_changes_metadata_only_and_refuses_source_drift(self):
        c.license_guard()
        old=Path.read_bytes
        def changed(path):
            body=old(path)
            return body+b'changed' if 'policy-fixtures' in path.parts and path.name=='Cargo.lock' else body
        with patch.object(Path,'read_bytes',changed),self.assertRaises(ValueError):c.license_guard()
