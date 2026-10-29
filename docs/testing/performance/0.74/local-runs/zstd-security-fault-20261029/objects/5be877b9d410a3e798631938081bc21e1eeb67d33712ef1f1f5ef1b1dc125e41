"""Offline localization of observed polling counts; never normalizes runtime logs."""
import copy
from pathlib import Path
import unittest
import review_local_faults as review


def signature(phase):
    return dict(phase=phase['phase'],auth=phase['authentication_calls'],signatures=phase['signature_calls'],
        live=[n['calls'] for n in phase['live']],steps=[dict(calls=s['calls'],completed=s['completed'],
            auth=s['authentication_calls'],signatures=s['signature_calls'],kernel=[n['calls'] for n in s['kernel']],
            live=[n['calls'] for n in s['live']]) for s in phase['steps']])


def analyze(facts):
    require=review.prior.require
    require(facts['remaining_fixture_cases_complete'] is True,'Cannot pair an incomplete fixture cohort')
    pairs=[]
    for case in review.method.CASES:
        pair=[p for p in facts['profiles'] if p['case']==case]
        require(len(pair)==2 and [p['lane'] for p in pair]==['194','188'], 'Pair ordering changed')
        left,right=pair
        require(left['original']['causal']['observations']==right['original']['causal']['observations']
                and [s['calls'] for s in left['original']['age']['sites']]
                == [s['calls'] for s in right['original']['age']['sites']], 'Original age call counts differ')
        require(len(left['phases'])==len(right['phases']), 'Phase count differs')
        differences=[]
        for a,b in zip(left['phases'],right['phases']):
            require(signature(a)==signature(b),'Original validation round/auth/live call counts differ')
            delta=[y['calls']-x['calls'] for x,y in zip(a['kernel'],b['kernel'])]
            if any(delta):
                require(a['phase']=='accept-consent','Difference outside the receiving phase')
                n=delta[7]
                # Origin::sample calls revalidate twice; each revalidate checks
                # two Boot/four Namespace calls; each Boot performs one Document.
                require(n and delta==[0,0,0,0,2*n,4*n,8*n,n,0,4*n], 'Not the original standalone Sample call signature')
                outside=[]
                for i in range(len(delta)):
                    x=a['kernel'][i]['calls']-sum(s['kernel'][i]['calls'] for s in a['steps'])
                    y=b['kernel'][i]['calls']-sum(s['kernel'][i]['calls'] for s in b['steps'])
                    require(x>=0 and y>=0,'Outside-round counts underflow')
                    outside.append(y-x)
                require(outside==delta,'Kernel difference entered original validation rounds')
                differences.append(dict(phase=a['phase'],msrv_minus_gnu_kernel_calls=delta,
                    outside_round_call_delta=outside,inferred_additional_standalone_samples=n))
        pairs.append(dict(case=case,validation_step_auth_live_counts_equal=True,
            full_phase_kernel_counts_equal=not differences,differences=differences))
    return dict(schema='original-held-fault-call-localization-074-v1',pairs=pairs,
        full_paired_phase_kernel_counts_equal=all(p['full_phase_kernel_counts_equal'] for p in pairs),
        validation_step_auth_live_counts_equal=True,
        interpretation='Outside-round delta matches the unchanged bounded empty-receive polling path; inference, not recvmsg attempt telemetry',
        raw_counts_normalized=False,timing_overhead_subtracted=False,guard_changed=False,
        original_delivery_proven=False,all_production_writers_proven=False,full_sigkill_matrix=False,
        native_gate_open=False,server=False,product=False)


class Controls(unittest.TestCase):
    def test_retained_counts_and_falsifications(self):
        root=Path(__file__).resolve().parents[2]/'docs/testing/performance/0.74/local-runs'/review.method.NAME
        facts=review.prior.load(root/'summary.json')
        report=analyze(facts)
        self.assertFalse(report['full_paired_phase_kernel_counts_equal'])
        self.assertTrue(report['validation_step_auth_live_counts_equal'])
        for kind in ('step','origin','phase','live','auth','age','incomplete'):
            altered=copy.deepcopy(facts);profile=altered['profiles'][0]
            if kind=='step':profile['phases'][1]['steps'][0]['kernel'][4]['calls']+=1
            if kind=='origin':profile['phases'][1]['kernel'][4]['calls']+=1
            if kind=='phase':profile['phases'][0]['kernel'][7]['calls']+=1
            if kind=='live':profile['phases'][1]['live'][0]['calls']+=1
            if kind=='auth':profile['phases'][1]['authentication_calls']+=1
            if kind=='age':profile['original']['causal']['observations']+=1
            if kind=='incomplete':altered['remaining_fixture_cases_complete']=False
            with self.subTest(mutation=kind),self.assertRaises(ValueError):analyze(altered)


if __name__=='__main__':
    import argparse
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--test',action='store_true')
    parser.add_argument('--output',type=Path)
    parser.add_argument('--verify',action='store_true')
    args=parser.parse_args()
    if args.test:unittest.main(argv=['audit_fault_call_counts'],verbosity=2)
    else:
        if args.output is None:parser.error('--output required')
        root=Path(__file__).resolve().parents[2]/'docs/testing/performance/0.74/local-runs'/review.method.NAME
        review.prior.publish(args.output,analyze(review.prior.load(root/'summary.json')),args.verify)
        print('Raw full-phase inequality retained; original validation round/auth/live counts agree')
