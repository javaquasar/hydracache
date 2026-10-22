"""Nonroot local compile and exact filesystem tests only; never an admission runner."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import subprocess

ROOT=Path(__file__).resolve().parents[2]
PACKAGE='hydracache-long-run-supervisor-074'
QUALIFICATION='11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'

def need(value,message):
    if not value:raise ValueError(message)

def sha(path):
    digest=hashlib.sha256()
    with Path(path).open('rb') as stream:
        for chunk in iter(lambda:stream.read(1024*1024),b''):digest.update(chunk)
    return digest.hexdigest()

def publish(path,data):
    with Path(path).open('x',encoding='utf-8',newline='\n') as stream:
        json.dump(data,stream,sort_keys=True,indent=2);stream.write('\n')

def git(*args):
    output=subprocess.run(['/mnt/c/Program Files/Git/cmd/git.exe','--no-optional-locks','-C',
        'C:/Workspace/prj/jq/cashe/hydracache-074',*args],capture_output=True,check=False,timeout=20)
    need(output.returncode==0,'Git source query failed')
    return output.stdout.decode().strip()

def selection():
    value=json.loads(Path(__file__).with_name('tests.json').read_text(encoding='utf-8-sig'))
    tests=value['tests']
    need(value['schema']=='authority-runtime-filesystem-selection-074-v1','Wrong selection schema')
    need(type(tests) is list and len(tests)==33 and len(set(tests))==33,'Wrong selection membership')
    need(all(type(t) is str and re.fullmatch(r'diagnostic_worker_policy::authority_registry::fixture::[A-Za-z0-9_:]+',t) for t in tests),'Unsafe test selection')
    need(sum('::authority_runtime_' in t for t in tests)==7,'Wrong new filesystem membership')
    return tests

def image(stdout):
    found=[]
    for line in stdout.decode().splitlines():
        row=json.loads(line)
        if row.get('reason')=='compiler-artifact' and row.get('executable') and row['target']['name']=='hydracache_long_run_supervisor_074' and row['profile']['test']:
            found.append(row['executable'])
    need(len(found)==1,'Exactly one lib test executable required')
    return found[0]

def test_passed(stdout,name):
    text=stdout.decode()
    need(re.search(r'(?m)^running 1 test$',text) and re.search(r'(?m)^test '+re.escape(name)+r' \.\.\. ok$',text),'Exact native test selection failed')
    need(re.search(r'test result: ok\. 1 passed; 0 failed; 0 ignored; 0 measured; \d+ filtered out',text),'Filesystem test failed or ignored')

def snapshot():
    configs=[Path('/etc/wsl.conf'),Path('/etc/systemd/timesyncd.conf'),Path('/mnt/c/Users/ArturBuzov/.wslconfig')]
    return dict(source=git('rev-parse','HEAD'),tracked_dirty=git('status','--porcelain','--untracked-files=no'),
        boot=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),kernel=os.uname().release,
        uid=os.getuid(),gid=os.getgid(),qualification=sha(ROOT/'docs/testing/performance/0.74/qualification-manifest.toml'),
        configs={str(p):sha(p) if p.exists() else None for p in configs},
        driver=sha(Path(__file__)),selection=sha(Path(__file__).with_name('tests.json')))

def invoke(args,output,name,steps,env,timeout=1200):
    process=subprocess.Popen(args,cwd=ROOT,env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True)
    timed_out=False
    try:stdout,stderr=process.communicate(timeout=timeout)
    except subprocess.TimeoutExpired:
        timed_out=True;os.killpg(process.pid,signal.SIGKILL);stdout,stderr=process.communicate()
    for suffix,data in [('stdout.log',stdout),('stderr.log',stderr)]:
        with (output/(name+'.'+suffix)).open('xb') as stream:stream.write(data)
    steps.append(dict(name=name,argv=args,exit=process.returncode,timeout=timed_out,
        stdout_sha256=sha(output/(name+'.stdout.log')),stderr_sha256=sha(output/(name+'.stderr.log'))))
    need(not timed_out and process.returncode==0,'Local step refused: '+name)
    return stdout

def run(output,lane,phase):
    need(ROOT.as_posix()=='/mnt/c/Workspace/prj/jq/cashe/hydracache-074','Unexpected local repository')
    need(output.parent.resolve()==(ROOT/'target').resolve() and output.name.startswith('authority-runtime-'),'Separate local output required')
    need(not output.exists() and not output.is_symlink(),'Never overwrite first lane')
    output.mkdir();steps=[];failure=None;before=None;after=None;executable=None;digest=None
    try:
        need(os.getuid()>0 and os.getgid()>0,'Nonroot local fixture owner required')
        need(not any(key.startswith('HC074_') for key in os.environ),'No live harness environment')
        tests=selection();before=snapshot();need(before['qualification']==QUALIFICATION,'Qualification drift')
        if phase=='final':need(not before['tracked_dirty'],'Final method must be committed and clean')
        target='/home/artur/.cache/hydracache-074-runtime-20261022-'+lane
        env=dict(os.environ,CARGO_TARGET_DIR=target,CARGO_NET_OFFLINE='true',
            PATH='/home/artur/.cargo/bin:'+os.environ.get('PATH','/usr/bin:/bin'))
        toolchain={'194':'1.94.0','188':'1.88.0'}[lane]
        invoke(['/home/artur/.cargo/bin/rustc','+'+toolchain,'--version'],output,'compiler',steps,env)
        common=['--offline','--locked','-p',PACKAGE,'--all-features']
        invoke(['/home/artur/.cargo/bin/cargo','+'+toolchain,'check',*common,'--all-targets'],output,'check',steps,env)
        invoke(['/home/artur/.cargo/bin/cargo','+'+toolchain,'clippy',*common,'--all-targets','--','-D','warnings'],output,'clippy',steps,env)
        built=invoke(['/home/artur/.cargo/bin/cargo','+'+toolchain,'test',*common,'--lib','--no-run','--message-format=json'],output,'build',steps,env)
        executable=image(built);need(Path(executable).is_file() and Path(executable).resolve().is_relative_to(Path(target).resolve()),'Wrong executable scope')
        digest=sha(executable)
        listed=invoke([executable,'--list','--format','terse'],output,'list',steps,env)
        names={line[:-6] for line in listed.decode().splitlines() if line.endswith(': test')}
        need(set(tests)<=names,'Frozen selection missing from compiled image')
        for index,name in enumerate(tests):
            stdout=invoke([executable,name,'--exact','--nocapture','--test-threads=1'],output,f'test-{index:03}',steps,env,timeout=120)
            test_passed(stdout,name)
        need(sha(executable)==digest,'Test image changed during cohort')
        after=snapshot();need(after==before,'Source boot credentials config or qualification changed')
    except Exception as error:failure=type(error).__name__+': '+str(error)
    publish(output/'result.json',dict(schema='authority-runtime-local-linux-074-v1',lane=lane,phase=phase,
        before=before,after=after,steps=steps,failure=failure,image=executable,image_sha256=digest,
        server=False,product=False,live_profiles=0,ignored_measurements=False,clocks_changed=False,services_changed=False))
    need(failure is None,failure)

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--lane',choices=('194','188'),required=True)
    parser.add_argument('--phase',choices=('development','final'),required=True)
    parser.add_argument('--output',type=Path,required=True);args=parser.parse_args()
    run(args.output,args.lane,args.phase)
