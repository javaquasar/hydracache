"""Offline success-age and cohort-call controls, separate from measured method."""
import copy
import json
from pathlib import Path
import unittest
from unittest.mock import patch
import review_preparation_impact as review


def validate(facts):
    require=review.prior.require
    require(facts['complete_profile_cohort'] is True and len(facts['profiles'])==6,
            'This success audit requires the complete cohort')
    boot=facts['metadata']['boot']
    signatures=[];ages=[]
    def event(row):
        require(row['diagnostic_reason']=='within-window'
                and row['issued_boot']==row['now_boot']==boot, 'Boot/reason drift')
        require(type(row['age_ns']) is int and row['now_ns']-row['issued_ns']==row['age_ns']
                and 0<=row['age_ns']<10_000_000_000, 'Original ten-second window exceeded')
    for profile in facts['profiles']:
        original=profile['original'];causal=original['causal']
        require(profile['complete_preparation'] is True and original['kernel'] is None,
                'Not a healthy kernel outcome')
        require(causal['observations']==155 and causal['first_refusal'] is None,
                'Causal observation/refusal drift')
        for point in ('first','last'):event(causal[point])
        require(causal['first']['issued_ns']==causal['last']['issued_ns'], 'Admission origin changed')
        sites=original['age']['sites']
        require([s['calls'] for s in sites]==[184,81,78], 'Live age call signature drift')
        highest=0
        for site in sites:
            require(site['first_refusal'] is None,'Hidden live age refusal')
            for point in ('first','last'):
                row=site[point]
                require(row['reason']=='within-window' and type(row['age_ns']) is int
                        and row['now_ns']-row['issued_ns']==row['age_ns']
                        and 0<=row['age_ns']<10_000_000_000
                        and row['issued_ns']==causal['first']['issued_ns'], 'Live age/origin drift')
                highest=max(highest,row['age_ns'])
        ages.append(dict(lane=profile['lane'],sample=profile['sample'],
                         last_causal_age_ns=causal['last']['age_ns'],highest_reported_site_age_ns=highest))
        signature=[]
        for phase in profile['phases']:
            signature.append(dict(phase=phase['phase'],authentication_calls=phase['authentication_calls'],
                signature_calls=phase['signature_calls'],kernel=[r['calls'] for r in phase['kernel']],
                live=[r['calls'] for r in phase['live']],steps=[dict(calls=s['calls'],completed=s['completed'],
                    authentication_calls=s['authentication_calls'],signature_calls=s['signature_calls'],
                    kernel=[r['calls'] for r in s['kernel']],live=[r['calls'] for r in s['live']]) for s in phase['steps']]))
        signatures.append(signature)
    require(all(s==signatures[0] for s in signatures),'Cohort call graph changed')
    return dict(ages=ages,cohort_call_graph_equal=True,live_age_calls=[184,81,78],causal_observations=155,
                original_windows_seconds=[300,60,10],overhead_subtracted=False,native_gate_open=False,
                server=False,product=False)


class Controls(unittest.TestCase):
    def test_real_success_and_mutations(self):
        root=Path(__file__).resolve().parents[2]/'docs/testing/performance/0.74/local-runs'/review.NAME
        facts=review.prior.load(root/'summary.json')
        self.assertTrue(validate(facts)['cohort_call_graph_equal'])
        for name in ('boot','age','origin','calls','refusal','graph','completion'):
            data=copy.deepcopy(facts);p=data['profiles'][0];o=p['original']
            if name=='boot':o['causal']['last']['now_boot']='other'
            if name=='age':o['age']['sites'][0]['last']['age_ns']=10_000_000_000
            if name=='origin':o['age']['sites'][0]['last']['issued_ns']+=1
            if name=='calls':o['causal']['observations']-=1
            if name=='refusal':o['age']['sites'][0]['first_refusal']={}
            if name=='graph':p['phases'][0]['kernel'][0]['calls']+=1
            if name=='completion':data['complete_profile_cohort']=False
            with self.subTest(name=name),self.assertRaises(ValueError):validate(data)


if __name__=='__main__':
    import argparse
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--test',action='store_true')
    parser.add_argument('--output',type=Path)
    parser.add_argument('--verify',action='store_true')
    args=parser.parse_args()
    if args.test:unittest.main(argv=['audit_preparation_impact'],verbosity=2)
    else:
        parser.error('--output required') if args.output is None else None
        root=Path(__file__).resolve().parents[2]/'docs/testing/performance/0.74/local-runs'/review.NAME
        review.prior.publish(args.output,validate(review.prior.load(root/'summary.json')),args.verify)
        print('Original ages, live calls and complete cohort graph verified offline')
