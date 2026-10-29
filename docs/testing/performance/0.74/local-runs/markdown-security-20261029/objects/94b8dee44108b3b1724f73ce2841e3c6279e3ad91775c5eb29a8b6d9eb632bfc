"""Offline original fault outcomes; expected rejection is not arbitrary failure."""
import argparse
import copy
from pathlib import Path
import attribute_partial as prior
import runner
import run_local_faults as method


def profile_facts(log, receipt, metadata):
    require = prior.require
    case = receipt['case']
    require(case in method.CASES and receipt['lane'] in runner.LANES, 'Unexpected fixture case/lane')
    require(type(receipt['exit']) is int and receipt['exit'] in (0,101)
            and receipt['containment_failure'] is None, 'Not an original harness result')
    require(receipt['log_sha256']==prior.sha(log) and receipt['bytes']==log.stat().st_size, 'Raw profile drift')
    rows = prior.rows(log,'HC074-HELD-PHASE ')
    originals = prior.rows(log,'HC074-HELD-LIVE-ORIGINAL ')
    expected = prior.PHASES + (('revalidate',) if case=='revalidate' else ())
    require(len(rows)<=len(expected) and tuple(r['phase'] for r in rows)==expected[:len(rows)], 'Phase order changed')
    require(len(originals)==1, 'Original result missing/duplicated')
    original = originals[0]
    prior.flags(original,false=('guard_clock_added','deadline_changed','server','product'))
    require(original['case']==case and type(original['returned']) is bool, 'Original case/result drift')
    passed = receipt['exit']==0
    require(original['returned'] is passed and all(r['returned'] is True for r in rows[:-1])
            and (not rows or rows[-1]['returned'] is passed), 'Exit does not join first original result')
    require(not passed or len(rows)==len(expected), 'Passing fixture missing phases')
    text = log.read_text(encoding='utf-8-sig')
    footer = (f'HC074-HELD-LIVE case={case} seed=0x074c051e actual_fixed=true actual_consuming=true '
        'original_bounds=true eof=true fence_retained=true custodians_terminal=true synthetic_images=true '
        'original_send=false completion=false server=false product=false')
    footers = [line for line in text.splitlines() if line.startswith('HC074-HELD-LIVE ')]
    require(footers==([footer] if passed else []), 'Missing/foreign/duplicate/premature success footer')
    phases=[]
    for row in rows:
        projected=copy.deepcopy(row)
        if row['phase']=='outputs' and not row['returned']:projected['phase']='unfinished-outputs'
        fact=prior.phase_facts(projected);fact['phase']=row['phase'];phases.append(fact)
    causal=original['causal']
    if passed:
        require(causal and causal['first_refusal'] is None, 'Expected fault cannot erase a Time refusal')
        origin=causal['first']['issued_ns']
        for position in ('first','last'):
            row=causal[position]
            require(row['diagnostic_reason']=='within-window' and row['issued_boot']==row['now_boot']==metadata['boot']
                    and row['issued_ns']==origin and type(row['age_ns']) is int
                    and row['now_ns']-row['issued_ns']==row['age_ns'] and 0<=row['age_ns']<10_000_000_000,
                    'Original causal window/boot/origin drift')
        for site in original['age']['sites']:
            require(site['first_refusal'] is None,'Live age refusal hidden by expected fault')
            if site['calls']:
                for position in ('first','last'):
                    row=site[position]
                    require(row['reason']=='within-window' and row['issued_ns']==origin
                            and type(row['age_ns']) is int and row['now_ns']-row['issued_ns']==row['age_ns']
                            and 0<=row['age_ns']<10_000_000_000,'Live age window/origin drift')
    return dict(lane=receipt['lane'],case=case,exit=receipt['exit'],harness_passed=passed,
                expected_fixture_assertions_passed=passed,expected_injected_rejection=passed and case!='revalidate',
                original=original,phases=phases,child_wall_seconds=receipt['wall_seconds'])


def analyze(packet,source):
    require=prior.require
    metadata,post,intent,result=(prior.load(packet/n) for n in ('metadata.json','postflight.json','intent.json','result.json'))
    require(metadata==post and all(r['source']==source for r in (metadata,intent,result)), 'Source/context drift')
    require(metadata['uid']==0 and not metadata['fixtures'] and not metadata['tracked_dirty']
            and metadata['qualification_sha256']==runner.QUALIFICATION, 'Scope/qualification/fixture drift')
    require(intent['schedule']==[[l,c] for l in runner.LANES for c in method.CASES]
            and intent['seed']=='0x074c051e' and intent['original_windows_seconds']==[300,60,10]
            and intent['child_timeout_seconds']==1200 and intent['kill_after_seconds']==5
            and intent['child_containment_seconds']==runner.CHILD_SECONDS
            and intent['session_seconds']==runner.SESSION_SECONDS and intent['max_log_bytes']==runner.MAX_LOG_BYTES,
            'Schedule/seed/windows/containment drift')
    for row in (intent,result):prior.flags(row,false=('server','product','original_send','native_gate_open'))
    prior.flags(result,false=('full_matrix',))
    require(result['failure'] is None, 'Incomplete wrapper context; retain original raw evidence')
    require(type(result['schedule_completed']) is bool,'Completion type changed')
    require(metadata['fault_method']=={n:prior.sha(Path(__file__).parent/n) for n in method.METHOD},'Method drift')
    profiles=[];expected=[]
    for lane in runner.LANES:
        for index,case in enumerate(method.CASES):
            prefix=packet/f'{lane}-{case}'
            receipt=prior.load(prefix.with_suffix('.exit.json'))
            require(receipt['lane']==lane and receipt['case']==case,'Case order drift')
            prior.flags(prior.load(prefix.with_suffix('.intent.json')),false=('server','product','original_send','native_gate_open'))
            require(prior.load(packet/f'{lane}-{case}-before.json')==metadata==prior.load(packet/f'{lane}-{case}-after.json'),
                    'Profile context drift')
            expected.append(receipt)
            profiles.append(profile_facts(prefix.with_suffix('.log'),receipt,metadata))
            if receipt['exit']!=0:
                require(not any(list(packet.glob(f'{lane}-{c}*')) for c in method.CASES[index+1:]), 'Continuation after lane refusal')
                break
    require(result['profiles']==expected and result['measured_profile_invocations']==len(expected), 'Recorded scheduling drift')
    complete=len(profiles)==8 and all(r['harness_passed'] for r in profiles)
    require(result['schedule_completed'] is complete,'False matrix completion')
    return dict(schema='original-held-local-fault-review-074-v1',source=source,metadata=metadata,profiles=profiles,
                remaining_fixture_cases_complete=complete,full_sigkill_matrix=False,original_delivery_proven=False,
                all_production_writers_proven=False,overhead_subtracted=False,cpu_speedup_claim=False,
                native_gate_open=False,server=False,product=False)


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source',required=True)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--verify',action='store_true')
    args=parser.parse_args()
    packet=Path(__file__).resolve().parents[2]/'docs/testing/performance/0.74/local-runs'/method.NAME
    prior.publish(args.output,analyze(packet,args.source),args.verify)
    print('Original local fault outcomes reconstructed; no full SIGKILL/server admission')
