"""Temporary native staging and synthetic gates; numerical launch is separate."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tarfile
import tempfile
import time

sys.dont_write_bytecode=True
PARENT=Path('/tmp/hc074-chitchat-control-root-20261029.1BBG91')
PARENT_PIN='90057ac9679be66d141a0f3b573fa6d911d8287365051fc10e379479ffb26c20'
NAMES=('coordinator.py','worker.py','maintenance_base.py','plan.py','stage.py')


def load(path,name):
    s=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(s);s.loader.exec_module(m);return m


def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()


def put(path,value):
    with path.open('x') as out:json.dump(value,out,indent=2,sort_keys=True);out.flush();os.fsync(out.fileno())


def command(argv):return subprocess.check_output(argv,timeout=45,text=True).strip()


def original():
    assert os.geteuid()==os.getegid()==0 and not PARENT.is_symlink()
    assert sha(PARENT/'input/inputs.json')==PARENT_PIN
    assert json.loads((PARENT/'result.json').read_bytes())['complete']
    adapter=load(PARENT/'input/remote.py','hc074_original_control');manifest=json.loads((PARENT/'input/inputs.json').read_bytes())
    adapter.verify(PARENT,manifest);host=load(PARENT/'input/host.py','hc074_original_host');state=host.snapshot();host.admit(state)
    assert state==manifest['expected_host']
    load(Path(__file__).parent/'coordinator.py','hc074_kernel_cpu_admission').cpu_profile()
    assert os.statvfs('/tmp').f_bavail*os.statvfs('/tmp').f_frsize>=4*1024**3
    return state


def build(package,synthetic=False,case=None):
    state=original();root=Path(tempfile.mkdtemp(prefix='hc074-chitchat-cost-20261029.',dir='/tmp'));root.chmod(0o700)
    shutil.copytree(PARENT/'input',root/'input',copy_function=shutil.copy2)
    for name in NAMES:
        shutil.copyfile(package/name,root/name);(root/name).chmod(0o400)
    pmod=load(root/'plan.py','hc074_fixed_plan');base=load(root/'maintenance_base.py','hc074_exact_base')
    if synthetic:
        standin=root.name+'-standin.service'
        command(['/usr/bin/systemd-run','--quiet','--no-ask-password','--unit='+standin,'--service-type=exec',
            '--property=RuntimeMaxSec=120','/usr/bin/sleep','100'])
        fields=base.service_state(standin);fake=dict(state);fake['supervisor']='\n'.join(k+'='+v for k,v in fields.items())
        fence=root/'private.lock';fence.touch(mode=0o400);s=fence.stat()
        code='''import os,sys,time
p=sys.argv[1];fd=[]
for n in os.listdir('/proc/self/fd'):
 try:
  if os.readlink('/proc/self/fd/'+n)==p:fd.append(int(n))
 except FileNotFoundError:pass
assert len(fd)==1 and os.getuid()==os.getgid()==0
copy=os.dup(fd[0]);pid=os.fork()
end=time.monotonic()+(.2 if sys.argv[2]=='healthy' else 90)
while time.monotonic()<end:os.write(1,b'owned-writer\\n');time.sleep(.01)
os._exit(0)
'''
        argv=['/usr/bin/python3','-B','-I','-c',code,str(fence),case]
    else:
        fake=state;fence=Path('/var/lib/hydracache-performance/campaigns/.host-execution.lock');s=fence.lstat();argv=None
        assert fence.is_file() and not fence.is_symlink() and s.st_uid==0 and s.st_nlink==1
    files={str(p):sha(p) for p in sorted((root/'input').rglob('*')) if p.is_file()}
    files.update({str(root/n):sha(root/n) for n in NAMES})
    plan=pmod.make(root,fake,files,fence,[s.st_dev,s.st_ino],synthetic,case,argv)
    plan['cpu_profile']=load(root/'coordinator.py','hc074_original_cpu_profile').cpu_profile()
    if synthetic:plan['expected_host']=state
    put(root/'plan.json',plan);(root/'plan.json').chmod(0o400);pin=sha(root/'plan.json')
    put(root/'prepared.json',dict(plan_sha256=pin,root=str(root),synthetic=synthetic,case=case))
    return root,pin,state


def faults(package):
    record=package/'faults-once';record.mkdir();cases=[];before=original()
    for case in ('healthy','deadline','owner-kill'):
        root,pin,_=build(package,True,case);pmod=load(root/'plan.py','hc074_launch_plan')
        argv=pmod.launch(root,pin,True);put(root/'launch-intent.json',dict(argv=argv,retry=False,case=case));command(argv)
        if case=='owner-kill':
            at=time.monotonic()
            while not (root/'0.child.json').exists() and time.monotonic()-at<10:time.sleep(.02)
            assert (root/'0.child.json').exists()
            pid=int(command(['/usr/bin/systemctl','show',root.name+'.service','--value','-p','MainPID']))
            fd=os.pidfd_open(pid)
            try:
                assert Path('/proc/'+str(pid)+'/cgroup').read_text().strip()=='0::/system.slice/'+root.name+'.service/control'
                signal.pidfd_send_signal(fd,signal.SIGKILL)
            finally:os.close(fd)
        at=time.monotonic()
        while not (root/'restored.json').exists() and time.monotonic()-at<20:time.sleep(.05)
        passed=(root/'restored.json').exists()
        if passed:
            stopped=json.loads((root/'all-writers-stopped.json').read_bytes());passed=stopped['original_fence_acquired'] and stopped['original_groups_empty']
        if case=='healthy':passed=passed and (root/'cohort-complete.json').exists()
        elif case=='deadline':passed=passed and (root/'0.deadline.json').exists() and not (root/'cohort-complete.json').exists()
        else:passed=passed and not (root/'cohort-complete.json').exists()
        journal=subprocess.run(['/usr/bin/journalctl','--no-pager','-u',root.name+'.service','-n','60','-o','short-monotonic'],capture_output=True,timeout=10)
        (root/'journal.log').write_bytes(journal.stdout)
        for unit in (root.name+'.service',root.name+'-standin.service'):
            result=subprocess.run(['/usr/bin/systemctl','stop',unit],capture_output=True,timeout=15)
            text=command(['/usr/bin/systemctl','show',unit,'-p','LoadState','-p','ActiveState','-p','MainPID'])
            assert 'MainPID=0' in text or 'LoadState=not-found' in text
            assert result.returncode==0 or result.returncode==5 and 'LoadState=not-found' in text
        cases.append(dict(case=case,root=str(root),passed=bool(passed)))
        if not passed:break
    after=original();put(record/'result.json',dict(complete=len(cases)==3 and all(x['passed'] for x in cases),cases=cases,stable_supervisor_unchanged=before==after))
    print(json.dumps(json.loads((record/'result.json').read_bytes())))


def prepare_numeric(package):
    result=json.loads((package/'faults-once/result.json').read_bytes());assert result['complete'] and result['stable_supervisor_unchanged']
    root,pin,state=build(package);put(package/'numerical.json',dict(root=str(root),plan_sha256=pin,prepared=True,launched=False))
    print(json.dumps(dict(root=str(root),plan_sha256=pin,attempts=60,prepared=True,launched=False)))


def launch_numeric(package):
    record=json.loads((package/'numerical.json').read_bytes());root=Path(record['root']);pin=record['plan_sha256'];original()
    assert sha(root/'plan.json')==pin
    pmod=load(root/'plan.py','hc074_numeric_plan');argv=pmod.launch(root,pin)
    put(root/'launch-intent.json',dict(argv=argv,retry=False,operator_approved=True));command(argv)
    print(json.dumps(dict(root=str(root),plan_sha256=pin,launched_once=True)))


if __name__=='__main__':
    assert len(sys.argv)==2 and sys.argv[1] in ('faults','prepare','launch')
    package=Path(__file__).resolve().parent
    {'faults':faults,'prepare':prepare_numeric,'launch':launch_numeric}[sys.argv[1]](package)
