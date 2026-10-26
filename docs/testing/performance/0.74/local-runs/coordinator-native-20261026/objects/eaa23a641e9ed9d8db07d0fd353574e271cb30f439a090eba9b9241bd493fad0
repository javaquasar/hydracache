"""Offline graph, warning-owner and fixed-loop proof controls, no producer run."""
import copy
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch

import contract_frontier_audit as audit

ROOT=Path(__file__).resolve().parents[2]


def fixture():
    return dict(sources=[dict(source='root.rs',sha256='data-only',sites=[
        dict(kind='function-attribute',scope='root_case',detail='test'),
        dict(kind='module',scope='nested',detail='literal:child.rs')]),
        dict(source='child.rs',sha256='data-only',sites=[
            dict(kind='function-attribute',scope='tests::child_case',detail='test')])])


class ContractFrontier(unittest.TestCase):
    def test_literal_graph_qualifies_without_running_or_evaluating_cfg(self):
        result=audit.case_catalog(fixture(),'root.rs')
        self.assertEqual(set(result['cases']),{'root_case','nested::tests::child_case'})
        self.assertFalse(result['cfg_evaluated']);self.assertFalse(result['compiler_expanded'])

    def test_missing_ambiguous_and_cyclic_modules_refuse(self):
        for mode in ('missing','ambiguous','cycle'):
            doc=fixture()
            if mode=='missing':doc['sources'].pop()
            elif mode=='cycle':doc['sources'][1]['sites'].append(dict(kind='module',scope='back',detail='literal:root.rs'))
            else:
                doc['sources'][0]['sites'][1]['detail']='default-module-path'
                for path in ('root/nested.rs','root/nested/mod.rs'):
                    doc['sources'].append(dict(source=path,sha256='data-only',sites=[]))
            with self.assertRaises(ValueError):audit.case_catalog(doc,'root.rs')

    def test_unsafe_module_unknown_edge_and_ignored_cases_refuse(self):
        for detail in ('literal:../escape.rs','literal:C:/file.rs','literal:/file.rs','computed'):
            doc=fixture();doc['sources'][0]['sites'][1]['detail']=detail
            with self.assertRaises(ValueError):audit.case_catalog(doc,'root.rs')
        doc=fixture();doc['sources'][0]['sites'].append(dict(kind='function-attribute',scope='root_case',detail='ignore'))
        with self.assertRaisesRegex(ValueError,'ignored case'):audit.case_catalog(doc,'root.rs')

    def test_duplicate_source_or_qualified_case_refuses(self):
        doc=fixture();doc['sources'].append(doc['sources'][0])
        with self.assertRaisesRegex(ValueError,'duplicate AST'):audit.case_catalog(doc,'root.rs')
        doc=fixture();doc['sources'][0]['sites'].append(doc['sources'][0]['sites'][0])
        with self.assertRaisesRegex(ValueError,'duplicate qualified'):audit.case_catalog(doc,'root.rs')

    def test_warning_is_owned_by_completed_target_not_incomplete_successor(self):
        stdout='running 1 test\ntest prior has been running for over 60 seconds\ntest prior ... ok\nend\nrunning 1 test\n'
        report=dict(targets=[dict(target='prior.rs',start_line=1,end_line=4),dict(target='unfinished.rs',start_line=5)])
        result=audit.warning_owners(stdout,report)
        self.assertEqual(result,[dict(case='prior',target='prior.rs',line=2,diagnostic_seconds=60,per_case_duration_measured=False)])
        report['targets'][0]['end_line']=1
        with self.assertRaisesRegex(ValueError,'warning outside'):audit.warning_owners(stdout,report)

    def test_selected_loop_body_helper_and_macro_scopes_are_independent(self):
        folder=ROOT/'docs/testing/performance/0.74/local-runs'/audit.DATASETS[0][0]
        index=audit.review.decode_json((folder/'index.json').read_bytes())
        doc=audit.review.decode_json(audit.review.member(folder,index,'syntax/final-closure/result.json'))
        spec=audit.SWEEPS[0];path='crates/xtask/tests/support/'+spec[0]
        raw=audit.review.member(folder,index,'reviewed-source/'+path)
        row=next(v for v in doc['sources'] if v['source']==path)
        self.assertEqual(audit.sweep_proof(raw,row,spec)['source_required_direct_show_calls'],426)
        for old,new in [('count(), 423','count(), 422'),('docs/COMPAT.md','docs/OTHER.md'),('.args(args)','.args(&[])'),
                        ('normalize(path, &read(path))','normalize(path, &read("other"))')]:
            changed=raw.replace(old.encode(),new.encode());self.assertNotEqual(changed,raw)
            with self.assertRaisesRegex(ValueError,'source drift'):audit.sweep_proof(changed,row,spec)
            # Even a synthetic changed-body hash cannot bypass the literal
            # loop/tail/helper anchors in this model-level negative control.
            modified={**row,'sha256':audit.review.sha(changed)}
            with self.assertRaises(ValueError):audit.sweep_proof(changed,modified,spec)
        modified=copy.deepcopy(row)
        for site in modified['sites']:
            if site['scope']==spec[1]+'::macro:assert_eq':site['scope']=spec[1]
        with self.assertRaisesRegex(ValueError,'call sites'):audit.sweep_proof(raw,modified,spec)

    def test_saved_runs_have_same_partial_frontier_not_complete_package(self):
        with patch.object(subprocess,'Popen',side_effect=AssertionError('no new process')):
            result=audit.analyze(ROOT)
        self.assertEqual(len(result['source_case_catalog']),321)
        self.assertTrue(result['case_catalogs_equal']);self.assertTrue(result['observed_case_sets_equal'])
        for run in result['runs']:
            self.assertEqual(len(run['observed_passing_cases']),171)
            self.assertEqual(len(run['missing_case_verdicts']),150)
            self.assertEqual(run['source_required_selected_direct_show_calls_lower_bound'],1264)
            self.assertEqual(run['unfinished_target_warning_count'],0)
            self.assertFalse(run['missing_verdict_is_started_or_unstarted_known'])
            self.assertFalse(run['package_pass']);self.assertFalse(run['phase_cost_attributed'])
        self.assertEqual([r['source_files'] for r in result['runs']],[114,115])
        self.assertFalse(result['producer_repeated']);self.assertFalse(result['subprocesses_executed'])

    def test_changed_packet_identity_refuses_before_reading_any_transcript(self):
        with patch.object(audit.review,'sha',return_value='changed'):
            with self.assertRaisesRegex(ValueError,'sealed package identity'):audit.analyze(ROOT)
