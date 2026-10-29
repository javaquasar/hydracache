"""One terminal recovery of the refused cohort; never launch or admit a sample.

The numerical frame remains failed. This recovery accepts only the observed
replacement of the unrelated /run/user/1000 tmpfs and preserves its full diff.
Durable original-inode empty proofs, inactive PID1 unit, absent original process
identities, retired timers and exclusive original-fence custody are all required.
An absent cgroup is deliberately never used as an empty-group proof.
"""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT=Path('/tmp/hc074-chitchat-cost-20261029.er47i8gp')
PIN='c38d5aa3712ad53b8f42aa706d8b4086258eb0b085d5cc46d7473491218e47f0'
BASE_SHA='fcb8a53eba5ebc7e1364cbfa0d2bcb9e0ad1098f75b4e53dae56b7503c02ed38'
STABLE='hydracache-performance-supervisor-074.service'


def need(ok,why):
    if not ok:raise RuntimeError(why)


def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()


def load(path,name):
    spec=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m


def mount_replacement(before,after):
    target='/run/user/1000'
    def split(raw):
        other=[];leaf=[]
        for line in raw.splitlines():
            fields=line.split();need(len(fields)>=10,'complete mount record')
            need(not fields[4].startswith(target+'/'),'no nested user mount')
            (leaf if fields[4]==target else other).append(line)
        need(len(leaf)==1,'one original/current user tmpfs')
        f=leaf[0].split();separator=f.index('-')
        need(separator==7 and f[3]=='/' and f[4]==target and f[6].startswith('shared:') and f[8:10]==['tmpfs','tmpfs'],'exact observed tmpfs leaf shape')
        # Mount ID, filesystem device ID and shared peer group changed. Every
        # parent/path/option/fstype/source field and every other mount is exact.
        signature=[v for i,v in enumerate(f) if i not in (0,2,6)]
        return other,signature,leaf[0]
    a,b,old=split(before);x,y,new=split(after)
    need(a==x and b==y,'only exact unrelated tmpfs identity may differ in recovery')
    need(old!=new,'retain the real mount refusal, never erase or rebase it')
    return dict(before=old,after=new,measurement_frame_pass=False)


def host_review(expected,actual):
    need({k:v for k,v in expected.items() if k not in ('mounts','supervisor')}==
         {k:v for k,v in actual.items() if k not in ('mounts','supervisor')},'unchanged protected host except approved service and refused leaf mount')
    return mount_replacement(expected['mounts'],actual['mounts'])


def original_gone(record):
    path=Path('/proc')/str(record['pid'])/'stat'
    try:now=path.read_text().rsplit(')',1)[1].split()[19]
    except FileNotFoundError:return True
    return now!=str(record['start_ticks'])


def stopped_unit(state):
    need(all(state.get(k)==v for k,v in dict(MainPID='0',ActiveState='failed',SubState='failed',
        ControlGroup='',Restart='no',KillMode='control-group',ExecMainStatus='1').items()),'exact failed original unit, no restart or live cgroup')


def unit_state(m,p):
    fields=('MainPID','ActiveState','SubState','ControlGroup','Restart','KillMode','ExecMainStatus')
    return dict(line.split('=',1) for line in m.command(['/usr/bin/systemctl','show',p['unit'],*sum((['-p',k] for k in fields),[])]).splitlines())


