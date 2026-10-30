"""Exclusive nominal controls using real packages and unchanged original owners."""
import json,os,shutil,subprocess,sys,time,traceback
from pathlib import Path
import contract as s
c=s.c;m=s.m;OUT=None
PHASES=('stage','resolve','build194','build188','negative194','negative188')

def mirror(deadline):
    ledger=json.loads((s.BASE/'input/registry.json').read_bytes())
    for name,digest in ledger.items():
        c.need(time.monotonic()<deadline,'original native deadline during mirror validation');s.capacity()
        folder=s.N/'vendor'/name;values=c.tree(folder);check=json.loads((folder/'.cargo-checksum.json').read_bytes());values.pop('.cargo-checksum.json')
        c.need(values==check['files'] and check['package']==digest,'whole actual immutable official mirror: '+name)

def invoke(name,argv,env,deadline,bound=780,cwd=None,mutate=False,expect=0):
    before=s.source();head=c.snapshot();s.capacity()
    if not mutate:s.actual()
    folder=OUT/name;folder.mkdir();image=c.sha(Path(argv[0]).read_bytes());cwd=str(cwd or s.N)
    c.write(folder,'intent.json',dict(argv=argv,env=env,cwd=cwd,deadline=deadline,bound=bound,image_sha256=image,retry=False))
    with (folder/'stdout.log').open('xb') as stdout,(folder/'stderr.log').open('xb') as stderr:
        child=subprocess.run([sys.executable,'-B',str(Path(__file__)),'command',str(folder)],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,env=env,timeout=max(0,min(bound,deadline-time.monotonic()))+10)
    own=json.loads((folder/'execution.json').read_bytes());c.need(child.returncode==own['exit']==expect and own['retired'] and not own['errors'],'complete first nominal command: '+name)
    c.need(s.source()==before and c.snapshot()==head and c.sha(Path(argv[0]).read_bytes())==image,'whole nominal source/ref/image postflight')
    if not mutate:s.actual()
    return folder

def args(compiler,variant):
    return ['--manifest-path',str(s.N/'source'/variant/'Cargo.toml'),'--target-dir',str(s.N/'builds'/compiler),'--locked','--offline','--release','--all-features']

def rows(folder):return [json.loads(line) for line in (folder/'stdout.log').read_bytes().decode().splitlines() if line.startswith('{')]

def stage(deadline):
    c.need(not s.N.exists(),'exclusive new native identity context');s.N.mkdir(parents=True,mode=0o700)
    s.prior.registry(deadline);(s.N/'vendor').mkdir()
    ledger=json.loads((s.BASE/'input/registry.json').read_bytes())
    for name in ledger:
        if name=='chitchat-0.11.1':
            files=m.codec().unpack((s.BASE/'input/legacy.crate').read_bytes(),name);folder=s.N/'vendor'/name
            for n,b in files.items():
                path=folder/n;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(b)
            c.write(folder,'.cargo-checksum.json',dict(package=m.UPSTREAM_SHA,files={n:c.sha(b) for n,b in files.items()}))
        else:shutil.copytree(s.prior.VENDOR/name,s.N/'vendor'/name)
    shutil.copytree(s.BASE/'source',s.N/'source');(s.N/'cargo-home').mkdir()
    reference=json.loads((s.BASE/'input/registry-reference.json').read_bytes())
    config='[source.crates-io]\nreplace-with="identity-reviewed"\n[source.identity-reviewed]\ndirectory="'+(s.N/'vendor').as_posix()+'"\n[registries.hc074-draft]\nindex="'+reference['registry'].removeprefix('registry+')+'"\n'
    (s.N/'cargo-home/config.toml').write_text(config,encoding='utf8');c.write(s.BASE/'state','config.json',dict(sha256=c.sha(config.encode())))
    for variant in s.VARIANTS:
        c.need(c.tree(s.N/'source'/variant)==c.tree(s.BASE/'source'/variant),'whole exact staged nominal fixture')
        c.write(s.BASE/'state','stage-'+variant+'.json',c.tree(s.N/'source'/variant))
    mirror(deadline);s.actual()

