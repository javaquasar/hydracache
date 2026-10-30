"""Bounded real consumer compilation, including the three mixed-type refusals."""
import json,os,subprocess,sys,time,traceback
from pathlib import Path
import caller_context as s
c=s.c;OUT=None
parent_runner=c.load(s.ROOT/'tools/chitchat-copied-root-074/runner.py','hc074_actual_caller_parent_mirror')
PHASES=('stage','resolve','194','188')

def invoke(label,argv,env,deadline,expect=0,mutate=False,bound=780):
    before=s.source();head=c.snapshot();s.parent.capacity()
    if not mutate:s.actual()
    else:s.parent.actual()
    out=OUT/label;out.mkdir();image=c.sha(Path(argv[0]).read_bytes())
    c.write(out,'intent.json',dict(argv=argv,env=env,cwd=str(s.N),deadline=deadline,bound=bound,expect=expect,image_sha256=image,retry=False))
    with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
        child=subprocess.run([sys.executable,'-B',str(Path(__file__)),'command',str(out)],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,env=env,timeout=max(0,min(bound,deadline-time.monotonic()))+10)
    owner=json.loads((out/'execution.json').read_bytes());c.need(child.returncode==owner['exit']==expect and owner['retired'] and not owner['errors'],'exact first actual consumer owner: '+label)
    c.need(s.source()==before and c.snapshot()==head and c.sha(Path(argv[0]).read_bytes())==image,'whole caller source/ref/image postflight')
    if not mutate:s.actual()
    else:s.parent.actual()
    return out

def stage():
    c.need(not s.N.exists(),'exclusive real caller directory');s.N.mkdir(parents=True,mode=0o700);(s.BASE/'state').mkdir()
    for v in s.VARIANTS:
        root=s.N/'source'/v;(root/'src').mkdir(parents=True);(root/'Cargo.toml').write_bytes(s.manifest(v));(root/'src/lib.rs').write_bytes((s.HERE/'caller.rs').read_bytes())
        c.write(s.BASE/'state','staged-'+v+'.json',c.tree(root))
    (s.N/'cargo-home').mkdir();raw=(s.parent.N/'cargo-home/config.toml').read_bytes();(s.N/'cargo-home/config.toml').write_bytes(raw)
    c.write(s.BASE/'state','config.json',dict(sha256=c.sha(raw)))

def resolve(tool,env,deadline):
    for v in s.VARIANTS:
        root=s.N/'source'/v;before=c.tree(root)
        out=invoke('metadata-'+v,[str(tool/'cargo'),'metadata','--manifest-path',str(root/'Cargo.toml'),'--all-features','--format-version','1'],dict(env,CARGO_NET_OFFLINE='false',CARGO_NET_RETRY='0'),deadline,mutate=True)
        after=c.tree(root);c.need(after.pop('Cargo.lock') and before==after,'only exact normal caller lock added')
        data=json.loads((out/'stdout.log').read_bytes());review=s.graph(data,v);c.write(out,'graph-review.json',review)
        for p in data['packages']:
            if p['name']!='hydra-chitchat':continue
            folder=Path(p['manifest_path']).parent;c.need(folder.is_relative_to(s.N/'cargo-home/registry/src') and not folder.is_symlink(),'contained actual normal registry source')
            values=c.tree(folder);values.pop('.cargo-ok',None);values.pop('.cargo-checksum.json',None);s.download((s.parent.INPUT/'owned.crate').read_bytes(),values)
            row=dict(path=str(folder),files=values,source=p['source'],version=p['version']);frame=s.BASE/'state/owned-download.json'
            if frame.exists():c.need(json.loads(frame.read_bytes())==row,'one exact shared normal owned download for actual callers')
            else:c.write(s.BASE/'state','owned-download.json',row)
        lock=(root/'Cargo.lock').read_bytes();s.parent.c.need(b'version = 4' in lock,'normal current Cargo caller lock')
        import tomllib
        ledger=json.loads((s.parent.INPUT/'registry.json').read_bytes());reference=json.loads((s.parent.INPUT/'registry-reference.json').read_bytes())
        for p in tomllib.loads(lock.decode())['package']:
            if p.get('source'):
                if p['name']=='hydra-chitchat':c.need(p['checksum']==s.parent.abi.CRATE_SHA and p['source']==reference['registry'],'whole owned dependency checksum/source')
                else:c.need(p['source']=='registry+https://github.com/rust-lang/crates.io-index' and ledger[p['name']+'-'+p['version']]==p['checksum'],'whole ordinary dependency checksum/source')
        c.write(s.BASE/'state','resolved-'+v+'.json',c.tree(root));(s.BASE/'state'/('lock-'+v+'.toml')).write_bytes(lock)
        (s.BASE/'state'/('metadata-'+v+'.json')).write_bytes((out/'stdout.log').read_bytes());s.actual()

