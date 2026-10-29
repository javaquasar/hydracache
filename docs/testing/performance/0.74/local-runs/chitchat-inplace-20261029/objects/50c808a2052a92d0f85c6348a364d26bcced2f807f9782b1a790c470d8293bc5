"""Exact one-node inverse, inherited read order and old-API refusal."""
import copy
import os
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch

import dbus_inputs as d
import markdown_inputs as m
import security_lock
import swagger_inputs as s
import yaml_inputs as y
from test_yaml_inputs import data

ROOT=Path(__file__).resolve().parents[2]


class MarkdownInputs(unittest.TestCase):
    def test_exact_complete_predecessor_and_original_crlf_normalization(self):
        raw=(ROOT/'Cargo.lock').read_bytes().replace(b'\r\n',b'\n')
        self.assertEqual(s.sha(m.predecessor(raw)),d.BODIES['Cargo.lock'][0])
        self.assertEqual(m.predecessor(raw),m.predecessor(raw.replace(b'\n',b'\r\n')))
        self.assertEqual(s.sha(d.predecessor('Cargo.lock',m.predecessor(raw))),y.BODIES['Cargo.lock'][0])

    def test_unreviewed_whole_input_and_incorrect_original_pin_refuse(self):
        raw=(ROOT/'Cargo.lock').read_bytes()
        for bad in (b'unknown',raw+b'\n',raw.replace(m.NEW,m.OLD),raw.replace(b'2.13.2',b'2.13.1')):
            with self.subTest(size=len(bad)),self.assertRaises(ValueError):m.predecessor(bad)
        with patch.object(m,'PREVIOUS','0'*64),self.assertRaises(ValueError):m.predecessor(raw)

    def test_one_exact_stanza_and_no_other_model_delta(self):
        import tomllib
        raw=(ROOT/'Cargo.lock').read_bytes();prior=m.predecessor(raw)
        before,after=tomllib.loads(prior.decode()),tomllib.loads(raw.decode())
        self.assertEqual(len(before['package']),len(after['package']))
        delta=[(a,b) for a,b in zip(before['package'],after['package']) if a!=b]
        self.assertEqual(len(delta),1)
        a,b=delta[0];self.assertEqual(a['name'],b['name']);self.assertEqual(a['name'],'pulldown-cmark')
        self.assertEqual({k for k in a.keys()|b.keys() if a.get(k)!=b.get(k)},{'version','checksum'})

    def test_original_versioned_apis_do_not_accept_the_new_input(self):
        raw=(ROOT/'Cargo.lock').read_bytes()
        for module in (d,y,s):
            with self.subTest(module=module.__name__),self.assertRaises(ValueError):module.predecessor('Cargo.lock',raw)

    def test_same_three_conditional_reads_and_all_old_chains_without_git(self):
        original=data();before=copy.deepcopy(original);reads=[];calls=[];actual=Path.read_bytes
        def observe(path):
            if path in [ROOT/n for n in ('Cargo.lock','Cargo.toml','crates/xtask/Cargo.toml')]:reads.append(path.relative_to(ROOT).as_posix())
            return actual(path)
        def traced(label,fn):
            def run(*args):
                calls.append(label+(':'+args[0] if label!='sys' else ''));return fn(*args)
            return run
        with patch.object(Path,'read_bytes',observe),patch.object(subprocess,'check_output',side_effect=AssertionError('no extra Git')),\
             patch.object(d,'predecessor',side_effect=traced('dbus',d.predecessor)),\
             patch.object(y,'predecessor',side_effect=traced('yaml',y.predecessor)),\
             patch.object(s,'predecessor',side_effect=traced('swagger',s.predecessor)),\
             patch.object(security_lock,'predecessor',side_effect=traced('sys',security_lock.predecessor)):
            result=m.reviewed_inputs(ROOT,original)
        expected=copy.deepcopy(before);expected['auxiliary'].update({'Cargo.lock':m.CURRENT,
            'Cargo.toml':d.BODIES['Cargo.toml'][0],'crates/xtask/Cargo.toml':y.BODIES['crates/xtask/Cargo.toml'][0]})
        self.assertEqual(original,before);self.assertEqual(result,expected)
        self.assertEqual(reads,['Cargo.lock','Cargo.toml','crates/xtask/Cargo.toml'])
        self.assertEqual(calls,['dbus:Cargo.lock','yaml:Cargo.lock','swagger:Cargo.lock','sys',
                               'dbus:Cargo.toml','swagger:Cargo.toml','yaml:crates/xtask/Cargo.toml'])

    def test_bad_catalog_refuses_before_reads(self):
        for name in ('Cargo.lock','Cargo.toml','crates/xtask/Cargo.toml'):
            original=data();original['auxiliary'][name]='unknown'
            with patch.object(Path,'read_bytes',side_effect=AssertionError('catalog')),self.assertRaises(ValueError):m.reviewed_inputs(ROOT,original)

    def test_reparse_and_build_environment_override_refuse_before_reads(self):
        with patch.object(Path,'is_junction',return_value=True),patch.object(Path,'read_bytes',side_effect=AssertionError('junction')),self.assertRaises(ValueError):m.reviewed_inputs(ROOT,data())
        with patch.dict(os.environ,{'SWAGGER_UI_DOWNLOAD_URL':'unknown'}),patch.object(Path,'read_bytes',side_effect=AssertionError('override')),self.assertRaises(ValueError):m.reviewed_inputs(ROOT,data())