def resolve(tool,env,deadline):
    nodes={};downloads={}
    for variant in s.VARIANTS:
        folder=s.N/'source'/variant;before=c.tree(folder)
        out=invoke('metadata-'+variant,[str(tool/'cargo'),'metadata','--manifest-path',str(folder/'Cargo.toml'),'--all-features','--format-version','1'],dict(env,CARGO_NET_OFFLINE='false',CARGO_NET_RETRY='0'),deadline,cwd=folder,mutate=True)
        after=c.tree(folder);c.need('Cargo.lock' not in before and after.pop('Cargo.lock') and before==after,'only first local lock added')
        metadata=json.loads((out/'stdout.log').read_bytes());nodes[variant]=s.graph(metadata,variant)
        c.write(s.BASE/'state','resolved-'+variant+'.json',c.tree(folder))
        (s.BASE/'state'/('lock-'+variant+'.toml')).write_bytes((folder/'Cargo.lock').read_bytes())
        for package in metadata['packages']:
            if package['name'] not in ('chitchat',m.NAME):continue
            name=package['name'];root=Path(package['manifest_path']).parent
            files=m.codec().unpack((s.BASE/'input'/('owned.crate' if name==m.NAME else 'legacy.crate')).read_bytes(),m.PREFIX if name==m.NAME else 'chitchat-0.11.1')
            actual=c.tree(root);actual.pop('.cargo-ok',None);actual.pop('.cargo-checksum.json',None)
            c.need(actual=={n:c.sha(b) for n,b in files.items()},'whole actually downloaded real package, no overlay')
            row=dict(path=str(root),files=actual,source=package['source'],version=package['version'])
            c.need(name not in downloads or downloads[name]==row,'all probes resolve same actual real package');downloads[name]=row
        s.actual()
    c.need(all(value==nodes['positive'] for value in nodes.values()),'identical whole registry graph for all four probes')
    c.write(s.BASE/'state','downloads.json',downloads);c.write(OUT,'graph-review.json',dict(complete=True,registry_nodes=len(nodes['positive']),real_packages=downloads,root_adopted=False))

def build(compiler,tool,env,deadline):
    folder=s.N/'source/positive';base=args(compiler,'positive')
    built=invoke('build',[str(tool/'cargo'),'test',*base,'--lib','--no-run','--message-format=json'],env,deadline,cwd=folder)
    records=rows(built);images=[r for r in records if r.get('reason')=='compiler-artifact' and r.get('executable') and r['target']['kind']==['lib'] and r['profile']['test']]
    c.need(len(images)==1 and not images[0]['fresh'] and images[0]['target']['src_path']==str(folder/'src/lib.rs'),'one fresh actual positive probe test image')
    for name in ('chitchat',m.NAME):
        library=[r for r in records if r.get('reason')=='compiler-artifact' and r['target']['name']=='chitchat' and r['target']['kind']==['lib'] and ('#'+name+'@' in r['package_id'] or '#'+name+'/' in r['package_id'])]
        c.need(len(library)==1 and not library[0]['fresh'] and not library[0]['profile']['test'],'fresh normal dependency, not test overlay: '+name)
    image=Path(images[0]['executable']);c.need(image.resolve().is_relative_to(s.N/'builds'/compiler),'owned exact positive image')
    receipt=dict(path=str(image),sha256=c.sha(image.read_bytes()));c.write(OUT,'image.json',receipt)
    listing=invoke('catalog',[str(image),'--list'],env,deadline,10,cwd=folder)
    text=(listing/'stdout.log').read_bytes().decode();names=sorted(line[:-6] for line in text.splitlines() if line.endswith(': test'))
    expected=sorted('tests::'+name for name in ('identical_display_names_do_not_establish_nominal_identity','small_wire_surfaces_agree_across_the_two_real_packages','owned_nominal_surface_remains_usable'))
    c.need(names==expected and text.splitlines()[-1]=='3 tests, 0 benchmarks','whole exact nominal catalog');c.write(OUT,'catalog.json',names)
    for action in ('check','clippy'):
        argv=[str(tool/'cargo'),action,*base,'--all-targets']
        if action=='clippy':argv+=['--','-D','warnings']
        invoke(action,argv,env,deadline,cwd=folder)
    for i,name in enumerate(names):
        c.need(c.sha(image.read_bytes())==receipt['sha256'],'same admitted image before execution')
        out=invoke(f'case-{i:02}',[str(image),'--exact',name,'--test-threads=1','--nocapture'],env,deadline,5,cwd=folder)
        c.need('1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out;' in (out/'stdout.log').read_bytes().decode(),'whole unignored positive test outcome')
    c.need(c.sha(image.read_bytes())==receipt['sha256'],'same original positive image after execution');c.write(OUT,'positive-review.json',dict(complete=True,passed=3,root_adopted=False,adapter_compiled=False))

