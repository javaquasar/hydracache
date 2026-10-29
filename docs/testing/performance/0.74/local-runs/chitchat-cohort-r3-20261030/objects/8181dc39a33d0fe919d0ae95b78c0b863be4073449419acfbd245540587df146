"""Finite approved diagnostic maintenance; exact existing fence/retirement/post."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

sys.dont_write_bytecode=True
BASE_SHA='fcb8a53eba5ebc7e1364cbfa0d2bcb9e0ad1098f75b4e53dae56b7503c02ed38'


def need(ok,why):
    if not ok:raise RuntimeError(why)


def load(path,name):
    spec=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m


def parse_cpus(raw):
    result=set()
    for part in raw.strip().split(','):
        values=part.split('-');need(len(values) in (1,2) and all(v.isdigit() for v in values),'kernel CPU-list syntax')
        low=int(values[0]);high=int(values[-1]);need(low<=high,'ordered kernel CPU range')
        result.update(range(low,high+1))
    return result


def cpu_profile():
    online=Path('/sys/devices/system/cpu/online').read_text().strip()
    effective=Path('/sys/fs/cgroup/system.slice/cpuset.cpus.effective').read_text().strip()
    need({0,1,2,3,4,5,6,7}<=parse_cpus(online)&parse_cpus(effective),'online/system.slice CPUs support frozen placement')
    return dict(online=online,system_slice_effective=effective)


def read_plan(root,pin):
    need(root.parent==Path('/tmp') and root.name.startswith('hc074-chitchat-cost-20261029.'),'exact temporary numerical/fixture root')
    s=root.lstat();need(s.st_uid==s.st_gid==0 and s.st_mode&0o7777==0o700 and not root.is_symlink(),'root-only original directory')
    raw=(root/'plan.json').read_bytes();need(hashlib.sha256(raw).hexdigest()==pin,'external immutable plan pin')
    state=(root/'plan.json').lstat();need(state.st_uid==state.st_gid==0 and not state.st_mode&0o222 and state.st_nlink==1,'immutable original plan owner/mode')
    p=json.loads(raw);need(p['unit']==root.name+'.service' and p['schema_version']==1,'original unit/schema')
    if p['synthetic']:need(p['supervisor']==root.name+'-standin.service','owned stand-in only')
    else:need(p['supervisor']=='hydracache-performance-supervisor-074.service' and p['operator_approved'] is True,'exact approved stable supervisor')
    p['_pin']=pin;return p


def backend(root):
    need(hashlib.sha256((root/'maintenance_base.py').read_bytes()).hexdigest()==BASE_SHA,'unchanged reviewed D3 base')
    return load(root/'maintenance_base.py','hc074_exact_maintenance_base')


def frame(p,root,m):
    ns=load(root/'namespace.py','hc074_live_mount_context');ns.verify(p)
    m.static_frame(p)
    need(cpu_profile()==p['cpu_profile'],'original online/effective CPU profile')
    need(shutil.disk_usage(root).free>=4*1024**3,'original four-GiB floor')
    h=load(root/'input/host.py','hc074_numerical_host');state=h.snapshot();original=p['expected_host']
    need({k:v for k,v in state.items() if k!='supervisor'}=={k:v for k,v in original.items() if k!='supervisor'},'original kernel/clock/mount/marker/source host frame')
    if p['synthetic'] or not (root/'inactive.json').exists():need(state['supervisor']==original['supervisor'],'original live supervisor generation')


def timer_install(p,seq,root,deadline,m):
    name=m.timer_name(p,seq)
    m.command(['/usr/bin/systemd-run','--quiet','--no-ask-password','--unit='+name,
        '--on-boot='+f'{deadline//10**9}.{deadline%10**9:09d}s','--timer-property=AccuracySec=1us',
        '--timer-property=RandomizedDelaySec=0','--property=Type=exec','--property=RuntimeMaxSec=10',
        '/usr/bin/python3','-B','-I',str(root/'coordinator.py'),'expire',str(root),str(seq),p['_pin']])


def validate_receipt(root,seq,attempt,p):
    path=root/(str(seq)+'.stdout.log');rows=[v.removeprefix('HC074_CLEAN_COST:') for v in path.read_text().splitlines() if v.startswith('HC074_CLEAN_COST:')]
    need(len(rows)==1,'one actual complete body');body=json.loads(rows[0])
    adapter=load(root/'input/remote.py','hc074_pinned_control_adapter')
    adapter.validator(root)(body,attempt['variant'],attempt['mode'])
    need(body['boot_id']==p['boot'],'same original process boot')
    need(body['pid']==json.loads((root/(str(seq)+'.worker-child.json')).read_bytes())['pid'],'actual measured child identity')
    return body


def main(p,root,m):
    deadline=time.monotonic_ns()+1100*10**9
    unit_group=m.own_group(p['unit'],'main');frame(p,root,m)
    before=m.service_state(p['supervisor'])
    need(before==p['supervisor_before'] and before['ActiveState']=='active','original supervisor generation')
    m.publish(root,'session-deadline.json',dict(deadline_ns=deadline,session_bound_seconds=1100,outer_bound_seconds=1200,rebased=False))
    m.publish(root,'before-stop.json',before);m.command(['/usr/bin/systemctl','stop',p['supervisor']],30)
    inactive=m.service_state(p['supervisor']);need(inactive['ActiveState']=='inactive' and inactive['MainPID']=='0','inactive before common fence')
    m.publish(root,'inactive.json',inactive);fence=m.open_fence(p)
    try:
        frame(p,root,m);os.sched_setaffinity(0,{0,5,6,7});need(set(os.sched_getaffinity(0))=={0,5,6,7},'exact controller placement')
        for seq,attempt in enumerate(p['attempts']):
            frame(p,root,m);need(m.service_state(p['supervisor'])['ActiveState']=='inactive','supervisor remains inactive')
            quiet=None if p['synthetic'] else m.quiet_guard()
            need(time.monotonic_ns()<deadline,'original complete session deadline before effect')
            group=unit_group/('writers-'+str(seq));group.mkdir();identity=m.group_identity(group)
            at=min(deadline,time.monotonic_ns()+p['synthetic_deadline_seconds']*10**9) if p['synthetic'] else deadline
            argv=[str(fence) if arg=='@FENCE_FD@' else p['_pin'] if arg=='@PLAN_SHA@' else arg for arg in attempt['argv']]
            if not p['synthetic']:need(attempt['argv'].count('@FENCE_FD@')==attempt['argv'].count('@PLAN_SHA@')==1,'fixed original fence/plan bindings')
            record=dict(attempt=attempt,actual_argv=argv,group=str(group),group_identity=identity,deadline_ns=at,quiet=quiet,boot=p['boot'],fence_fd=fence)
            m.publish(root,str(seq)+'.intent.json',record);timer_install(p,seq,root,at,m)
            def enter():
                need(time.monotonic_ns()<at,'deadline before entering original writer group')
                (group/'cgroup.procs').write_text(str(os.getpid()));os.sched_setaffinity(0,{1,2,3,4})
                os.setgroups([]);os.setgid(0);os.setuid(0)
                need(time.monotonic_ns()<at,'deadline before writer exec')
            child=subprocess.Popen(argv,stdin=subprocess.DEVNULL,stdout=subprocess.PIPE,stderr=subprocess.PIPE,
                pass_fds=(fence,),preexec_fn=enter,env=p['child_env'])
            m.publish(root,str(seq)+'.child.json',dict(pid=child.pid,fence_fd=fence,
                start_ticks=Path(f'/proc/{child.pid}/stat').read_text().rsplit(')',1)[1].split()[19]))
            code=m.capture(root,str(seq),child,16*1024**2)
            states=m.empty_group(group,identity);m.timer_stop(p,seq)
            m.publish(root,str(seq)+'.retired.json',dict(exit=code,events=states,all_original_writers_stopped=True))
            need(code==0,'first child refusal '+str(seq)+': '+str(code)+'; no retry')
            frame(p,root,m)
            if not p['synthetic']:
                body=validate_receipt(root,seq,attempt,p);m.publish(root,str(seq)+'.body.json',body)
            need(time.monotonic_ns()<deadline,'original complete session deadline after effect')
        m.publish(root,'cohort-complete.json',dict(attempts=len(p['attempts']),complete=True,admission=False))
    finally:os.close(fence)


def post(p,root,m,external=False):
    ns=load(root/'namespace.py','hc074_post_mount_context')
    recovery=load(root/'recovery.py','hc074_terminal_recovery')
    recovery.post(p,root,m,ns,frame,types_base,external)
    if not external:ns.stop(root,p,m)
    else:
        state=m.service_state(p['namespace_unit'])
        need(state['MainPID']=='0' and state['ActiveState'] in ('inactive','failed'),'departed original namespace owner unit')
        m.publish(root,'namespace-retired.json',dict(unit=p['namespace_unit'],original_identity=p['namespace']['identity'],state=state,owner_departed=True))
    m.publish(root,'terminal-complete.json',dict(restoration_verified=True,namespace_retired=True,admission=False))


def types_base(static):
    import types
    return types.SimpleNamespace(static_frame=static)


def entry():
    need(len(sys.argv) in (4,5),'exact phase/root/pin args')
    mode=sys.argv[1];root=Path(sys.argv[2]).resolve(strict=True);p=read_plan(root,sys.argv[-1]);m=backend(root)
    if mode in ('main','post'):
        ns=load(root/'namespace.py','hc074_namespace_entry');external=False
        try:ns.join(root,p)
        except (FileNotFoundError,ProcessLookupError):
            if mode!='post':raise
            external=True
    if mode=='main' and len(sys.argv)==4:main(p,root,m)
    elif mode=='post' and len(sys.argv)==4:post(p,root,m,external)
    elif mode=='expire' and len(sys.argv)==5 and sys.argv[3].isdigit():m.expire(p,root,int(sys.argv[3]))
    else:raise RuntimeError('fixed main/post/expire only')


if __name__=='__main__':entry()
