"""Seal the separate GetUnit attempt and attribute its saved clock refusal offline."""
import argparse
import json
from pathlib import Path
import subprocess
import getunit_once as run
import seal as old

ROOT=run.ROOT
INPUT=run.OUTPUT
OUTPUT=ROOT/'docs/testing/performance/0.74/local-runs/bootstrap-original-getunit-20261023'

def clock_reason(event):
    if event['site']!='clock-order' or len(event['clock'])!=6:
        raise ValueError('Not the recorded ordering refusal')
    before_boot,after_boot,before_real,after_real,same,hold=event['clock']
    if same not in (0,1) or hold!=60_000_000_000:
        raise ValueError('Unexpected original clock contract')
    boot_delta=after_boot-before_boot
    real_delta=after_real-before_real
    if not same: reason='boot_changed'
    elif boot_delta<0: reason='boottime_backwards'
    elif real_delta<0: reason='realtime_backwards'
    else: raise ValueError('Clock-order trace does not violate original predicate')
    return {'reason':reason,'boottime_delta_ns':boot_delta,'realtime_delta_ns':real_delta,
        'same_boot':bool(same),'original_hold_ns':hold,'trace':event,
        'caused_by_specific_service':None,'clocks_changed_by_operator':False}

def native(folder):
    result=old.read(folder/'result.json')
    if result['returncode']!=101 or result['retry'] or result['original_send_observed'] or result['readiness_observed']:
        raise ValueError('Do not relabel the first negative attempt')
    stdout=(folder/'live.stdout.log').read_text()
    stderr=(folder/'live.stderr.log').read_text()
    if stdout.count('running 1 test')!=1 or stdout.count('HC074-ORIGINAL-SERVICE-FIRST ')!=1:
        raise ValueError('Missing or duplicate original test')
    if stdout.count('HC074-ORIGINAL-SCOPE-INTENT ')!=1 or 'HC074-ORIGINAL-SCOPE slice=' not in stdout:
        raise ValueError('No exact fresh scope creation')
    for required in ('HC074-ORIGINAL-GROUP cleanup_ok=true','HC074-ORIGINAL-SCOPE cleanup_ok=true'):
        if stdout.count(required)!=1: raise ValueError('Owned cleanup not confirmed')
    if 'original_send=true' in stdout or 'readiness=true' in stdout or 'Reservation(Input(Kernel(Invalid)))' not in stderr:
        raise ValueError('Unexpected delivery or refusal')
    if (folder/'postflight-result.txt').read_text().strip()!='test_rc=101 cleanup_rc=0':
        raise ValueError('Cleanup failed')
    if (folder/'preflight.txt').read_bytes()!=(folder/'postflight.txt').read_bytes():
        raise ValueError('Context changed')
    facts=json.loads(stdout.split('HC074-ORIGINAL-SERVICE-FIRST ',1)[1].splitlines()[0])
    if facts['returned'] or facts['deadline_changed'] or facts['guard_clock_added'] or facts['causal']['observations'] or any(s['calls'] for s in facts['age']['sites']):
        raise ValueError('Original observations or origins differ')
    analysis=clock_reason(facts['kernel'])
    intent=old.read(folder/'intent.json')
    if intent['original_windows']!=[300,60,10] or intent['qualification']!=run.old.QUAL:
        raise ValueError('Original windows or qualification differ')
    for name in ('bootstrap','libtest'):
        if run.old.sha(folder/name)!=intent[name]: raise ValueError('Executed artifact differs')
    for stream in ('stdout','stderr'):
        if run.old.sha(folder/('session.'+stream+'.log'))!=result[stream+'_sha256']:
            raise ValueError('Session log differs')
    analysis.update(causal_observations=0,age_site_calls=[s['calls'] for s in facts['age']['sites']],
        original_send=False,readiness=False,test_rc=101,cleanup_rc=0,
        kernel_phase=facts['kernel']['phase'],original_windows=[300,60,10])
    return analysis

def retain():
    old.validate_lane(INPUT/'checks',False)
    proof=old.read(INPUT/'build-proof.json')
    if proof['checked_sources']!=old.read(INPUT/'checks/source-after.json'):
        raise ValueError('Build/source proof mismatch')
    analysis=native(INPUT/'native-once')
    if run.old.sha(run.ARCHIVE)!=run.ARCHIVE_SHA: raise ValueError('Previous archive changed')
    OUTPUT.mkdir(parents=True,exist_ok=False)
    (OUTPUT/'objects').mkdir()
    with (OUTPUT/'.gitattributes').open('xb') as stream:
        stream.write(b'objects/* -text -diff -whitespace\n*.json text eol=lf\n')
    files={}
    def add(name,source):
        old.relative(name)
        if source.is_symlink(): raise ValueError('Evidence symlink')
        raw=source.read_bytes();sha=old.digest(raw);target=OUTPUT/'objects'/sha
        if not target.exists():
            with target.open('xb') as stream: stream.write(raw)
        files[name]={'sha256':sha,'bytes':len(raw)}
    for lane in ('checks','native-once'):
        for source in sorted((INPUT/lane).rglob('*')):
            if not source.is_file() or (lane=='native-once' and source.name in ('bootstrap','libtest')): continue
            add(source.relative_to(INPUT).as_posix(),source)
    add('build-proof.json',INPUT/'build-proof.json')
    for name in ('getunit_seal.py','test_getunit_seal.py'):
        add('method/'+name,ROOT/'tools/bootstrap-original-service-074'/name)
    index={'schema':'hc074-original-getunit-evidence-v1','files':files,'analysis':analysis,
        'admission':False,'server':False,'product':False,'release_install':False,
        'original_source':old.read(INPUT/'native-once/intent.json')['git'],
        'seal_source':run.git('rev-parse','HEAD'),'previous_index_sha256':run.ARCHIVE_SHA,
        'qualification_sha256':run.old.QUAL}
    old.publish(OUTPUT/'index.json',index)
    old.verify(OUTPUT)
    print(json.dumps({'files':len(files),'objects':len(list((OUTPUT/'objects').iterdir())),
        'index_sha256':run.old.sha(OUTPUT/'index.json'),'analysis':analysis}))

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode',choices=['retain','verify'])
    if parser.parse_args().mode=='retain': retain()
    else: print(json.dumps({'verified':len(old.verify(OUTPUT)['files'])}))
