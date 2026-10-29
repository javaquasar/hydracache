"""Deterministic scheduler and offline fault result falsification controls."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import runner
import run_local_faults as fault
import review_local_faults as review


class SessionTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        self.output=Path(self.temp.name)/'packet'
        self.pins=dict(source='a'*40,bundle=[1],images={'194':'b','188':'c'})
        self.context=dict(source=self.pins['source'],bundle=[1],images=self.pins['images'],tracked_dirty='',uid=0,
            fixtures=[],boot='aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee',qualification_sha256=runner.QUALIFICATION,
            pid=74,start_ticks='19',config={'original':'same'})
        self.calls=[]

    def observe(self):return copy.deepcopy(self.context)

    def execute(self,lane,case,log,deadline):
        self.calls.append([lane,case]);Path(log).write_bytes(b'offline synthetic control\n')
        return dict(exit=0)

    def test_exact_eight_cases_once(self):
        result=fault.run_session(self.output,self.pins,self.observe,self.execute)
        self.assertEqual(self.calls,[[l,c] for l in runner.LANES for c in fault.CASES])
        self.assertTrue(result['schedule_completed']);self.assertFalse(result['full_matrix'])
        intent=runner.read_json(self.output/'intent.json')
        self.assertEqual(intent['original_windows_seconds'],[300,60,10])
        self.assertEqual(intent['session_seconds'],7500)

    def test_unexpected_refusal_stops_lane_without_retry(self):
        def execute(*args):
            result=self.execute(*args)
            result['exit']=101 if args[0]=='194' else 0
            return result
        result=fault.run_session(self.output,self.pins,self.observe,execute)
        self.assertEqual(self.calls,[['194','revalidate']]+[['188',c] for c in fault.CASES])
        self.assertFalse(result['schedule_completed'])
        self.assertFalse((self.output/'194-role-before.log').exists())

    def test_context_drift_stops_all(self):
        def observe():
            result=self.observe()
            if self.calls:result['boot']='bbbbbbbb-bbbb-cccc-dddd-eeeeeeeeeeee'
            return result
        result=fault.run_session(self.output,self.pins,observe,self.execute)
        self.assertEqual(len(self.calls),1);self.assertIsNotNone(result['failure'])

    def test_existing_packet_never_resumed(self):
        fault.run_session(self.output,self.pins,self.observe,self.execute)
        with self.assertRaises(FileExistsError):fault.run_session(self.output,self.pins,self.observe,self.execute)
        self.assertEqual(len(self.calls),8)

    def test_precheck_refusal_keeps_postflight_no_children(self):
        def prepare():raise RuntimeError('source drift')
        result=fault.run_session(self.output,self.pins,self.observe,self.execute,prepare=prepare)
        self.assertEqual(self.calls,[]);self.assertIsNotNone(result['failure'])
        self.assertTrue((self.output/'postflight.json').exists())

    def test_budget_refuses_dispatch_not_extended(self):
        times=iter([0,7500])
        result=fault.run_session(self.output,self.pins,self.observe,self.execute,clock=lambda:next(times,7500))
        self.assertEqual(self.calls,[]);self.assertIn('budget',result['failure'])

    def test_interrupt_keeps_intent_and_postflight(self):
        def execute(*_):raise InterruptedError('lost')
        result=fault.run_session(self.output,self.pins,self.observe,execute)
        self.assertEqual(result['measured_profile_invocations'],1)
        self.assertTrue((self.output/'194-revalidate.intent.json').exists())
        self.assertTrue((self.output/'postflight.json').exists())

    def test_postflight_failure_never_green(self):
        n=0
        def observe():
            nonlocal n
            n+=1
            if n==18:raise RuntimeError('postflight')
            return self.observe()
        result=fault.run_session(self.output,self.pins,observe,self.execute)
        self.assertFalse(result['schedule_completed']);self.assertEqual(len(self.calls),8)

    def test_environment_only_changes_original_case(self):
        for case in fault.CASES:
            env=fault.child_env(Path('/fixture'),case)
            expected=runner.child_env(Path('/fixture'));expected['HC074_CONSUMING_HELD_LIVE_CASE']=case
            self.assertEqual(env,expected)
        for case in ('healthy','', 'sigkill','role-after;send'):
            with self.assertRaises(RuntimeError):fault.child_env(Path('/fixture'),case)

    def test_healthy_predecessor_bytes_membership_and_path_guard(self):
        repo=Path(self.temp.name)/'repo'
        prior=repo/'docs/testing/performance/0.74/local-runs'/fault.PRIOR
        prior.mkdir(parents=True)
        member=prior/'member.json';member.write_bytes(b'original')
        row=dict(path='member.json',bytes=8,sha256=runner.sha(member))
        inventory=prior/'inventory.json'
        runner.write_new(inventory,dict(files=[row]))
        real_sha=runner.sha
        def sha(path):return fault.PRIOR_SHA if Path(path)==inventory else real_sha(path)
        with patch.object(fault.preparation,'verify_preparation',return_value=None),patch.object(runner,'sha',side_effect=sha):
            fault.verify_preparation(repo,self.pins)
            member.write_bytes(b'changed!')
            with self.assertRaisesRegex(RuntimeError,'member drift'):fault.verify_preparation(repo,self.pins)
            member.write_bytes(b'original')
            extra=prior/'unexpected';extra.write_bytes(b'extra')
            with self.assertRaisesRegex(RuntimeError,'membership drift'):fault.verify_preparation(repo,self.pins)
            # Remove only this TemporaryDirectory-owned unit-control file.
            extra.unlink()
            for rows in ([row,row],[{**row,'path':'../escape'}]):
                inventory.write_text(json.dumps(dict(files=rows)),encoding='utf-8')
                with self.assertRaisesRegex(RuntimeError,'Unsafe/duplicate'):fault.verify_preparation(repo,self.pins)


class ReaderTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        self.log=Path(self.temp.name)/'control.log'
        root=Path(__file__).resolve().parents[2]/'docs/testing/performance/0.74/local-runs'/fault.PRIOR
        self.metadata=runner.read_json(root/'metadata.json')
        self.phases=review.prior.rows(root/'194-Profile-0.log','HC074-HELD-PHASE ')
        self.original=review.prior.rows(root/'194-Profile-0.log','HC074-HELD-LIVE-ORIGINAL ')[0]

    def encode(self,case,exit=0,footer=True):
        phases=copy.deepcopy(self.phases);original=copy.deepcopy(self.original);original['case']=case
        if case=='revalidate':
            extra=copy.deepcopy(phases[0]);extra['phase']='revalidate';phases.append(extra)
        if exit!=0:
            phases=phases[:5];phases[-1]['returned']=False;original['returned']=False
        text=''.join('HC074-HELD-PHASE '+json.dumps(r)+'\n' for r in phases)
        text+='HC074-HELD-LIVE-ORIGINAL '+json.dumps(original)+'\n'
        if footer:
            text+=(f'HC074-HELD-LIVE case={case} seed=0x074c051e actual_fixed=true actual_consuming=true '
                'original_bounds=true eof=true fence_retained=true custodians_terminal=true synthetic_images=true '
                'original_send=false completion=false server=false product=false\n')
        self.log.write_text(text,encoding='utf-8')
        return dict(case=case,lane='194',exit=exit,containment_failure=None,log_sha256=runner.sha(self.log),
                    bytes=self.log.stat().st_size,wall_seconds=7.0)

    def test_all_case_shapes_and_unexpected_original_failure(self):
        for case in fault.CASES:
            receipt=self.encode(case)
            facts=review.profile_facts(self.log,receipt,self.metadata)
            self.assertTrue(facts['harness_passed'])
            self.assertEqual(facts['expected_injected_rejection'],case!='revalidate')
        receipt=self.encode('role-after',101,False)
        self.assertFalse(review.profile_facts(self.log,receipt,self.metadata)['harness_passed'])

    def test_expected_rejection_does_not_accept_any_failure(self):
        for name in ('exit','raw','missing-footer','foreign-footer','duplicate-footer','scope','phase','age','causal','boot'):
            receipt=self.encode('trust-after')
            if name=='exit':receipt['exit']=False
            if name=='raw':receipt['log_sha256']='0'*64
            text=self.log.read_text()
            if name=='missing-footer':text='\n'.join(l for l in text.splitlines() if not l.startswith('HC074-HELD-LIVE '))
            if name=='foreign-footer':text=text.replace('seed=0x074c051e','seed=0x075c051e')
            if name=='duplicate-footer':text+=text.splitlines()[-1]+'\n'
            if name in ('scope','phase','age','causal','boot'):
                lines=text.splitlines()
                for i,line in enumerate(lines):
                    if line.startswith('HC074-HELD-LIVE-ORIGINAL '):
                        row=json.loads(line.split(' ',1)[1])
                        if name=='scope':row['deadline_changed']=True
                        if name=='age':row['age']['sites'][0]['last']['age_ns']=10_000_000_000
                        if name=='causal':row['causal']['first_refusal']={'reason':'expired'}
                        if name=='boot':row['causal']['last']['now_boot']='foreign'
                        lines[i]='HC074-HELD-LIVE-ORIGINAL '+json.dumps(row)
                if name=='phase':lines[0]=lines[0].replace('prepare-consent','foreign-phase')
                text='\n'.join(lines)+'\n'
            if name not in ('exit','raw'):
                self.log.write_text(text,encoding='utf-8');receipt.update(log_sha256=runner.sha(self.log),bytes=self.log.stat().st_size)
            with self.subTest(mutation=name),self.assertRaises(ValueError):review.profile_facts(self.log,receipt,self.metadata)


if __name__=='__main__':unittest.main(verbosity=2)
