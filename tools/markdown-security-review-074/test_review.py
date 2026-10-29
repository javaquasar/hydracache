"""Inert faults for fixed regression accounting and command custody."""
from pathlib import Path
import unittest
from unittest.mock import patch

import analyze as a
import consumer as c
import native as n
import run as w


class ReviewControls(unittest.TestCase):
    def test_complete_positive_and_all_missing_duplicate_growth_controls(self):
        raw=(w.BASE/'test-candidate-194/stdout.log').read_text()
        self.assertEqual(len(a.probe(raw,True)),8)
        for bad in (raw.replace('index=7','index=8'),raw.replace('repeats=64','repeats=32'),
                    raw.replace('count=65','count=66'),raw.replace('HC074-MD-CORPUS index=0','missing index=0')):
            with self.subTest(prefix=bad[:20]),self.assertRaises(ValueError):a.probe(bad,True)

    def test_original_baseline_oracle_is_not_green(self):
        raw=(w.BASE/'test-baseline-194/stdout.log').read_text()
        self.assertEqual(len(a.probe(raw,False)),8)
        with self.assertRaises(ValueError):a.probe(raw,True)
        with self.assertRaises(ValueError):a.probe(raw.replace('327676','65'),False)

    def test_target_identity_not_parallel_artifact_arrival_order(self):
        one=dict(target='a',cases=['a'],passed=1,ignored=0);two=dict(target='b',cases=[],passed=0,ignored=0)
        self.assertEqual(a.catalog([one,two]),a.catalog([two,one]))
        for bad in ([],[one,one],[dict(one,cases=['a','a'])],[dict(one,passed=0)]):
            with self.assertRaises(ValueError):a.catalog(bad)

    def test_actual_consumer_output_is_bound_to_both_cargo_producers(self):
        value=a.generated(w.BASE/'consumer-candidate-194','194')
        self.assertEqual(len(value['generated']),4)
        with patch.object(Path,'as_uri',return_value='file:///wrong-owner'),self.assertRaises(ValueError):
            a.generated(w.BASE/'consumer-candidate-194','194')
        with patch.object(Path,'read_bytes',return_value=b'changed'),self.assertRaises(ValueError):
            a.generated(w.BASE/'consumer-candidate-194','194')

    def test_fixed_real_consumer_argv_keeps_complete_check_and_clippy(self):
        for compiler in ('194','188'):
            for action in ('build','check','clippy'):
                argv,env=c.command(action,compiler)
                self.assertIn('--locked',argv);self.assertIn('--offline',argv)
                self.assertEqual(argv.count('-p'),2)
                for package in c.PACKAGES:self.assertIn(package,argv)
                if action in ('check','clippy'):self.assertIn('--all-targets',argv);self.assertIn('--all-features',argv)
                if action=='clippy':self.assertEqual(argv[-3:],['--','-D','warnings'])
                self.assertEqual(env['CARGO_INCREMENTAL'],'0')
        with self.assertRaises(KeyError):c.command('version','unknown')

    def test_actual_registry_and_codec_tampering_refuse(self):
        self.assertIsNotNone(w.registry_guard('candidate'))
        with patch.object(Path,'read_bytes',return_value=b'changed'),self.assertRaises(ValueError):n.codec()
        with patch.object(Path,'read_bytes',return_value=b'{}'),self.assertRaises(Exception):w.registry_guard('candidate')

    def test_native_registry_refresh_detects_changed_vendor_member(self):
        # Same supplied file graph; alter one decoded source, never bypass the fresh read.
        actual=Path.read_bytes
        def changed(path):
            if path.name=='registry.json':return b'{"crate":{"src/lib.rs":"wrong"}}'
            return b'changed'
        with patch.object(Path,'read_bytes',changed),self.assertRaises(ValueError):n.registry()

    def test_invalid_consumer_modes_refuse_before_output_creation(self):
        for mode,compiler in (('unknown','194'),('candidate','unknown')):
            with patch.object(Path,'mkdir',side_effect=AssertionError('no effects')),self.assertRaises(ValueError):c.main(mode,compiler)
