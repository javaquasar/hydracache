"""New bounded private context for the explicitly approved replacement matrix."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import time

sys.dont_write_bytecode=True
PARENT=Path('/tmp/hc074-chitchat-control-root-20261029.1BBG91')
PARENT_PIN='90057ac9679be66d141a0f3b573fa6d911d8287365051fc10e379479ffb26c20'
NAMES=('coordinator.py','worker.py','maintenance_base.py','plan.py','stage.py','namespace.py','recovery.py')
CASES=('healthy','deadline','owner-kill','frame-denial','frame-denial-owner-kill','namespace-owner-kill','user-leaf-removal')

def need(ok,why):
    if not ok:raise RuntimeError(why)

def load(path,name):
    s=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(s);s.loader.exec_module(m);return m

def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def put(path,value):
    with path.open('x') as f:json.dump(value,f,indent=2,sort_keys=True);f.flush();os.fsync(f.fileno())
def command(argv):return subprocess.check_output(argv,timeout=45,text=True).strip()

def original(package):
    need(os.geteuid()==os.getegid()==0 and not PARENT.is_symlink(),'original root-only source custody')
    need(sha(PARENT/'input/inputs.json')==PARENT_PIN and json.loads((PARENT/'result.json').read_bytes())['complete'],'qualified original input manifest')
    adapter=load(PARENT/'input/remote.py','hc074_source_control');adapter.verify(PARENT,json.loads((PARENT/'input/inputs.json').read_bytes()))
    h=load(PARENT/'input/host.py','hc074_public_host');actual=h.snapshot();h.admit(actual)
    expected=json.loads((package/'anchor.json').read_bytes())['host']
    # Public ephemeral mounts are captured anew BEFORE the new namespace/plan.
    # Every mount record of that new private context is then frozen and checked.
    need({k:v for k,v in actual.items() if k!='mounts'}=={k:v for k,v in expected.items() if k!='mounts'},'pinned current public boot/service/protected/clock/namespace context before preparation')
    load(package/'coordinator.py','hc074_cpu_gate').cpu_profile()
    need(shutil.disk_usage('/tmp').free>=4*1024**3,'original capacity floor')
    return actual

def build(package,synthetic=False,case=None,controls=False):
    external=original(package);root=Path(tempfile.mkdtemp(prefix='hc074-chitchat-cost-20261029.',dir='/tmp'));root.chmod(0o700)
    shutil.copytree(PARENT/'input',root/'input',copy_function=shutil.copy2)
    for name in NAMES:shutil.copyfile(package/name,root/name);(root/name).chmod(0o400)
    m=load(root/'maintenance_base.py','hc074_original_base');pmod=load(root/'plan.py','hc074_new_plan')
    if synthetic:
        standin=root.name+'-standin.service';command(['/usr/bin/systemd-run','--quiet','--no-ask-password','--unit='+standin,'--service-type=exec','--property=RuntimeMaxSec=120','/usr/bin/sleep','100'])
        fence=root/'private.lock';fence.touch(mode=0o400)
    else:fence=Path('/var/lib/hydracache-performance/campaigns/.host-execution.lock')
    st=fence.lstat();need(fence.is_file() and not fence.is_symlink() and st.st_uid==0 and st.st_nlink==1,'exact original fence')
    namespace_unit=root.name+'-namespace.service'
    argv=['/usr/bin/systemd-run','--quiet','--no-ask-password','--unit='+namespace_unit,'--service-type=exec',
        '--property=RuntimeMaxSec=1200','--property=KillMode=control-group','--property=Restart=no',
        '/usr/bin/unshare','--mount','--propagation','private','/usr/bin/python3','-B','-I',str(root/'namespace.py'),'keeper',str(root)]
    put(root/'namespace-intent.json',dict(argv=argv,external_host=external,retry=False));command(argv)
    at=time.monotonic()
    while not (root/'namespace-ready.json').exists() and time.monotonic()-at<10:time.sleep(.02)
    meta=json.loads((root/'namespace-ready.json').read_bytes())
    need(m.service_state(namespace_unit)['MainPID']==str(meta['pid']),'original PID1 namespace owner')
    need({k:v for k,v in meta['host'].items() if k!='mounts'}=={k:v for k,v in external.items() if k!='mounts'},'private context changes only own mount graph')
    state=dict(meta['host']);fake=dict(state)
    if synthetic:fake['supervisor']='\n'.join(k+'='+v for k,v in m.service_state(standin).items())
    code='''import json,os,pathlib,subprocess,sys,time
root=pathlib.Path(sys.argv[1]);case=sys.argv[2];fd=int(sys.argv[3])
assert os.getuid()==os.getgid()==0;copy=os.dup(fd)
if case.startswith('frame-denial'):
 subprocess.run(['/usr/bin/mount','-o','remount,bind,ro',str(root/'fixture-mount')],check=True,timeout=5)
 with (root/'fault-mounted.json').open('x') as f:json.dump(dict(private_only=True),f);f.flush();os.fsync(f.fileno())
if case=='user-leaf-removal':subprocess.run(['/usr/bin/umount','--lazy','--','/run/user/1000'],check=True,timeout=5)
os.fork();end=time.monotonic()+(20 if case=='ssh-disconnect' else .2 if case in ('healthy','frame-denial','user-leaf-removal') else 90)
while time.monotonic()<end:os.write(1,b'owned-writer\\n');time.sleep(.01)
os._exit(0)
'''
    argv=['/usr/bin/python3','-B','-I','-c',code,str(root),case or '', '@FENCE_FD@'] if synthetic else None
    files={str(p):sha(p) for p in sorted((root/'input').rglob('*')) if p.is_file()};files.update({str(root/n):sha(root/n) for n in NAMES})
    p=pmod.make(root,fake,files,fence,[st.st_dev,st.st_ino],synthetic,case,argv)
    p.update(namespace=meta,namespace_unit=namespace_unit,external_host=external,cpu_profile=load(root/'coordinator.py','hc074_cpu_profile').cpu_profile(),synthetic_case=case)
    if synthetic:p['expected_host']=state
    if controls:p['attempts']=[dict(mode='control',variant=v,argv=['/usr/bin/python3','-B','-I',str(root/'worker.py'),str(root),str(i),'@PLAN_SHA@','@FENCE_FD@']) for i,v in enumerate(('B','P','Q'))]
    put(root/'plan.json',p);(root/'plan.json').chmod(0o400);pin=sha(root/'plan.json')
    put(root/'prepared.json',dict(root=str(root),plan_sha256=pin,synthetic=synthetic,controls=controls,case=case))
    return root,pin,external

def kill_original(root,m):
    pid=int(m.service_state(root.name+'.service')['MainPID']);fd=os.pidfd_open(pid)
    try:
        need(Path('/proc/'+str(pid)+'/cgroup').read_text().strip()=='0::/system.slice/'+root.name+'.service/control','exact coordinator cgroup before external SIGKILL')
        signal.pidfd_send_signal(fd,signal.SIGKILL)
    finally:os.close(fd)

def wait_result(root,seconds=25):
    at=time.monotonic()
    while not (root/'terminal-complete.json').exists() and time.monotonic()-at<seconds:time.sleep(.05)
    need((root/'terminal-complete.json').exists(),'autonomous postflight and namespace retirement must complete')
    return json.loads((root/'postflight-result.json').read_bytes())

def faults(package):
    record=package/'faults-once';record.mkdir();before=original(package);cases=[]
    for case in CASES:
        root,pin,_=build(package,True,case);m=load(root/'maintenance_base.py','hc074_fault_base');pmod=load(root/'plan.py','hc074_fault_plan')
        argv=pmod.launch(root,pin,True);put(root/'launch-intent.json',dict(argv=argv,retry=False));command(argv)
        if case in ('owner-kill','frame-denial-owner-kill','namespace-owner-kill'):
            required='fault-mounted.json' if case=='frame-denial-owner-kill' else '0.child.json';at=time.monotonic()
            while not (root/required).exists() and time.monotonic()-at<10:time.sleep(.02)
            need((root/required).exists(),'fault boundary reached')
            if case=='namespace-owner-kill':
                meta=json.loads((root/'namespace-ready.json').read_bytes());fd=os.pidfd_open(meta['pid'])
                try:need(load(root/'namespace.py','hc074_owner_identity').identity(meta['pid'])==meta['start_ticks'],'original namespace owner');signal.pidfd_send_signal(fd,signal.SIGKILL)
                finally:os.close(fd)
            kill_original(root,m)
        result=wait_result(root);stopped=json.loads((root/'all-writers-stopped.json').read_bytes());restored=json.loads((root/'restored.json').read_bytes())
        need(result['restoration_verified'] and restored['verified'] and stopped['original_groups_empty'] and stopped['original_fence_acquired'],'autonomous strict original retirement/fence/restoration')
        need((root/'cohort-complete.json').exists()==(case=='healthy'),'fault cannot promote cohort completion')
        if case.startswith('frame-denial') or case in ('namespace-owner-kill','user-leaf-removal'):need(not result['full_measurement_frames_pass'],'denied frame remains denied after restoration')
        if case=='deadline':need((root/'0.deadline.json').exists(),'real absolute watchdog fired')
        (root/'journal.log').write_text(command(['/usr/bin/journalctl','--no-pager','-u',root.name+'.service','-n','100','-o','short-monotonic']),encoding='utf8')
        for unit in (root.name+'.service',root.name+'-standin.service'):
            x=subprocess.run(['/usr/bin/systemctl','stop',unit],capture_output=True,timeout=15)
            state=m.service_state(unit);need(state['MainPID']=='0' and state['ActiveState'] in ('inactive','failed'),'only own completed fixture units retired')
            need(x.returncode in (0,5),'owned fixture stop status')
        cases.append(dict(case=case,root=str(root),passed=True,postflight=result))
    after=original(package);need({k:v for k,v in before.items() if k!='mounts'}=={k:v for k,v in after.items() if k!='mounts'},'real stable service unchanged by faults')
    value=dict(complete=True,cases=cases,stable_supervisor_unchanged=True,external_before=before,external_after=after)
    put(record/'result.json',value);print(json.dumps(value))

def prepare_numeric(package):
    need(json.loads((package/'faults-once/result.json').read_bytes())['complete'],'seven actual fault controls before maintenance')
    need(json.loads((package/'worker-controls-once/result.json').read_bytes())['complete'],'whole worker controls before maintenance')
    need(json.loads((package/'context-once/result.json').read_bytes())['complete'],'actual disconnected context control before maintenance')
    root,pin,_=build(package);put(package/'numerical.json',dict(root=str(root),plan_sha256=pin,prepared=True,launched=False))
    print(json.dumps(dict(root=str(root),plan_sha256=pin,prepared=True,attempts=60)))

def context_start(package):
    record=package/'context-once';record.mkdir();root,pin,external=build(package,True,'ssh-disconnect')
    argv=load(root/'plan.py','hc074_disconnected_context').launch(root,pin,True)
    put(root/'launch-intent.json',dict(argv=argv,retry=False));command(argv)
    value=dict(root=str(root),plan_sha256=pin,external_before=external,retry=False)
    put(record/'started.json',value);print(json.dumps(value))

def context_check(package):
    record=package/'context-once';v=json.loads((record/'started.json').read_bytes());root=Path(v['root'])
    result=wait_result(root);need(result['full_measurement_frames_pass'] and (root/'cohort-complete.json').exists(),'complete full context across actual SSH disconnect')
    p=json.loads((root/'plan.json').read_bytes());m=load(root/'maintenance_base.py','hc074_disconnect_receipt')
    need(p['namespace']['user_roots'] and json.loads((root/'0.retired.json').read_bytes())['events']['populated']=='0','held user roots and original writer retirement')
    after=original(package);need(after['supervisor']==v['external_before']['supervisor'],'real supervisor unchanged across disconnect')
    for unit in (root.name+'.service',root.name+'-standin.service'):
        x=subprocess.run(['/usr/bin/systemctl','stop',unit],capture_output=True,timeout=15);need(x.returncode in (0,5),'only own completed context units stop')
    value=dict(complete=True,root=str(root),private_full_context_unchanged=True,synthetic_writer_seconds=20,external_before=v['external_before'],external_after=after)
    put(record/'result.json',value);print(json.dumps(value))

def worker_controls(package):
    record=package/'worker-controls-once';record.mkdir();before=original(package)
    root,pin,_=build(package,True,'healthy',True);pmod=load(root/'plan.py','hc074_worker_smoke');argv=pmod.launch(root,pin,True)
    put(root/'launch-intent.json',dict(argv=argv,retry=False,controls_only=True));command(argv);result=wait_result(root)
    need(result['full_measurement_frames_pass'] and (root/'cohort-complete.json').exists(),'complete healthy full-worker controls')
    c=load(root/'coordinator.py','hc074_worker_validator');p=json.loads((root/'plan.json').read_bytes());rows=[]
    for i,a in enumerate(p['attempts']):
        body=c.validate_receipt(root,i,a,p);owner=json.loads((root/(str(i)+'.execution.json')).read_bytes())
        need(owner['exit']==0 and owner['retired'] and not owner['errors'],'original worker ownership')
        rows.append(dict(variant=body['variant'],pid=body['pid'],start_ticks=body['start_ticks'],semantic_rows=len(body['rows'])))
    m=load(root/'maintenance_base.py','hc074_smoke_cleanup')
    for unit in (root.name+'.service',root.name+'-standin.service'):
        x=subprocess.run(['/usr/bin/systemctl','stop',unit],capture_output=True,timeout=15);need(x.returncode in (0,5),'only owned completed control units stop')
    after=original(package);need(before['supervisor']==after['supervisor'],'stable unchanged by worker smoke')
    value=dict(complete=True,root=str(root),bodies=rows,stable_supervisor_unchanged=True,CPU_qualified=False,RSS_qualified=False)
    put(record/'result.json',value);print(json.dumps(value))

def launch_numeric(package):
    original(package);v=json.loads((package/'numerical.json').read_bytes());root=Path(v['root']);need(sha(root/'plan.json')==v['plan_sha256'],'original external plan pin')
    p=json.loads((root/'plan.json').read_bytes());need(time.monotonic_ns()+1140*10**9<p['namespace']['deadline_ns'],'full original session plus restoration fits original namespace bound')
    argv=load(root/'plan.py','hc074_numerical_launch').launch(root,v['plan_sha256']);put(root/'launch-intent.json',dict(argv=argv,retry=False));command(argv)
    print(json.dumps(dict(launched_once=True,root=str(root),plan_sha256=v['plan_sha256'])))

if __name__=='__main__':
    package=Path(__file__).resolve().parent;need(len(sys.argv)==2 and sys.argv[1] in ('faults','worker-controls','context-start','context-check','prepare','launch'),'fixed modes')
    {'faults':faults,'worker-controls':worker_controls,'context-start':context_start,'context-check':context_check,'prepare':prepare_numeric,'launch':launch_numeric}[sys.argv[1]](package)