def run():
    need(os.geteuid()==os.getegid()==0,'root for own recovery only')
    need(sha(ROOT/'plan.json')==PIN,'external original plan pin');p=json.loads((ROOT/'plan.json').read_bytes())
    need(p['supervisor']==STABLE and p['operator_approved'] and not p['synthetic'],'exact approved stable supervisor')
    need(sha(ROOT/'maintenance_base.py')==BASE_SHA,'original maintenance predicates')
    m=load(ROOT/'maintenance_base.py','hc074_original_recovery_base');m.static_frame(p)
    need(sha(ROOT/'input/host.py')==p['inputs'][str(ROOT/'input/host.py')],'reviewed host reader')
    h=load(ROOT/'input/host.py','hc074_recovery_host');before=h.snapshot();mount=host_review(p['expected_host'],before)
    service=m.service_state(STABLE);need(service['ActiveState']=='inactive' and service['MainPID']=='0','inactive before external fence')
    unit=unit_state(m,p);stopped_unit(unit)
    need(not (ROOT/'cohort-complete.json').exists() and not (ROOT/'restored.json').exists(),'preserve incomplete cohort and failed automatic postflight')
    intents=sorted(ROOT.glob('[0-9]*.intent.json'));need([x.name for x in intents]==[str(i)+'.intent.json' for i in range(5)],'exact five first worker intents, no later effect')
    receipts=[];processes=[]
    for i,path in enumerate(intents):
        record=json.loads(path.read_bytes());retired=json.loads((ROOT/(str(i)+'.retired.json')).read_bytes())
        need(record['boot']==p['boot'] and record['attempt']==p['attempts'][i] and record['group'].endswith('/'+p['unit']+'/writers-'+str(i)),'original intent identity')
        need(retired['all_original_writers_stopped'] and retired['events']['populated']=='0' and retired['exit']==(0 if i<4 else 1),'durable original-inode empty proof before PID1 removed group')
        need(type(record['group_identity']) is list and len(record['group_identity'])==2,'original held group identity')
        for name in (str(i)+'.child.json',str(i)+'.worker-child.json'):
            if name.endswith('.worker-child.json') and i==4:need(not (ROOT/name).exists(),'refused worker never spawned fifth image');continue
            child=json.loads((ROOT/name).read_bytes());need(original_gone(child),'original worker/image process still alive');processes.append(dict(name=name,**child))
        for suffix in ('.timer','.service'):
            name=m.timer_name(p,i)+suffix
            state=dict(line.split('=',1) for line in m.command(['/usr/bin/systemctl','show',name,'-p','LoadState','-p','ActiveState','-p','MainPID']).splitlines())
            need(state.get('LoadState') in ('loaded','not-found') and state.get('ActiveState')=='inactive' and state.get('MainPID','0')=='0','original deadline retired')
        receipts.append(dict(intent=record,retired=retired))
    m.publish(ROOT,'manual-recovery-intent.json',dict(retry=False,installation=False,numerical_replay=False,original_plan_sha256=PIN,
        prior_retirement_receipts=receipts,original_processes_gone=processes,unit=unit,host=before,mount_refusal=mount,
        measurement_frame_pass=False,missing_cgroup_used_as_empty_proof=False))
    fence=m.open_fence(p)
    try:
        m.static_frame(p);host_review(p['expected_host'],h.snapshot());stopped_unit(unit_state(m,p))
        need(m.service_state(STABLE)==service,'same inactive service before restoration')
        m.publish(ROOT,'manual-recovery-fence.json',dict(original_fence_acquired=True,fence_identity=p['fence_identity'],
            durable_original_group_empty_proofs=5,original_processes_gone=9,deadline_units_retired=10,
            measurement_frame_pass=False,cohort_complete=False,missing_cgroup_used_as_empty_proof=False))
    finally:os.close(fence)
    # The stable supervisor must acquire its own fence after our descriptor closes.
    m.command(['/usr/bin/systemctl','start',STABLE],30)
    restored=m.service_state(STABLE);need(restored['ActiveState']=='active' and restored['SubState']=='running' and restored['MainPID']!='0','approved supervisor restored')
    exe='/opt/hydracache-perf/bin/hydracache-long-run-supervisor-074';actual=Path('/proc')/restored['MainPID']/'exe'
    need(os.readlink(actual)==exe and sha(actual)==p['protected_files'][exe],'original installed executable')
    m.static_frame(p);after=h.snapshot();host_review(p['expected_host'],after)
    m.publish(ROOT,'manual-restored.json',dict(supervisor=restored,host=after,verified=True,installation=False,
        automatic_postflight_pass=False,measurement_frame_pass=False,CPU_qualified=False,RSS_qualified=False,numerical_replay=False))
    print(json.dumps(dict(manual_restored=True,supervisor=restored,first_refusal_retained=True,cohort_complete=False,numerical_replay=False)))


if __name__=='__main__':run()
