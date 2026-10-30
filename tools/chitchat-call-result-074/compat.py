"""Additional semantic peers, not repeated allocation/CPU observations."""
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time
import traceback
import review as a
sys.path.insert(0,str(a.ROOT/'tools/chitchat-call-context-074'))
import recipe as p
import native_codegen as g
import run as w
c=p.c
NATIVE=Path('/home/artur/.cache/hydracache-074-call-context/20261030-r1')
PEERS={'gnu-'+v+'-'+compiler for v in ('B0','C2') for compiler in ('194','188')}
NEGATIVES=('serialize::security_stream_review::minimal_bound_contract','serialize::security_stream_review::all_stream_bounds_hold')


def source():
    value=c.source()
    for path in p.HERE.iterdir():
        if path.is_file():value[path.relative_to(c.ROOT).as_posix()]=c.sha(path.read_bytes())
    value[Path(__file__).relative_to(c.ROOT).as_posix()]=c.sha(Path(__file__).read_bytes())
    value.update(g.held());return value


def capacity():
    c.capacity()
    if os.name=='posix' and shutil.disk_usage(NATIVE).free<4*1024**3:raise c.owned.CapacityError('original native four-GiB floor')


def session(peer):
    out=p.BASE/'semantic'/peer;_,v,compiler=peer.split('-');deadline=time.monotonic()+1100
    result=dict(complete=False,root_adopted=False,mtu_repaired=False,allocation_measurement=False)
    try:
        c.need(os.getuid()==os.getgid()==1000,'native nonroot semantic peer')
        c.need(json.loads((p.BASE/'analysis/result.json').read_bytes())['prerequisite_passed'],'native resource prerequisite first')
        root=NATIVE/'fixtures'/v if compiler=='194' else NATIVE/'functional188/fixtures'/v
        if compiler=='188':
            c.need(not root.exists(),'fresh MSRV source context');shutil.copytree(p.BASE/'fixtures'/v,root)
        expected=c.tree((p.BASE/'native-input-r1/fixtures'/v) if compiler=='194' else p.BASE/'fixtures'/v)
        def guard():c.need(c.tree(root)==expected,'whole actual semantic source frame');capacity()
        core=c.load(g.CORE,'call_semantic_qualified_owner')
        original=core.source();guard()
        registry=c.load(c.HERE/'runner.py','call_semantic_registry');registry.registry(deadline)
        tool,env,_=c.environment(compiler)
        def invoke(name,argv,bound=1200,exit_code=0):
            guard();folder=out/name;folder.mkdir(exist_ok=False)
            c.write(folder,'intent.json',dict(argv=argv,env=env,cwd=str(root),bound=bound,deadline=deadline,
              image_sha256=c.sha(Path(argv[0]).read_bytes()),expected_exit=exit_code,retry=False))
            with (folder/'stdout.log').open('xb') as stdout,(folder/'stderr.log').open('xb') as stderr:
                child=subprocess.run([sys.executable,'-B',str(Path(__file__)),'command',str(folder)],stdin=subprocess.DEVNULL,
                  stdout=stdout,stderr=stderr,env=env,timeout=max(0,min(bound,deadline-time.monotonic()))+10)
            own=json.loads((folder/'execution.json').read_bytes())
            c.need(child.returncode==own['exit']==exit_code and own['retired'] and not own['errors'],'first original semantic command: '+name)
            c.need(c.sha(Path(argv[0]).read_bytes())==json.loads((folder/'intent.json').read_bytes())['image_sha256'],'actual command image postflight')
            guard();return folder
        compiler_out=invoke('compiler',[str(tool/'rustc'),'-vV'])
        c.need('release: '+('1.94.0' if compiler=='194' else '1.88.0') in (compiler_out/'stdout.log').read_text(),'actual compiler')
        if compiler=='194':image=json.loads((p.BASE/'native-r1'/('image-'+v+'.json')).read_bytes())
        else:
            target=NATIVE/'functional188/builds'/v
            args=['--manifest-path',str(root/'Cargo.toml'),'--target-dir',str(target),'--locked','--offline','--release','--all-features']
            built=invoke('build',[str(tool/'cargo'),'test',*args,'--lib','--no-run','--message-format=json'])
            rows=[json.loads(s) for s in (built/'stdout.log').read_text().splitlines() if s.startswith('{')]
            artifacts=[r for r in rows if r.get('reason')=='compiler-artifact' and r.get('executable') and r['target']['kind']==['lib'] and r['profile']['test']]
            c.need(len(artifacts)==1 and artifacts[0]['fresh'] is False and [r['success'] for r in rows if r.get('reason')=='build-finished']==[True],'fresh complete MSRV image')
            c.need(Path(artifacts[0]['target']['src_path']).resolve()==root/'src/lib.rs','actual MSRV source')
            path=Path(artifacts[0]['executable']);c.need(path.resolve().is_relative_to(target),'isolated compiler/input target');image=dict(path=str(path),sha256=c.sha(path.read_bytes()))
            for action in ('check','clippy'):
                argv=[str(tool/'cargo'),action,*args,'--all-targets']
                if action=='clippy':argv+=['--','-D','warnings']
                invoke(action,argv)
        c.need(c.sha(Path(image['path']).read_bytes())==image['sha256'],'held actual semantic image');c.write(out,'image.json',image)
        listed=invoke('catalog',[image['path'],'--list'],10)
        raw=(listed/'stdout.log').read_text();names=sorted(s[:-6] for s in raw.splitlines() if s.endswith(': test'))
        original_names=json.loads((p.BASE/('catalog-'+v)/'result.json').read_bytes())['catalog']
        extra=['serialize::native_lifetime::allocator_controls','serialize::native_lifetime::context_lifetime_observation'] if compiler=='194' else []
        c.need(names==sorted(original_names+extra) and len(names)==len(set(names)) and raw.splitlines()[-1]==f'{len(names)} tests, 0 benchmarks','complete actual semantic catalog')
        selected=[n for n in names if n.startswith(w.PURE) or n in w.OBS or v=='C2' and n.startswith('serialize::call_context_review::')]
        negatives=list(NEGATIVES)+([w.LEGACY] if v=='C2' else [])
        c.need(len(selected)==(63 if v=='B0' else 70) and all(n in names for n in negatives),'complete original semantic selection')
        c.write(out,'catalog.json',dict(all=names,selected=selected,negative=negatives,unexecuted=[n for n in names if n not in selected and n not in negatives],full_library_passed=False))
        for i,test in enumerate(selected+negatives):
            negative=test in negatives;folder=invoke(f'test-{i:03}',[image['path'],'--exact',test,'--test-threads=1','--nocapture'],5,101 if negative else 0)
            text=(folder/'stdout.log').read_text()
            c.need(re.findall(r'^test (.+?) (?:- should panic )?\.\.\. ',text,re.M)==[test],'one exact actual case')
            summaries=re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',text,re.M)
            c.need(summaries==[('FAILED' if negative else 'ok','0' if negative else '1','1' if negative else '0','0','0',str(len(names)-1))],'complete case accounting')
        c.need(core.source()==original,'fresh complete Root/helper source postflight');registry.registry(deadline);guard()
        c.need(c.sha(Path(image['path']).read_bytes())==image['sha256'] and time.monotonic()<deadline,'original image/session postflight')
        result.update(complete=True,passed=len(selected),expected_negative=len(negatives),full_library_passed=False)
    except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
    c.write(out,'session-result.json',result);raise SystemExit(0 if result['complete'] else 1)


