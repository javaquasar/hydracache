"""One complete library run per compiler, then separately gated registry/callers."""
import json,os,shutil,subprocess,sys,time,traceback
from pathlib import Path
import context as s
m=s.m;c=s.c;OUT=None

def invoke(name,argv,env,deadline,bound=1200,cwd=None,allow_mutation=False,expect=0):
    before=s.source();head=c.snapshot();s.capacity()
    if not allow_mutation:s.actual()
    folder=OUT/name;folder.mkdir();image=c.sha(Path(argv[0]).read_bytes())
    c.write(folder,'intent.json',dict(argv=argv,env=env,cwd=str(cwd or s.N),deadline=deadline,bound=bound,image_sha256=image,retry=False))
    with (folder/'stdout.log').open('xb') as stdout,(folder/'stderr.log').open('xb') as stderr:
        child=subprocess.run([sys.executable,'-B',str(Path(__file__)),'command',str(folder)],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,env=env,timeout=max(0,min(bound,deadline-time.monotonic()))+10)
    own=json.loads((folder/'execution.json').read_bytes());c.need(child.returncode==own['exit']==expect and own['retired'] and not own['errors'],'first complete full-lane command: '+name)
    c.need(s.source()==before and c.snapshot()==head and c.sha(Path(argv[0]).read_bytes())==image,'whole original full-lane source/ref/image postflight')
    if not allow_mutation:s.actual()
    return folder

def session(phase):
    global OUT
    OUT=s.BASE/'native'/phase;deadline=time.monotonic()+1100;result=dict(complete=False,root_adopted=False,public_registry_qualified=False,release_admitted=False)
    try:
        c.need(os.getuid()==os.getgid()==1000,'original local nonroot GNU owner');s.capacity();before=s.source();boot=Path('/proc/sys/kernel/random/boot_id').read_bytes()
        for name in s.prerequisites(phase):c.need(json.loads((s.BASE/'native'/name/'result.json').read_bytes())['complete'],'complete full-lane prerequisite: '+name)
        tool,env,_=s.environment('188' if phase.endswith('188') else '194')
        if phase=='stage':
            c.need(not s.N.exists(),'exclusive new full-lane native context');s.N.mkdir(parents=True,mode=0o700)
            shutil.copytree(s.BASE/'source',s.N/'source');(s.N/'cargo-home').mkdir()
            config='[source.crates-io]\nreplace-with="reviewed-upstream"\n[source.reviewed-upstream]\ndirectory="'+s.prior.VENDOR.as_posix()+'"\n'
            (s.N/'cargo-home/config.toml').write_text(config,encoding='utf8')
            c.write(s.BASE/'state','source.json',c.tree(s.N/'source'));c.write(s.BASE/'state','cargo-config.toml.json',dict(sha256=c.sha(config.encode())))
        else:
            s.actual();s.prior.registry(deadline)
            if phase.startswith('library'):
                compiler=phase[-3:];image=json.loads((s.PARENT/'state'/('image-'+compiler+'.json')).read_bytes());names=json.loads((s.PARENT/'state'/('catalog-'+compiler+'.json')).read_bytes())
                folder=invoke('full-library',[image['path'],'--test-threads=1','--nocapture'],env,deadline,780,cwd=s.prior.NATIVE/'extracted')
                c.need(json.loads((folder/'intent.json').read_bytes())['image_sha256']==image['sha256'],'same fresh original extracted-package image')
                c.write(OUT,'library-review.json',s.full_outcome((folder/'stdout.log').read_bytes(),names))
            else:
                calls=s.registry_functions(OUT,invoke)
                if phase=='registry':calls.registry_phase(env,tool,deadline)
                else:
                    version=invoke('rustc-version',[str(tool/'rustc'),'-vV'],env,deadline,10)
                    c.need(('release: '+('1.88.0' if phase.endswith('188') else '1.94.0')+'\n') in (version/'stdout.log').read_text(),'actual pinned caller compiler')
                    calls.consumer_phase(phase[-3:],env,tool,deadline)
            s.prior.registry(deadline);s.actual()
        c.need(s.source()==before and Path('/proc/sys/kernel/random/boot_id').read_bytes()==boot and time.monotonic()<deadline,'whole source/boot and original1100 deadline')
        result['complete']=True
    except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
    c.write(OUT,'session-result.json',result);raise SystemExit(0 if result['complete'] else 1)

def outer(phase):
    c.need(phase in s.PHASES and not (s.BASE/'STOP.json').exists(),'fixed full-lane phase; first refusal stops')
    out=s.BASE/'native'/phase;out.mkdir(parents=True,exist_ok=False);before=s.source();head=c.snapshot()
    argv=[str(Path(os.environ['SystemRoot'])/'System32/wsl.exe'),'-d','Ubuntu','-u','artur','--cd','/','--exec','/usr/bin/python3','-B','/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/chitchat-package-complete-074/run.py','session',phase]
    c.write(out,'intent.json',dict(argv=argv,source=before,git=head,bound=1200,session_bound=1100,floor=4*1024**3,poll=5,cleanup=5,retry=False))
    result=dict(complete=False,root_adopted=False,public_registry_qualified=False,release_admitted=False)
    with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
        try:
            child=subprocess.run([sys.executable,'-B',str(Path(__file__)),'outer-worker',phase],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,timeout=1210)
            own=json.loads((out/'execution.json').read_bytes());c.need(child.returncode==own['exit']==0 and own['retired'] and not own['errors'],'complete original full-lane outer owner')
            c.need(s.source()==before and c.snapshot()==head and json.loads((out/'session-result.json').read_bytes())['complete'],'whole full-lane source/ref/session postflight');result['complete']=True
        except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
    c.write(out,'result.json',result)
    if not result['complete']:
        try:c.write(s.BASE,'STOP.json',dict(phase=phase,first_refusal=result['first_refusal'],root_adopted=False))
        except FileExistsError:pass
    print(json.dumps(dict(phase=phase,**result)));raise SystemExit(0 if result['complete'] else 1)

if __name__=='__main__':
    action=sys.argv[1]
    c.need(not (s.BASE/'STOP.json').exists(),'first full-lane refusal stops unchanged dispatch')
    if action=='prepare':s.prepare()
    elif action=='run':outer(sys.argv[2])
    elif action=='session':session(sys.argv[2])
    elif action=='outer-worker':
        phase=sys.argv[2];out=s.BASE/'native'/phase;intent=json.loads((out/'intent.json').read_bytes());_,env,_=s.environment('194');os.environ.clear();os.environ.update(env)
        own=c.owned._owned(c.owned.load_job().WindowsJob(),intent['argv'],str(m.ROOT),s.capacity,bound=1200);c.write(out,'execution.json',own);raise SystemExit(own['exit'])
    elif action=='command':
        folder=Path(sys.argv[2]);c.need(folder.resolve().is_relative_to((s.BASE/'native').resolve()),'contained original full-lane command')
        intent=json.loads((folder/'intent.json').read_bytes());os.environ.clear();os.environ.update(intent['env'])
        group=c.load(m.ROOT/'scripts/ci/watchdog_linux_group.py','hc074_full_lane_group').LinuxGroup()
        own=c.owned._owned(group,intent['argv'],intent['cwd'],s.capacity,bound=intent['bound'],deadline=min(intent['deadline'],time.monotonic()+intent['bound']));c.write(folder,'execution.json',own);raise SystemExit(own['exit'])
    else:raise ValueError('fixed full-lane action')
