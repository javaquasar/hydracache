"""Offline fixed allocation gate. No execution, retries or timing admission."""
import json
import math
from pathlib import Path
import statistics
import sys

T4=2.7764451051977987


def interval(ratios):
    if len(ratios)!=5 or any(not math.isfinite(r) or r<=0 for r in ratios):
        raise ValueError('exactly five positive independent pairs required')
    logs=[math.log(r) for r in ratios]
    center=statistics.mean(logs)
    half=T4*statistics.stdev(logs)/math.sqrt(5)
    return dict(paired_ratios=ratios,geometric_mean=math.exp(center),lower_95=math.exp(center-half),upper_95=math.exp(center+half))


def analyse(read, receipt_check=None):
    plan=json.loads(read('plan.json'))
    attempts=plan['attempts']
    if len(attempts)!=240 or plan['synthetic']:
        raise ValueError('not the sealed real 240-process stage')
    if not json.loads(read('restored.json'))['verified'] or not json.loads(read('all-writers-stopped.json'))['original_fence_acquired']:
        raise ValueError('host restoration/retirement not proven')
    if json.loads(read('cohort-complete.json'))['attempts']!=240:
        raise ValueError('partial cohort; no effect claim')
    groups={}
    cells=list(dict.fromkeys(a['cell'] for a in attempts))
    expected=[(mode,pair,cell,role) for mode in ['aa','ab'] for pair in range(1,6) for cell in cells for role in (['off','on'] if pair%2 else ['on','off'])]
    if [(a['mode'],a['pair'],a['cell'],a['role']) for a in attempts]!=expected:
        raise ValueError('counterbalanced schedule/order drift')
    for sequence,a in enumerate(attempts):
        receipt=json.loads(read(str(sequence)+'.stdout.log'))
        if receipt_check is not None:receipt_check(receipt,a)
        retired=json.loads(read(str(sequence)+'.retired.json'))
        if retired['exit'] or retired['all_original_writers_stopped'] is not True:
            raise ValueError('first child/retirement refusal')
        for key,value in a['receipt_pins'].items():
            if receipt.get(key)!=value:raise ValueError('receipt identity drift: '+key)
        report=receipt['report']
        if receipt['error'] is not None or report['error'] is not None or not report['final_dataset_verified'] or not report['shutdown_verified']:
            raise ValueError('invalid measured receipt')
        if report['get_owner_feature'] is not a['feature_enabled']:
            raise ValueError('compiled feature mismatch')
        expected_feature=a['mode']=='ab' and a['role']=='on'
        if a['feature_enabled'] is not expected_feature:raise ValueError('AA/AB feature drift')
        group=groups.setdefault((a['mode'],a['pair'],a['cell']),{})
        if a['role'] in group:raise ValueError('duplicate measured role')
        group[a['role']]=report['requested_layouts']['gross_allocated_bytes']
    if len(cells)!=12:raise ValueError('cell registry drift')
    results=[];red=[]
    for cell in cells:
        aa=[];ab=[]
        for mode,ratios in [('aa',aa),('ab',ab)]:
            for pair in range(1,6):
                g=groups[(mode,pair,cell)]
                if set(g)!={'off','on'} or min(g.values())<=0:raise ValueError('missing measured pair')
                ratios.append(g['on']/g['off'])
        if max(abs(r-1) for r in aa)>.01:
            raise ValueError('first AA gross noise refusal for '+cell+': '+repr(aa))
        affected=next(a['affected'] for a in attempts if a['cell']==cell)
        ceiling=.8 if affected else 1.05
        ci=interval(ab);passed=ci['upper_95']<=ceiling+1e-12
        if not passed:red.append(cell)
        results.append(dict(cell=cell,affected=affected,aa_ratios=aa,gross=ci,ceiling=ceiling,passed=passed))
    return dict(schema_version=1,profile_id='get-owner-full-d3-allocation-074-v2',attempts=240,
        classification='rejected-mandatory-full-d3-allocation-floor' if red else 'allocation-stage-passed-full-d3-not-complete',
        rejected_cells=red,results=results,candidate_admitted=False,timing_or_retention_proven=False,
        full_d3_terminal_rejection=bool(red),thresholds_changed=False)


if __name__=='__main__':
    folder=Path(sys.argv[1])
    print(json.dumps(analyse(lambda name:(folder/name).read_bytes()),indent=2))