def outer(peer):
    c.need(peer in PEERS,'fixed semantic peer');stop=p.BASE/'semantic-stop.json';c.need(not stop.exists(),'stop after first unexpected semantic refusal')
    out=p.BASE/'semantic'/peer;out.mkdir(parents=True,exist_ok=False);before=source();head=c.snapshot();fixtures=c.tree(p.BASE/'fixtures')
    argv=[str(Path(os.environ['SystemRoot'])/'System32/wsl.exe'),'-d','Ubuntu','-u','artur','--cd','/','--exec','/usr/bin/python3','-B',
      '/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/chitchat-call-result-074/compat.py','session',peer]
    c.write(out,'intent.json',dict(argv=argv,source=before,git=head,fixtures=fixtures,bound=1200,session_bound=1100,floor=4*1024**3,poll=5,cleanup=5,retry=False))
    result=dict(complete=False,root_adopted=False,allocation_measurement=False)
    with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
        try:
            child=subprocess.run([sys.executable,'-B',str(Path(__file__)),'outer-worker',peer],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,timeout=1210)
            own=json.loads((out/'execution.json').read_bytes());c.need(child.returncode==own['exit']==0 and own['retired'] and not own['errors'],'complete original outer semantic Job')
            c.need(source()==before and c.snapshot()==head and c.tree(p.BASE/'fixtures')==fixtures,'whole Root/source/ref/input postflight')
            c.need(json.loads((out/'session-result.json').read_bytes())['complete'],'complete first semantic session');result['complete']=True
        except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
    c.write(out,'result.json',result)
    if not result['complete']:c.write(p.BASE,stop.name,dict(peer=peer,first_refusal=result.get('first_refusal')))
    print(json.dumps(dict(peer=peer,**result)));raise SystemExit(0 if result['complete'] else 1)


if __name__=='__main__':
    if len(sys.argv)==2:outer(sys.argv[1])
    elif sys.argv[1]=='session':session(sys.argv[2])
    elif sys.argv[1]=='outer-worker':
        out=p.BASE/'semantic'/sys.argv[2];intent=json.loads((out/'intent.json').read_bytes());_,env,_=c.environment('194');os.environ.clear();os.environ.update(env)
        own=c.owned._owned(c.owned.load_job().WindowsJob(),intent['argv'],str(c.ROOT),capacity,bound=1200);c.write(out,'execution.json',own);raise SystemExit(own['exit'])
    elif sys.argv[1]=='command':
        folder=Path(sys.argv[2]);c.need(folder.resolve().is_relative_to((p.BASE/'semantic').resolve()),'contained semantic output');intent=json.loads((folder/'intent.json').read_bytes())
        os.environ.clear();os.environ.update(intent['env']);group=c.load(c.ROOT/'scripts/ci/watchdog_linux_group.py','call_semantic_group').LinuxGroup()
        own=c.owned._owned(group,intent['argv'],intent['cwd'],capacity,bound=intent['bound'],deadline=min(intent['deadline'],time.monotonic()+intent['bound']))
        c.write(folder,'execution.json',own);raise SystemExit(own['exit'])
    else:raise ValueError('fixed semantic modes')
