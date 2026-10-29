"""Offline controls for the successor; original runner tests remain unchanged."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import runner
import run_preparation_impact as impact
import review_preparation_impact as review


class ImpactTests(unittest.TestCase):
    def test_original_schedule_environment_and_images_are_reused(self):
        self.assertIs(impact.original, runner)
        self.assertEqual(runner.child_env(Path('/synthetic'))['HC074_CONSUMING_HELD_LIVE_CASE'], 'healthy')
        self.assertNotIn('fixed-leaf-alloc-probe', runner.command('194'))
        self.assertEqual(runner.command('194')[:4], ['/usr/bin/timeout','--signal=TERM','--kill-after=5s','1200s'])

    def test_real_historical_refusal_and_mutations(self):
        root = Path(__file__).resolve().parents[2]/'docs/testing/performance/0.74/local-runs'/review.prior.PACKET
        metadata = review.prior.load(root/'metadata.json')
        for lane in runner.LANES:
            receipt = review.prior.load(root/f'{lane}-Profile-0.exit.json')
            log = root/f'{lane}-Profile-0.log'
            facts = review.profile_facts(log,receipt,metadata)
            self.assertFalse(facts['complete_preparation'])
            for key,value in [('exit',0),('exit',False),('log_sha256','0'*64),('bytes',0),('containment_failure','lost')]:
                altered = {**receipt,key:value}
                with self.subTest(lane=lane,key=key), self.assertRaises(ValueError):
                    review.profile_facts(log,altered,metadata)
            original_rows = review.prior.rows
            for kind in ('phase-order','missing-original','false-return','changed-scope','boot','round-count'):
                def rows(path,marker):
                    data = copy.deepcopy(original_rows(path,marker))
                    if marker.endswith('PHASE '):
                        if kind=='phase-order': data.reverse()
                        if kind=='round-count': data[0]['round_steps'][0]['completed'] += 1
                    else:
                        if kind=='missing-original': return []
                        if kind=='false-return': data[0]['returned']=True
                        if kind=='changed-scope': data[0]['deadline_changed']=True
                        if kind=='boot': data[0]['causal']['first_refusal']['now_boot']='changed'
                    return data
                with self.subTest(lane=lane,mutation=kind), patch.object(review.prior,'rows',side_effect=rows), self.assertRaises(ValueError):
                    review.profile_facts(log,receipt,metadata)

    def test_source_guard_is_candidate_exact_not_old_runtime(self):
        with tempfile.TemporaryDirectory() as name:
            repo = Path(name)
            with patch.object(runner,'git',side_effect=RuntimeError('drift')) as git:
                with self.assertRaisesRegex(RuntimeError,'drift'):
                    impact.verify_preparation(repo,dict(source='a'*40))
                self.assertEqual(git.call_args.args[3],impact.CANDIDATE)

    def test_synthetic_complete_profile_and_partial_output(self):
        root=Path(__file__).resolve().parents[2]/'docs/testing/performance/0.74/local-runs'/review.prior.PACKET
        old=review.prior.rows(root/'194-Profile-0.log','HC074-HELD-PHASE ')
        phases=[]
        for index,phase in enumerate(review.prior.PHASES):
            row=copy.deepcopy(old[index] if index<5 else old[0])
            row['phase']=phase
            phases.append(row)
        original=dict(returned=True,causal=None,case='healthy',guard_clock_added=False,
                      deadline_changed=False,server=False,product=False)
        with tempfile.TemporaryDirectory() as name:
            log=Path(name)/'synthetic.log'
            text=''.join('HC074-HELD-PHASE '+json.dumps(r)+'\n' for r in phases)
            text+='HC074-HELD-LIVE-ORIGINAL '+json.dumps(original)+'\n'
            text+='HC074-HELD-LIVE case=healthy original_send=false completion=false server=false product=false\n'
            log.write_text(text,encoding='utf-8')
            receipt=dict(lane='194',sample=0,exit=0,containment_failure=None,
                         log_sha256=review.prior.sha(log),bytes=log.stat().st_size)
            self.assertTrue(review.profile_facts(log,receipt,{})['complete_preparation'])
            for missing in (0,5,9):
                with patch.object(review.prior,'rows',side_effect=lambda p,m:
                    [r for i,r in enumerate(phases) if i!=missing] if m.endswith('PHASE ') else [original]):
                    with self.assertRaises(ValueError): review.profile_facts(log,receipt,{})
            failed=copy.deepcopy(old[-1]);failed['phase']='outputs'
            original['returned']=False
            receipt['exit']=101
            with patch.object(review.prior,'rows',side_effect=lambda p,m:
                old[:2]+[failed] if m.endswith('PHASE ') else [original]):
                self.assertFalse(review.profile_facts(log,receipt,{})['complete_preparation'])

    def test_fresh_preparation_feature_and_log_guards(self):
        with tempfile.TemporaryDirectory() as name:
            repo=Path(name);root=repo/'docs/testing/performance/0.74/local-runs'
            latest=root/impact.PRIOR;latest.mkdir(parents=True)
            old=root/runner.PRIOR;old.mkdir()
            runner.write_new(latest/'inventory.json',dict(files=[]))
            runner.write_new(old/'inventory.json',dict(files=[]))
            runner.write_new(old/'context-Verified-Before.json',dict(bundle=[1]))
            pins=dict(source='a'*40,bundle=[1],images={l:'b'*64 for l in runner.LANES},preparation=str(repo/'fresh.json'))
            fresh={}
            for lane in runner.LANES:
                log=repo/f'{lane}.log'
                log.write_text(json.dumps(dict(reason='compiler-artifact',target=dict(name='hydracache_long_run_supervisor_074'),
                    profile=dict(test=True),features=[],executable=runner.IMAGES[lane]))+'\n')
                fresh[lane]=dict(source=pins['source'],exit=0,image=runner.IMAGES[lane],sha256=pins['images'][lane],
                    build_log=log.name,build_log_sha256=runner.sha(log))
            runner.write_new(pins['preparation'],fresh)
            realsha=runner.sha
            def sha(p):
                if Path(p)==latest/'inventory.json':return impact.PRIOR_SHA
                if Path(p)==old/'inventory.json':return runner.PRIOR_HASH
                return realsha(p)
            with patch.object(runner,'sha',side_effect=sha),patch.object(runner,'git',return_value=''):
                impact.verify_preparation(repo,pins)
                for key,value in [('source','bad'),('bundle',[]),('images',{})]:
                    with self.subTest(key=key),self.assertRaises(RuntimeError):
                        impact.verify_preparation(repo,{**pins,key:value})
                log=repo/'194.log';artifact=json.loads(log.read_text());artifact['features']=['fixed-leaf-alloc-probe']
                log.write_text(json.dumps(artifact)+'\n')
                with self.assertRaisesRegex(RuntimeError,'Build log drift'):impact.verify_preparation(repo,pins)
                fresh['194']['build_log_sha256']=realsha(log)
                Path(pins['preparation']).write_text(json.dumps(fresh))
                with self.assertRaisesRegex(RuntimeError,'default-feature'):impact.verify_preparation(repo,pins)


if __name__ == '__main__':
    unittest.main()