def negative(raw,source,package_id):
    rows=[json.loads(line) for line in raw.decode().splitlines() if line.startswith('{')]
    messages=[r for r in rows if r.get('reason')=='compiler-message' and r['message']['level']=='error'];c.need(all(r['package_id']==package_id for r in messages),'errors belong only to the actual mixed caller')
    errors=[r['message'] for r in messages]
    codes=sorted(e['code']['code'] for e in errors if e.get('code'));c.need(codes==['E0277','E0308','E0308'] and len(errors)==3,'only the three actual public boundary refusals')
    lines=source.decode().splitlines();wanted={i:('E0277' if 'spawn_with_transport(config, channel)' in line else 'E0308') for i,line in enumerate(lines,1) if 'config.failure_detector_config(detector)' in line or 'discovery.chitchat_id()' in line or 'spawn_with_transport(config, channel)' in line}
    got=set()
    for error in errors:
        spans=[x for x in error['spans'] if x['is_primary']];c.need(len(spans)==1 and spans[0]['file_name'] in ('src/lib.rs',str(s.N/'source/mixed/src/lib.rs')),'actual original typed caller span')
        line=spans[0]['line_start'];c.need(wanted.get(line)==error['code']['code'],'exact expected code at each actual boundary');got.add(line)
    c.need(got==set(wanted) and len(wanted)==3,'all three real call sites, no unrelated compile refusal')
    finished=[r for r in rows if r.get('reason')=='build-finished'];c.need(len(finished)==1 and not finished[0]['success'],'actual refused mixed build')
    return dict(complete=True,codes=codes,lines=sorted(got),expected_refusal=True,root_adopted=False)

def compile_callers(compiler,tool,env,deadline):
    s.admission(compiler)
    version=invoke('rustc-version',[str(tool/'rustc'),'-vV'],env,deadline,bound=10)
    c.need(('release: '+('1.94.0' if compiler=='194' else '1.88.0')+'\n').encode() in (version/'stdout.log').read_bytes(),'actual pinned caller compiler')
    for v in ('B','O'):
        args=['--manifest-path',str(s.N/'source'/v/'Cargo.toml'),'--target-dir',str(s.parent.N/'builds'/compiler),'--locked','--offline','--all-targets','--all-features']
        invoke('check-'+v,[str(tool/'cargo'),'check',*args],env,deadline)
        invoke('lint-'+v,[str(tool/'cargo'),'clippy',*args,'--','-D','warnings'],env,deadline)
    args=['--manifest-path',str(s.N/'source/mixed/Cargo.toml'),'--target-dir',str(s.parent.N/'builds'/compiler),'--locked','--offline','--all-targets','--all-features','--message-format=json']
    out=invoke('negative',[str(tool/'cargo'),'check',*args],env,deadline,expect=101)
    metadata=json.loads((s.BASE/'state/metadata-mixed.json').read_bytes());c.need(len(metadata['workspace_members'])==1,'one actual mixed caller root')
    c.write(out,'negative-review.json',negative((out/'stdout.log').read_bytes(),(s.HERE/'caller.rs').read_bytes(),metadata['workspace_members'][0]))