def session(phase):
    global OUT
    OUT=s.BASE/'native'/phase;deadline=time.monotonic()+1100;result=dict(complete=False,root_adopted=False,adapter_compiled=False,release_admitted=False)
    try:
        c.need(os.getuid()==os.getgid()==1000,'original local nonroot GNU owner');s.capacity();before=s.source();head=c.snapshot();boot=Path('/proc/sys/kernel/random/boot_id').read_bytes()
        compiler='188' if phase.endswith('188') else '194';tool,env,_=s.environment(compiler)
        required=[] if phase=='stage' else ['stage'] if phase=='resolve' else ['resolve'] if phase.startswith('build') else ['build'+compiler]
        for name in required:c.need(json.loads((s.BASE/'native'/name/'result.json').read_bytes())['complete'],'complete nominal prerequisite')
        if phase=='stage':stage(deadline)
        else:
            s.actual();mirror(deadline)
            if phase=='resolve':resolve(tool,env,deadline)
            else:
                v=invoke('rustc-version',[str(tool/'rustc'),'-vV'],env,deadline,10)
                c.need(('release: '+('1.88.0' if compiler=='188' else '1.94.0')+'\n').encode() in (v/'stdout.log').read_bytes(),'actual pinned compiler')
                if phase.startswith('build'):build(compiler,tool,env,deadline)
                else:
                    for variant in ('config','id','transport'):
                        folder=s.N/'source'/variant
                        out=invoke(variant,[str(tool/'cargo'),'check',*args(compiler,variant),'--lib','--message-format=json'],env,deadline,cwd=folder,expect=101)
                        c.write(out,'negative-review.json',s.negative(rows(out),variant,folder/'src/lib.rs'))
            mirror(deadline);s.actual()
        c.need(s.source()==before and c.snapshot()==head and Path('/proc/sys/kernel/random/boot_id').read_bytes()==boot and time.monotonic()<deadline,'whole original source/ref/boot/deadline postflight');result['complete']=True
    except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
    c.write(OUT,'session-result.json',result);raise SystemExit(0 if result['complete'] else 1)

def outer(phase):
    c.need(phase in PHASES and not (s.BASE/'STOP.json').exists(),'fixed phase, no retry after first unexpected refusal')
    out=s.BASE/'native'/phase;out.mkdir(parents=True,exist_ok=False);before=s.source();head=c.snapshot()
    argv=[str(Path(os.environ['SystemRoot'])/'System32/wsl.exe'),'-d','Ubuntu','-u','artur','--cd','/','--exec','/usr/bin/python3','-B','/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/chitchat-public-identity-074/runner.py','session',phase]
    c.write(out,'intent.json',dict(argv=argv,source=before,git=head,bound=1200,session_bound=1100,floor=4*1024**3,poll=5,cleanup=5,retry=False))
    result=dict(complete=False,root_adopted=False,adapter_compiled=False,release_admitted=False)
    with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
        try:
            child=subprocess.run([sys.executable,'-B',str(Path(__file__)),'outer-worker',phase],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,timeout=1210)
            own=json.loads((out/'execution.json').read_bytes());c.need(child.returncode==own['exit']==0 and own['retired'] and not own['errors'],'complete original nominal Job')
            c.need(s.source()==before and c.snapshot()==head and json.loads((out/'session-result.json').read_bytes())['complete'],'whole source/ref/session postflight');result['complete']=True
        except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
    c.write(out,'result.json',result)
    if not result['complete']:
        try:c.write(s.BASE,'STOP.json',dict(phase=phase,first_refusal=result['first_refusal'],root_adopted=False))
        except FileExistsError:pass
    print(json.dumps(dict(phase=phase,**result)));raise SystemExit(0 if result['complete'] else 1)

if __name__=='__main__':
    c.need(not (s.BASE/'STOP.json').exists(),'first unexpected refusal stops this context');action=sys.argv[1]
    if action=='prepare':s.prepare()
    elif action=='run':outer(sys.argv[2])
    elif action=='session':session(sys.argv[2])
    elif action=='outer-worker':
        phase=sys.argv[2];out=s.BASE/'native'/phase;intent=json.loads((out/'intent.json').read_bytes());_,env,_=s.environment();os.environ.clear();os.environ.update(env)
        own=c.owned._owned(c.owned.load_job().WindowsJob(),intent['argv'],str(s.ROOT),s.capacity,bound=1200);c.write(out,'execution.json',own);raise SystemExit(own['exit'])
    elif action=='command':
        folder=Path(sys.argv[2]);c.need(folder.resolve().is_relative_to((s.BASE/'native').resolve()),'contained original command output');intent=json.loads((folder/'intent.json').read_bytes());os.environ.clear();os.environ.update(intent['env'])
        group=c.load(s.ROOT/'scripts/ci/watchdog_linux_group.py','identity_original_group').LinuxGroup()
        own=c.owned._owned(group,intent['argv'],intent['cwd'],s.capacity,bound=intent['bound'],deadline=min(intent['deadline'],time.monotonic()+intent['bound']));c.write(folder,'execution.json',own);raise SystemExit(own['exit'])
    else:raise ValueError('fixed nominal command')
