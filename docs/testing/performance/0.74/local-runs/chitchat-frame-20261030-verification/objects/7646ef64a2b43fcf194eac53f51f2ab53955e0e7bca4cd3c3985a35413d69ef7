"""Qualified C2 pure-case envelope plus unchanged full native invoke controls."""
import json,os,re,subprocess,sys,time,traceback
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tools/chitchat-frame-result-074'));import native as core
p=core.p;c=core.c
sys.path.insert(0,str(p.HERE));import run as windows

def source():
    value=core.source();value[Path(__file__).relative_to(ROOT).as_posix()]=c.sha(Path(__file__).read_bytes());return value

def session(compiler):
    out=p.BASE/('native-semantic'+compiler);core.OUT=out
    deadline=time.monotonic()+1100;result=dict(complete=False,root_adopted=False,resources_qualified=False)
    try:
        c.need(os.getuid()==os.getgid()==1000,'nonroot semantic owner');core.guard()
        c.need(json.loads((p.BASE/'native-build/result.json').read_bytes())['complete'],'complete original native build')
        before=source();registry=c.load(c.HERE/'runner.py','frame_semantic_registry');registry.registry(deadline)
        _,env=core.environment(compiler)
        for key in (('B194','P194','G194') if compiler=='194' else ('G188',)):
            image=json.loads((core.STATE/('image-'+key+'.json')).read_bytes())
            names=json.loads((core.STATE/('catalog-'+key+'.json')).read_bytes());expected=c.tree(core.NATIVE/'fixtures')
            def guard():
                core.capacity();c.need(c.tree(core.NATIVE/'fixtures')==expected,'fresh complete actual fixture')
                c.need(c.sha(Path(image['path']).read_bytes())==image['sha256'],'held exact semantic image')
            controls=['delta::cost_review::'+n for n in ('allocator_control','native_c_allocator_control','native_rust_tag_control','c_checkpoint_positive_and_retained_negative_control')]
            controls+=['serialize::stateful_review::'+n for n in ('stateful_semantics_control','stateful_capacity_control')]
            controls+=['delta::cost_review::actual_caller::'+n for n in ('fixed_caller_semantics_control','fixed_caller_capacity_control')]
            pure=[]
            if key.startswith('G'):
                pure=[n for n in names if n.startswith(windows.PURE+('serialize::call_context_review::','serialize::fused_review::','delta::fused_review::','serialize::frame_review::')) or n in windows.OBS]
                pure+=['serialize::security_stream_review::minimal_bound_contract','serialize::security_stream_review::all_stream_bounds_hold','serialize::retained_finish::retained_finish_output']
            selected=controls+pure;c.need(len(set(selected))==len(selected) and len(controls)==8 and (not pure or len(pure)==87),'complete unique original selection')
            c.write(out,'selection-'+key+'.json',dict(selected=selected,unexecuted=[n for n in names if n not in selected],full_library_passed=False,
              pure_envelope='unchanged C2 compat: full Root entry/exit; fresh fixture/image per case',full_native_controls=controls))
            for i,test in enumerate(controls):
                folder=core.invoke('control-'+key+f'-{i:02}',[image['path'],'--exact',test,'--test-threads=1','--nocapture'],env,deadline,5)
                core.account(folder,test,names)
            for i,test in enumerate(pure):
                guard();folder=out/('case-'+key+f'-{i:03}');folder.mkdir(exist_ok=False)
                argv=[image['path'],'--exact',test,'--test-threads=1','--nocapture']
                c.write(folder,'intent.json',dict(argv=argv,env=env,cwd=str(core.NATIVE/'fixtures/G'),bound=5,deadline=deadline,image_sha256=image['sha256'],retry=False))
                with (folder/'stdout.log').open('xb') as stdout,(folder/'stderr.log').open('xb') as stderr:
                    child=subprocess.run([sys.executable,'-B',str(Path(__file__)),'command',str(folder)],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,env=env,timeout=max(0,min(5,deadline-time.monotonic()))+10)
                own=json.loads((folder/'execution.json').read_bytes());c.need(child.returncode==own['exit']==0 and own['retired'] and not own['errors'],'complete first pure semantic case')
                guard();core.account(folder,test,names)
        registry.registry(deadline);core.guard();c.need(source()==before and time.monotonic()<deadline,'fresh full source and original semantic deadline')
        result['complete']=True
    except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
    c.write(out,'session-result.json',result);raise SystemExit(0 if result['complete'] else 1)

def outer(compiler):
    c.need(compiler in ('194','188') and p.EXECUTION_ALLOWED and not (p.BASE/'STOP.json').exists(),'fixed live G semantics')
    out=p.BASE/('native-semantic'+compiler);out.mkdir(exist_ok=False);before=source();head=c.snapshot()
    argv=[str(Path(os.environ['SystemRoot'])/'System32/wsl.exe'),'-d','Ubuntu','-u','artur','--cd','/','--exec','/usr/bin/python3','-B',
      '/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/chitchat-frame-evidence-074/semantics.py','session',compiler]
    c.write(out,'intent.json',dict(argv=argv,source=before,git=head,bound=1200,session_bound=1100,floor=4*1024**3,poll=5,cleanup=5,retry=False))
    result=dict(complete=False,root_adopted=False,resources_qualified=False)
    with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
        try:
            child=subprocess.run([sys.executable,'-B',str(Path(__file__)),'outer-worker',compiler],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,timeout=1210)
            own=json.loads((out/'execution.json').read_bytes());c.need(child.returncode==own['exit']==0 and own['retired'] and not own['errors'],'complete original outer Job')
            c.need(source()==before and c.snapshot()==head and json.loads((out/'session-result.json').read_bytes())['complete'],'full source/ref semantic postflight');result['complete']=True
        except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
    c.write(out,'result.json',result);print(json.dumps(dict(compiler=compiler,**result)));raise SystemExit(0 if result['complete'] else 1)

if __name__=='__main__':
    c.need(p.EXECUTION_ALLOWED and not (p.BASE/'STOP.json').exists(),'G stop seal')
    action=sys.argv[1]
    if action=='run':outer(sys.argv[2])
    elif action=='session':session(sys.argv[2])
    elif action=='outer-worker':
        compiler=sys.argv[2];out=p.BASE/('native-semantic'+compiler);intent=json.loads((out/'intent.json').read_bytes())
        _,env,_=c.environment('194');os.environ.clear();os.environ.update(env)
        own=c.owned._owned(c.owned.load_job().WindowsJob(),intent['argv'],str(ROOT),core.capacity,bound=1200)
        c.write(out,'execution.json',own);raise SystemExit(own['exit'])
    elif action=='command':
        folder=Path(sys.argv[2]);relative=folder.resolve().relative_to(p.BASE.resolve());c.need(relative.parts[0] in ('native-semantic194','native-semantic188'),'fixed semantic output')
        intent=json.loads((folder/'intent.json').read_bytes());os.environ.clear();os.environ.update(intent['env'])
        group=c.load(ROOT/'scripts/ci/watchdog_linux_group.py','frame_semantic_group').LinuxGroup()
        own=c.owned._owned(group,intent['argv'],intent['cwd'],core.capacity,bound=5,deadline=min(intent['deadline'],time.monotonic()+5))
        c.write(folder,'execution.json',own);raise SystemExit(own['exit'])
    else:raise ValueError('fixed semantic actions')
