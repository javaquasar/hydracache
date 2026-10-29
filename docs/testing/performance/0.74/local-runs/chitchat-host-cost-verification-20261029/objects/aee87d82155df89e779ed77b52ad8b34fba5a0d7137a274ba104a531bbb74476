"""Independent raw receipt accounting for two control contexts, never CPU/RSS."""
import json
import re
import hashlib
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]
R0='chitchat-host-cost-20261029';R1=R0+'-r1'


def need(ok,reason):
    if not ok:raise ValueError(reason)


def local(name):return (ROOT/'target'/name).read_bytes()


def owner(value,code):
    need(value==dict(bound_seconds=1200,capacity_poll_seconds=5,classification='success' if code==0 else 'test-failure',
        cleanup_seconds=5,errors=[],exit=code,observed_child_exit=code,production_writer_authority=False,retired=True),'exact retired original ownership')


def reconcile(get=local):
    read=lambda name:json.loads(get(name))
    first=read(R0+'/collection/result.json');session=read(R0+'/collection/session-result.json')
    need(first['complete'] is False and first['postflight_passed'] and session['complete'] is False
        and session['first_refusal']['reason']=="name 'FIELDS' is not defined",'first whole carrier refusal retained')
    owner(read(R0+'/collection/execution.json'),1);owner(read(R0+'/collection/control-B/execution.json'),0)
    need(read(R0+'/collection/preflight.json')==read(R0+'/collection/postflight.json'),'first unchanged postflight')
    body0=read(R0+'/collection/control-B/raw-body.json')
    expected=read(R0+'/prepared/input/semantics/input/expected-caller.json')
    need(body0['rows']==expected and len(expected)==30,'first complete native semantic body, unpromoted')
    final=read(R1+'/collection/result.json');session=read(R1+'/collection/session-result.json')
    need(final['complete'] and final['postflight_passed'] and session['complete'],'whole r1 control session')
    for result in (first,final,session):
        need(all(result[k] is False for k in ('CPU_qualified','RSS_qualified','cohort_dispatched','exclusive_host')),'no numerical qualification promotion')
    owner(read(R1+'/collection/execution.json'),0)
    before=read(R1+'/collection/preflight.json')
    need(before==read(R1+'/collection/postflight.json')==read(R0+'/collection/postflight.json')==read(R0+'/preflight.stdout.log'),'same original host/boot/supervisor/clock/mount frame')
    old=read(R0+'/prepared/input/inputs.json');new=read(R1+'/patch/input/inputs.json')
    old_raw=get(R0+'/prepared/input/inputs.json');new_raw=get(R1+'/patch/input/inputs.json')
    need(hashlib.sha256(old_raw).hexdigest()==new['parent_manifest_sha256'],'original parent manifest pin')
    files=dict(old['files']);files['remote.py']=hashlib.sha256(get(R1+'/patch/input/remote.py')).hexdigest()
    need(files==new['files'] and old['expected_host']==new['expected_host']==before and old['protocol']==new['protocol'],'one carrier change, no input/protocol/host mutation')
    started=read(R1+'/collection/started.json')
    need(started['manifest_sha256']==hashlib.sha256(new_raw).hexdigest() and started['boot']==before['boot'],'original owner manifest/boot')
    pids=re.findall(r'^OWNER_PID=(\d+)$',get(R1+'/stage.stdout.log').decode(),re.M)
    need(pids==[str(started['owner_pid'])] and int(started['owner_start_ticks'])>0,'same detached owner PID/start ticks')
    summary=[];identities=[];deadlines=[]
    for variant in ('B','P','Q'):
        prefix=R1+'/collection/control-'+variant+'/'
        owner(read(prefix+'execution.json'),0);need(read(prefix+'result.json')['complete'],'complete per-image control')
        call=read(prefix+'intent.json');need(call['retry'] is False and call['bound']==1200 and call['argv'][-1]=='control','fixed unchanged control call')
        need(call['image_sha256']==new['files']['images/'+variant] and call['manifest_sha256']==started['manifest_sha256'],'original exact executable join')
        deadlines.append(call['deadline'])
        rows=[s.removeprefix('HC074_CLEAN_COST:') for s in get(prefix+'stdout.log').decode().splitlines() if s.startswith('HC074_CLEAN_COST:')]
        need(len(rows)==1,'one complete raw control body');body=json.loads(rows[0])
        need(body==read(prefix+'raw-body.json') and body['rows']==expected and body['variant']==variant and body['mode']=='control','raw full semantic equality')
        need(body['seed']==new['protocol']['seed'] and not body['timings'] and body['test_cfg'] is body['allocation_wrapper'] is False,'same seed, non-test/non-wrapper control')
        need(body['cpu_repetitions']==32 and body['memory_epochs']==6 and body['memory_sweeps']==8 and body['warm_sweeps']==4,'unmodified frozen work configuration')
        stamp=body['elapsed'];limit=new['protocol']['monotonic_raw_drift_fraction'];need(limit==.01,'original clock threshold')
        need(len(body['resolutions_ns'])==4 and all(type(v) is int and v>0 for v in body['resolutions_ns']),'qualified resolutions')
        need(stamp['raw']>100*body['resolutions_ns'][0] and stamp['cpu']>100*body['resolutions_ns'][3] and stamp['cpu']<=stamp['raw']*1.1,'resolved one-thread CPU/wall')
        for field in ('mono','boot'):need(abs(stamp[field]-stamp['raw'])<=stamp['raw']*limit,'unchanged clock predicate')
        need(body['boot_id']==before['boot'] and body['pid']>0 and body['start_ticks']>0,'same live process boot')
        identities.append((body['pid'],body['start_ticks']))
        need(len(body['memory'])==7 and all(v['threads']==1 for v in body['memory'][:2]),'single-thread control telemetry')
        summary.append(dict(variant=variant,pid=body['pid'],start_ticks=body['start_ticks'],semantic_rows=30,
            raw_ns=stamp['raw'],mono_ns=stamp['mono'],relative_clock_discrepancy=abs(stamp['raw']-stamp['mono'])/stamp['raw']))
    need(len(set(deadlines))==1 and len(set(identities))==3,'one original session deadline and three separate processes')
    return dict(complete=True,host_control_gate_pass=True,controls=summary,successful_control_semantic_rows=90,
        first_carrier_refusal_retained=True,first_WSL_refusal_unchanged=True,supervisor_unchanged=True,
        exclusive_host=False,cohort_dispatched=False,CPU_qualified=False,RSS_qualified=False,
        continuous_retention_qualified=False,generic_component_gate_pass=False,root_adopted=False,release_admitted=False)


if __name__=='__main__':
    folder=ROOT/'target'/R1/'analysis';folder.mkdir(exist_ok=False)
    result=reconcile();(folder/'result.json').write_text(json.dumps(result,indent=2,sort_keys=True)+'\n',encoding='utf8',newline='\n')
    print(json.dumps(result))