def session(phase):
    global OUT
    OUT=s.BASE/'native'/phase;deadline=time.monotonic()+1100;result=dict(complete=False,root_adopted=False,release_admitted=False)
    try:
        c.need(phase in PHASES and os.getuid()==os.getgid()==1000,'fixed nonroot actual caller session');s.parent.capacity();before=s.source();boot=Path('/proc/sys/kernel/random/boot_id').read_bytes()
        tool,env,_=s.environment(phase if phase in ('194','188') else '194')
        if phase=='stage':stage()
        else:
            required='stage' if phase=='resolve' else 'resolve';c.need(json.loads((s.BASE/'native'/required/'result.json').read_bytes())['complete'],'actual caller parent prerequisite')
            s.actual();parent_runner.mirror(deadline)
            if phase=='resolve':resolve(tool,env,deadline)
            else:compile_callers(phase,tool,env,deadline)
            parent_runner.mirror(deadline);s.actual()
        c.need(s.source()==before and Path('/proc/sys/kernel/random/boot_id').read_bytes()==boot and time.monotonic()<deadline,'whole caller source/boot/deadline postflight');result['complete']=True
    except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
    c.write(OUT,'session-result.json',result);raise SystemExit(0 if result['complete'] else 1)

def outer(phase):
    c.need(phase in PHASES and not (s.BASE/'STOP.json').exists(),'fixed phase, never reopen first caller refusal')
    out=s.BASE/'native'/phase;out.mkdir(parents=True,exist_ok=False);before=s.source();head=c.snapshot()
    argv=[str(Path(os.environ['SystemRoot'])/'System32/wsl.exe'),'-d','Ubuntu','-u','artur','--cd','/','--exec','/usr/bin/python3','-B','/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/chitchat-copied-root-caller-074/runner.py','session',phase]
    c.write(out,'intent.json',dict(argv=argv,source=before,git=head,bound=1200,session_bound=1100,floor=4*1024**3,poll=5,cleanup=5,retry=False))
    result=dict(complete=False,root_adopted=False,release_admitted=False)
    with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
        try:
            child=subprocess.run([sys.executable,'-B',str(Path(__file__)),'outer-worker',phase],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,timeout=1210)
            own=json.loads((out/'execution.json').read_bytes());c.need(child.returncode==own['exit']==0 and own['retired'] and not own['errors'],'complete actual caller outer owner')
            c.need(s.source()==before and c.snapshot()==head and json.loads((out/'session-result.json').read_bytes())['complete'],'whole actual caller source/ref/session postflight');result['complete']=True
        except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
    c.write(out,'result.json',result)
    if not result['complete']:c.write(s.BASE,'STOP.json',dict(phase=phase,first_refusal=result['first_refusal'],root_adopted=False))
    print(json.dumps(dict(phase=phase,**result)));raise SystemExit(0 if result['complete'] else 1)

if __name__=='__main__':
    action=sys.argv[1];c.need(not (s.BASE/'STOP.json').exists(),'first caller refusal remains terminal')
    if action=='run':outer(sys.argv[2])
    elif action=='session':session(sys.argv[2])
    elif action=='outer-worker':
        phase=sys.argv[2];out=s.BASE/'native'/phase;intent=json.loads((out/'intent.json').read_bytes());_,env,_=s.environment('194');os.environ.clear();os.environ.update(env)
        own=c.owned._owned(c.owned.load_job().WindowsJob(),intent['argv'],str(s.ROOT),s.parent.capacity,bound=1200);c.write(out,'execution.json',own);raise SystemExit(own['exit'])
    elif action=='command':
        out=Path(sys.argv[2]);c.need(out.resolve().is_relative_to((s.BASE/'native').resolve()),'contained current caller output');intent=json.loads((out/'intent.json').read_bytes());os.environ.clear();os.environ.update(intent['env'])
        group=c.load(s.ROOT/'scripts/ci/watchdog_linux_group.py','actual_caller_original_group').LinuxGroup()
        own=c.owned._owned(group,intent['argv'],intent['cwd'],s.parent.capacity,bound=intent['bound'],deadline=min(intent['deadline'],time.monotonic()+intent['bound']));c.write(out,'execution.json',own);raise SystemExit(own['exit'])
    else:raise ValueError('fixed actual caller action')
