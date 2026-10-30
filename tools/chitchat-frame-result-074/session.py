"""Fixed native phases. Every original invoke guard and bound is retained."""
import json,os,shutil,time,traceback
from pathlib import Path

def session(core):
    c=core.c;p=core.p;out=core.OUT;state=core.STATE;native=core.NATIVE
    import run as windows
    deadline=time.monotonic()+1100
    result=dict(complete=False,root_adopted=False,resources_qualified=False,phase=core.PHASE)
    try:
        c.need(os.getuid()==os.getgid()==1000,'native nonroot owner');core.guard()
        registry=c.load(c.HERE/'runner.py','frame_registry');registry.registry(deadline)
        _,env,_=c.environment('194')
        def invoke(name,argv,bound=1200):return core.invoke(name,argv,env,deadline,bound)
        def account(name,key,test):
            image=json.loads((state/('image-'+key+'.json')).read_bytes())
            c.need(c.sha(Path(image['path']).read_bytes())==image['sha256'],'held exact image')
            names=json.loads((state/('catalog-'+key+'.json')).read_bytes())
            core.account(invoke(name,[image['path'],'--exact',test,'--test-threads=1','--nocapture'],5),test,names)
        if core.PHASE=='build':
            c.need(not native.exists() and not state.exists(),'fresh native source/state')
            native.mkdir(parents=True,mode=0o700);state.mkdir()
            shutil.copytree(core.INPUT/'fixtures',native/'fixtures')
            for name in ('wrap.c','wrap.h','control.c','peek_control.c'):shutil.copyfile(core.INPUT/name,native/name)
            invoke('source-control',['/usr/bin/python3','-B',str(c.ROOT/'tools/chitchat-call-context-074/parallel_source.py'),str(core.INPUT/'preparation.json')])
            invoke('gcc-version',['/usr/bin/gcc','--version']);invoke('ld-version',['/usr/bin/ld.bfd','--version'])
            flags=['/usr/bin/gcc','-std=c11','-O2','-g','-Wall','-Wextra','-Werror','-fno-builtin','-fno-lto']
            invoke('compile-wrapper',[*flags,'-c',str(native/'wrap.c'),'-o',str(native/'wrap.o')])
            c.write(state,'object.json',dict(path=str(native/'wrap.o'),sha256=c.sha((native/'wrap.o').read_bytes())))
            for label in ('control','peek_control'):
                invoke('compile-'+label,[*flags,str(native/(label+'.c')),str(native/'wrap.o'),core.WRAP,'-pthread','-o',str(native/label)])
                folder=invoke(label,[str(native/label)],5)
                marker='HC074_NATIVE_C_CONTROLS:' if label=='control' else 'HC074_PEEK_CONTROL:'
                raw=(folder/'stdout.log').read_text();c.need(raw.count(marker)==1 and json.loads(raw.split(marker)[1].splitlines()[0])['complete'],'original C controls')
            c.need('13.3.0' in (out/'gcc-version/stdout.log').read_text() and '2.42' in (out/'ld-version/stdout.log').read_text(),'qualified GNU compiler/linker')
            for v,compiler in [('B','194'),('P','194'),('G','194'),('G','188')]:
                key=v+compiler;tool,env=core.environment(compiler)
                version=invoke('compiler-'+key,[str(tool/'rustc'),'-vV'])
                c.need('release: '+('1.94.0' if compiler=='194' else '1.88.0')+'\n' in (version/'stdout.log').read_text(),'actual compiler')
                args=core.cargo_args(v,compiler)
                built=invoke('build-'+key,[str(tool/'cargo'),'test',*args,'--lib','--no-run','--message-format=json'])
                rows=[json.loads(s) for s in (built/'stdout.log').read_text().splitlines() if s.startswith('{')]
                actual=[r for r in rows if r.get('reason')=='compiler-artifact' and r.get('executable') and r['target']['kind']==['lib'] and r['profile']['test']]
                c.need(len(actual)==1 and not actual[0]['fresh'] and [r['success'] for r in rows if r.get('reason')=='build-finished']==[True],'fresh independent image')
                c.need(Path(actual[0]['target']['src_path']).resolve()==native/'fixtures'/v/'src/lib.rs','actual source identity')
                image=Path(actual[0]['executable']);c.need(image.resolve().is_relative_to(native/'builds'/key),'independent target')
                record=dict(path=str(image),sha256=c.sha(image.read_bytes()))
                for previous in state.glob('image-*.json'):c.need(record['sha256']!=json.loads(previous.read_bytes())['sha256'],'no binary substitution')
                c.write(state,'image-'+key+'.json',record)
                listed=invoke('catalog-'+key,[str(image),'--list'],10)
                raw=(listed/'stdout.log').read_text();names=sorted(s[:-6] for s in raw.splitlines() if s.endswith(': test'))
                old=json.loads((windows.phase('catalog-'+('B' if v=='P' else v)+('-188' if compiler=='188' else ''))/'result.json').read_bytes())['catalog']
                extra=['delta::cost_review::'+n for n in ('allocator_control','allocation_observations','operation_allocation_observations','native_c_allocator_control','native_rust_tag_control','native_operation_observations','c_checkpoint_positive_and_retained_negative_control')]
                extra+=['delta::cost_review::actual_caller::'+n for n in ('fixed_caller_semantics_control','fixed_caller_capacity_control','fixed_caller_observations')]
                extra+=['serialize::stateful_review::'+n for n in ('stateful_semantics_control','stateful_capacity_control','stateful_observations')]
                c.need(names==sorted(old+extra) and len(set(names))==len(names) and raw.splitlines()[-1]==f'{len(names)} tests, 0 benchmarks','whole catalog')
                c.write(state,'catalog-'+key+'.json',names)
                for action in ('check','clippy'):
                    argv=[str(tool/'cargo'),action,*args,'--all-targets']
                    if action=='clippy':argv+=['--','-D','warnings']
                    invoke(action+'-'+key,argv)
                archives=list((native/'builds'/key).glob('x86_64-unknown-linux-gnu/release/build/zstd-sys-*/out/libzstd.a'));c.need(len(archives)==1,'one linked Zstd')
                invoke('zstd-imports-'+key,['/usr/bin/nm','-u',str(archives[0])]);invoke('linked-zstd-'+key,['/usr/bin/objdump','-d','--disassemble=ZSTD_createCCtx',str(image)])
        else:
            c.need(json.loads((p.BASE/'native-build/result.json').read_bytes())['complete'],'complete original build phase')
            _,env=core.environment('194')
            if core.PHASE in ('semantic194','semantic188'):
                keys=('B194','P194','G194') if core.PHASE=='semantic194' else ('G188',)
                for key in keys:
                    names=json.loads((state/('catalog-'+key+'.json')).read_bytes())
                    selected=['delta::cost_review::'+n for n in ('allocator_control','native_c_allocator_control','native_rust_tag_control','c_checkpoint_positive_and_retained_negative_control')]
                    selected+=['serialize::stateful_review::'+n for n in ('stateful_semantics_control','stateful_capacity_control')]
                    selected+=['delta::cost_review::actual_caller::'+n for n in ('fixed_caller_semantics_control','fixed_caller_capacity_control')]
                    if key.startswith('G'):
                        selected += [n for n in names if n.startswith(windows.PURE+('serialize::call_context_review::','serialize::fused_review::','delta::fused_review::','serialize::frame_review::')) or n in windows.OBS]
                        selected+=['serialize::security_stream_review::minimal_bound_contract','serialize::security_stream_review::all_stream_bounds_hold','serialize::retained_finish::retained_finish_output']
                    c.need(len(selected)==len(set(selected)),'complete unique selection')
                    c.write(out,'selection-'+key+'.json',dict(selected=selected,unexecuted=[n for n in names if n not in selected],full_library_passed=False))
                    for i,test in enumerate(selected):account('case-'+key+f'-{i:03}',key,test)
            else:
                for phase in ('semantic194','semantic188'):c.need(json.loads((p.BASE/('native-'+phase)/'result.json').read_bytes())['complete'],'controls before observations')
                for i,v in enumerate(('B','P','G','G','P','B'),1):
                    for kind,test in [('operation','delta::cost_review::native_operation_observations'),('stateful','serialize::stateful_review::stateful_observations'),('caller','delta::cost_review::actual_caller::fixed_caller_observations')]:account(f'observe-{i}-{v}-{kind}',v+'194',test)
        registry.registry(deadline);core.guard()
        for path in state.glob('image-*.json'):
            image=json.loads(path.read_bytes());c.need(c.sha(Path(image['path']).read_bytes())==image['sha256'],'image postflight')
        c.need(time.monotonic()<deadline,'original native phase deadline');result['complete']=True
    except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
    c.write(out,'session-result.json',result);raise SystemExit(0 if result['complete'] else 1)
