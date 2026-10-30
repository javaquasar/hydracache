"""Fixed package/consumer phases; first outcomes never overwritten or retried."""
import json,os,re,shutil,subprocess,sys,time,traceback
from pathlib import Path
print('HC074_PACKAGE: interpreter entered '+sys.argv[1],file=sys.stderr,flush=True)
import common as s
print('HC074_PACKAGE: common imports ready',file=sys.stderr,flush=True)
m=s.m;c=s.c;N=s.NATIVE
PHASES={'stage','resolve','package','build194','build188','pure194','pure188','rest194','rest188','registry','consumer194','consumer188'}
PURE=('serialize::tests::','delta::tests::','message::tests::','digest::tests::','state::tests::')
OUT=None

def invoke(name,argv,env,deadline,bound=1200,cwd=None,allow_mutation=False,expect=0):
    before=s.source();head=c.snapshot();s.capacity()
    if not allow_mutation:s.actual()
    folder=OUT/name;folder.mkdir()
    c.write(folder,'intent.json',dict(argv=argv,env=env,cwd=str(cwd or N),deadline=deadline,bound=bound,image_sha256=c.sha(Path(argv[0]).read_bytes()),retry=False))
    with (folder/'stdout.log').open('xb') as stdout,(folder/'stderr.log').open('xb') as stderr:
        child=subprocess.run([sys.executable,'-B',str(Path(__file__)),'command',str(folder)],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,env=env,timeout=max(0,min(bound,deadline-time.monotonic()))+10)
    c.need((folder/'execution.json').is_file(),'dispatch stopped by a prior refusal before command ownership')
    own=json.loads((folder/'execution.json').read_bytes());c.need(child.returncode==own['exit']==expect and own['retired'] and not own['errors'],'first complete owned command: '+name)
    c.need(s.source()==before and c.snapshot()==head and c.sha(Path(argv[0]).read_bytes())==json.loads((folder/'intent.json').read_bytes())['image_sha256'],'whole original source/ref/image postflight')
    if not allow_mutation:s.actual()
    return folder

def args(manifest,target):return ['--manifest-path',str(manifest),'--target-dir',str(target),'--offline','--locked','--release','--all-features']

def artifact(folder,manifest,package,test=True):
    rows=[json.loads(v) for v in (folder/'stdout.log').read_text().splitlines() if v.startswith('{')]
    found=[r for r in rows if r.get('reason')=='compiler-artifact' and r.get('executable') and r['profile']['test'] is test and r['target']['kind']==['lib']]
    c.need([r['success'] for r in rows if r.get('reason')=='build-finished']==[True] and len(found)==1 and not found[0]['fresh'],'one fresh actual library test artifact')
    row=found[0];c.need(Path(row['target']['src_path']).resolve()==manifest.parent/'src/lib.rs','actual source identity')
    p=Path(row['executable']);c.need(p.resolve().is_relative_to(N/'builds'), 'owned independent target')
    return dict(path=str(p),sha256=c.sha(p.read_bytes()),source=str(manifest),package=package)

def listing(folder):
    raw=(folder/'stdout.log').read_text();names=sorted(v[:-6] for v in raw.splitlines() if v.endswith(': test'))
    c.need(len(names)==len(set(names)) and raw.splitlines()[-1]==f'{len(names)} tests, 0 benchmarks','whole actual catalog');return names

def session(phase):
    global OUT
    OUT=m.BASE/'native'/phase;deadline=time.monotonic()+1100;result=dict(complete=False,root_adopted=False,registry_qualified=False,release_admitted=False)
    try:
        c.need(os.getuid()==os.getgid()==1000,'original nonroot local GNU owner');s.capacity();before=s.source();boot=Path('/proc/sys/kernel/random/boot_id').read_bytes()
        tool,env,_=s.environment('188' if phase.endswith('188') else '194');state=m.BASE/'state';state.mkdir(exist_ok=True)
        prerequisites={'resolve':['stage'],'package':['resolve'],'registry':['pure194','rest194','pure188','rest188']}
        prior=prerequisites.get(phase,['package'] if phase.startswith('build') else ['build'+phase[-3:]] if phase.startswith(('pure','rest')) else ['registry'] if phase.startswith('consumer') else [])
        for name in prior:c.need(json.loads((m.BASE/'native'/name/'result.json').read_bytes())['complete'],'complete prior phase: '+name)
        if phase=='stage':
            c.need(not N.exists(),'exclusive new native source root');N.mkdir(parents=True,mode=0o700)
            shutil.copytree(m.BASE/'source',N/'source');(N/'cargo-home').mkdir()
            config='[source.crates-io]\nreplace-with="reviewed-upstream"\n[source.reviewed-upstream]\ndirectory="'+s.VENDOR.as_posix()+'"\n'
            (N/'cargo-home/config.toml').write_text(config,encoding='utf8')
            c.write(state,'cargo-config.toml.json',dict(sha256=c.sha(config.encode())))
            c.write(state,'source.json',c.tree(N/'source'))
        else:
            s.actual();s.registry(deadline)
            if phase=='package' or phase.startswith(('build','consumer')):
                version=invoke('rustc-version',[str(tool/'rustc'),'-vV'],env,deadline,10)
                expected='1.88.0' if phase.endswith('188') else '1.94.0'
                c.need('release: '+expected+'\n' in (version/'stdout.log').read_text(),'actual pinned compiler version')
            if phase=='resolve':
                old=c.tree(N/'source')
                folder=invoke('metadata',[str(tool/'cargo'),'metadata','--manifest-path',str(N/'source/Cargo.toml'),'--offline','--all-features','--format-version','1'],env,deadline,allow_mutation=True)
                new=c.tree(N/'source');old.pop('Cargo.lock');new.pop('Cargo.lock');c.need(old==new,'only owned active lock normalization')
                c.need(m.registry_nodes((N/'source/Cargo.lock').read_bytes())==m.registry_nodes((m.BASE/'source/Cargo.lock').read_bytes()),'no registry node/checksum/edge drift')
                (state/'source.json').unlink();c.write(state,'source.json',c.tree(N/'source'))
                (state/'resolved.lock').write_bytes((N/'source/Cargo.lock').read_bytes())
            elif phase=='package':
                argv=[str(tool/'cargo'),'package',*[v for v in args(N/'source/Cargo.toml',N/'builds/package194') if v!='--release'],'--allow-dirty']
                invoke('cargo-package',argv,env,deadline,780)
                raw=(N/'builds/package194/package'/(m.PREFIX+'.crate')).read_bytes()
                source={p.relative_to(N/'source').as_posix():p.read_bytes() for p in (N/'source').rglob('*') if p.is_file()}
                _,u,_=m.parent();files=m.verify_crate(raw,u,source)
                out=m.BASE/'artifact';out.mkdir();(out/(m.PREFIX+'.crate')).write_bytes(raw)
                c.write(out,'receipt.json',dict(sha256=c.sha(raw),bytes=len(raw),members={n:c.sha(b) for n,b in files.items()},root_adopted=False,registry_qualified=False))
                for name,body in files.items():
                    p=N/'extracted'/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(body)
                c.write(state,'extracted.json',c.tree(N/'extracted'))
            elif phase.startswith('build'):
                compiler=phase[-3:];manifest=N/'extracted/Cargo.toml';target=N/'builds'/('extracted'+compiler)
                built=invoke('build',[str(tool/'cargo'),'test',*args(manifest,target),'--lib','--no-run','--message-format=json'],env,deadline)
                image=artifact(built,manifest,m.NAME);c.write(state,'image-'+compiler+'.json',image)
                names=listing(invoke('catalog',[image['path'],'--list'],env,deadline,10));c.write(state,'catalog-'+compiler+'.json',names)
                for action in ('check','clippy'):
                    argv=[str(tool/'cargo'),action,*args(manifest,target),'--all-targets']
                    if action=='clippy':argv+=['--','-D','warnings']
                    invoke(action,argv,env,deadline)
            elif phase.startswith(('pure','rest')):
                compiler=phase[-3:];image=json.loads((state/('image-'+compiler+'.json')).read_bytes());names=json.loads((state/('catalog-'+compiler+'.json')).read_bytes())
                selected=[n for n in names if n.startswith(PURE)==phase.startswith('pure')]
                c.need(selected,'nonempty fixed catalog partition');c.write(OUT,'selection.json',dict(selected=selected,unexecuted=[n for n in names if n not in selected],whole_catalog=len(names)))
                # Existing C2 pure envelope: full Root entry/exit, whole actual
                # source/image before/after each separately owned five-second case.
                for i,test in enumerate(selected):
                    s.actual();c.need(c.sha(Path(image['path']).read_bytes())==image['sha256'],'held exact actual test image')
                    folder=OUT/f'case-{i:03}';folder.mkdir()
                    argv=[image['path'],'--exact',test,'--test-threads=1','--nocapture']
                    c.write(folder,'intent.json',dict(argv=argv,env=env,cwd=str(N/'extracted'),bound=5,deadline=deadline,image_sha256=image['sha256'],retry=False))
                    with (folder/'stdout.log').open('xb') as stdout,(folder/'stderr.log').open('xb') as stderr:
                        child=subprocess.run([sys.executable,'-B',str(Path(__file__)),'command',str(folder)],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,env=env,timeout=max(0,min(5,deadline-time.monotonic()))+10)
                    own=json.loads((folder/'execution.json').read_bytes());raw=(folder/'stdout.log').read_text()
                    c.need(child.returncode==own['exit']==0 and own['retired'] and not own['errors'],'first complete selected case: '+test)
                    c.need(re.findall(r'^test (.+?) (?:- should panic )?\.\.\. ',raw,re.M)==[test] and f'1 passed; 0 failed; 0 ignored; 0 measured; {len(names)-1} filtered out;' in raw,'complete unignored exact accounting')
                    s.actual();c.need(c.sha(Path(image['path']).read_bytes())==image['sha256'],'held exact image postflight')
                result['passed']=len(selected)
            elif phase=='registry':
                # Implementation follows the documented file-backed registry, with
                # no publish API and no patch/source substitution for the fork.
                registry_phase(env,tool,deadline)
            elif phase.startswith('consumer'):
                consumer_phase(phase[-3:],env,tool,deadline)
            s.registry(deadline);s.actual()
        c.need(s.source()==before and Path('/proc/sys/kernel/random/boot_id').read_bytes()==boot and time.monotonic()<deadline,'whole source/boot and original native deadline')
        result['complete']=True
    except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
    c.write(OUT,'session-result.json',result);raise SystemExit(0 if result['complete'] else 1)

def registry_phase(env,tool,deadline):
    raw=(m.BASE/'artifact'/(m.PREFIX+'.crate')).read_bytes();receipt=json.loads((m.BASE/'artifact/receipt.json').read_bytes())
    c.need(c.sha(raw)==receipt['sha256'],'exact packaged artifact before registry preparation')
    source={p.relative_to(N/'source').as_posix():p.read_bytes() for p in (N/'source').rglob('*') if p.is_file()}
    config=(N/'cargo-home/config.toml').read_bytes()
    def index(label,corrupt=False):
        folder=N/label;folder.mkdir();dl=N/(label+'-download');dl.mkdir()
        (dl/(m.PREFIX+'.crate')).write_bytes(raw+(b'\n' if corrupt else b''))
        (folder/'config.json').write_bytes(m.encoded(dict(dl=dl.as_uri()+'/{crate}-{version}.crate')))
        entry=folder/'hy/dr'/m.NAME;entry.parent.mkdir(parents=True)
        entry.write_bytes(json.dumps(m.registry_row(source,receipt['sha256']),sort_keys=True).encode()+b'\n')
        flags=['/usr/bin/git','-c','core.hooksPath=/dev/null','-c','commit.gpgsign=false','-c','user.name=HydraCache Diagnostic','-c','user.email=diagnostic@invalid','-C',str(folder)]
        invoke(label+'-init',[*flags,'init','--template=','--initial-branch=main'],env,deadline)
        invoke(label+'-add',[*flags,'add','--','config.json','hy/dr/'+m.NAME],env,deadline)
        invoke(label+'-commit',[*flags,'commit','--quiet','-m','isolated unpublished package checksum fixture'],env,deadline)
        return folder
    positive=index('registry-index');negative=index('bad-registry-index',True)
    stanza=b'\n[registries.hc074-draft]\nindex="'+positive.as_uri().encode()+b'"\n'
    (N/'cargo-home/config.toml').write_bytes(config+stanza)
    frame=m.BASE/'state/cargo-config.toml.json';frame.unlink();c.write(frame.parent,frame.name,dict(sha256=c.sha(config+stanza)))
    consumer=N/'consumer';(consumer/'src').mkdir(parents=True)
    (consumer/'Cargo.toml').write_text('''[package]
name="hc074-owned-consumer"
version="0.0.0"
edition="2024"
rust-version="1.88"
publish=false
[dependencies]
chitchat={package="hydra-chitchat",version="=0.11.1-hydra.1",registry="hc074-draft"}
tokio={version="=1.53.3",features=["rt","macros","sync","time"]}
[features]
testsuite=["chitchat/testsuite"]
[workspace]
resolver="2"
''',encoding='utf8')
    (consumer/'src/lib.rs').write_text('pub fn original_library_type()->&\'static str {std::any::type_name::<chitchat::Chitchat>()}\n#[cfg(test)]\n#[path="consumer_cases.rs"]\nmod consumer;\n',encoding='utf8')
    (consumer/'src/consumer_cases.rs').write_bytes((m.BASE/'input/consumer.rs').read_bytes())
    # Seed only a file:// registry, with ordinary crates.io source bytes supplied
    # by the reviewed immutable mirror. All later builds are offline/locked.
    seed=dict(env,CARGO_NET_OFFLINE='false',CARGO_NET_RETRY='0')
    metadata=invoke('registry-seed',[str(tool/'cargo'),'metadata','--manifest-path',str(consumer/'Cargo.toml'),'--all-features','--format-version','1'],seed,deadline)
    data=json.loads((metadata/'stdout.log').read_bytes());owned=[p for p in data['packages'] if p['name']==m.NAME]
    c.need(len(owned)==1 and owned[0]['version']==m.VERSION and owned[0]['source']=='registry+'+positive.as_uri(),'actual independent registry source, no path fallback')
    extracted=Path(owned[0]['manifest_path']).parent;files=m.codec().unpack(raw,m.PREFIX)
    for name,body in files.items():c.need((extracted/name).read_bytes()==body,'whole actual downloaded package: '+name)
    lock=(consumer/'Cargo.lock').read_bytes();nodes=m.registry_nodes(lock)
    old={(p['name'],p['version']):p for p in m.registry_nodes((m.BASE/'input/U-fixture.lock').read_bytes())}
    for node in nodes:
        if node['name']==m.NAME:c.need(node['checksum']==receipt['sha256'] and node['source']=='registry+'+positive.as_uri(),'actual registry checksum')
        else:c.need((node['name'],node['version']) in old and node['checksum']==old[node['name'],node['version']]['checksum'] and node['source']==old[node['name'],node['version']]['source'],'no unrelated registry version/source drift')
    registry_files={str((positive/'config.json').relative_to(N)):c.sha((positive/'config.json').read_bytes()),
        str((positive/'hy/dr'/m.NAME).relative_to(N)):c.sha((positive/'hy/dr'/m.NAME).read_bytes()),
        str((N/'registry-index-download'/(m.PREFIX+'.crate')).relative_to(N)):receipt['sha256']}
    c.write(m.BASE/'state','consumer.json',dict(source=c.tree(consumer),downloaded_path=str(extracted),downloaded={n:c.sha(b) for n,b in files.items()},registry='registry+'+positive.as_uri(),checksum=receipt['sha256'],registry_files=registry_files))
    badhome=N/'bad-cargo-home';badhome.mkdir()
    (badhome/'config.toml').write_bytes(config+b'\n[registries.hc074-draft]\nindex="'+negative.as_uri().encode()+b'"\n')
    badconsumer=N/'bad-consumer';shutil.copytree(consumer,badconsumer,ignore=shutil.ignore_patterns('Cargo.lock'))
    badenv=dict(seed,CARGO_HOME=str(badhome))
    refused=invoke('checksum-negative',[str(tool/'cargo'),'metadata','--manifest-path',str(badconsumer/'Cargo.toml'),'--all-features','--format-version','1'],badenv,deadline,expect=101)
    error=(refused/'stderr.log').read_text();c.need('checksum' in error.lower() and m.NAME in error,'actual checksum refusal, not another preparation failure')
    c.write(OUT,'registry-review.json',dict(complete=True,local_registry_resolved=True,checksum_negative=True,patch_fallback=False,public_registry_qualified=False,root_adopted=False))

def consumer_phase(compiler,env,tool,deadline):
    frame=json.loads((m.BASE/'state/consumer.json').read_bytes());consumer=N/'consumer';manifest=consumer/'Cargo.toml';target=N/'builds'/('consumer'+compiler)
    c.need(c.tree(consumer)==frame['source'],'exact external consumer source/lock')
    def held():
        c.need(c.tree(consumer)==frame['source'],'same complete consumer source/lock')
        for n,digest in frame['downloaded'].items():c.need(c.sha((Path(frame['downloaded_path'])/n).read_bytes())==digest,'whole actual registry package')
    held()
    invoke('default-check',[str(tool/'cargo'),'check','--manifest-path',str(manifest),'--target-dir',str(target),'--offline','--locked','--release','--lib','--no-default-features'],env,deadline)
    built=invoke('build',[str(tool/'cargo'),'test',*args(manifest,target),'--lib','--no-run','--message-format=json'],env,deadline)
    image=artifact(built,manifest,'hc074-owned-consumer');c.write(OUT,'image.json',image)
    rows=[json.loads(v) for v in (built/'stdout.log').read_text().splitlines() if v.startswith('{')]
    fork=[r for r in rows if r.get('reason')=='compiler-artifact' and r['target']['name']=='chitchat' and r['target']['kind']==['lib']]
    c.need(len(fork)==1 and not fork[0]['fresh'] and not fork[0]['profile']['test'] and Path(fork[0]['target']['src_path']).resolve()==Path(frame['downloaded_path'])/'src/lib.rs','fresh actual normal registry dependency, not test overlay')
    names=listing(invoke('catalog',[image['path'],'--list'],env,deadline,10));c.need(len(names)==6,'complete public consumer catalog')
    c.write(OUT,'catalog.json',names)
    for action in ('check','clippy'):
        argv=[str(tool/'cargo'),action,*args(manifest,target),'--all-targets']
        if action=='clippy':argv+=['--','-D','warnings']
        invoke(action,argv,env,deadline)
    for i,test in enumerate(names):
        folder=invoke(f'case-{i:02}',[image['path'],'--exact',test,'--test-threads=1','--nocapture'],env,deadline,5,cwd=consumer)
        raw=(folder/'stdout.log').read_text();c.need(f'1 passed; 0 failed; 0 ignored; 0 measured; {len(names)-1} filtered out;' in raw,'complete public caller accounting');held()
    c.need(c.sha(Path(image['path']).read_bytes())==image['sha256'],'held original consumer image postflight')
    c.write(OUT,'consumer-review.json',dict(complete=True,passed=6,actual_registry=True,public_registry_qualified=False,root_adopted=False))

def outer(phase):
    global OUT
    c.need(phase in PHASES and not (m.BASE/'STOP.json').exists(),'fixed phase/no continuation after first refusal');OUT=m.BASE/'native'/phase;OUT.mkdir(parents=True,exist_ok=False)
    before=s.source();head=c.snapshot();argv=[str(Path(os.environ['SystemRoot'])/'System32/wsl.exe'),'-d','Ubuntu','-u','artur','--cd','/','--exec','/usr/bin/python3','-B','/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/chitchat-package-r4-074/run.py','session',phase]
    c.write(OUT,'intent.json',dict(argv=argv,source=before,git=head,bound=1200,session_bound=1100,floor=4*1024**3,poll=5,cleanup=5,retry=False))
    result=dict(complete=False,root_adopted=False,registry_qualified=False,release_admitted=False)
    with (OUT/'stdout.log').open('xb') as stdout,(OUT/'stderr.log').open('xb') as stderr:
        try:
            child=subprocess.run([sys.executable,'-B',str(Path(__file__)),'outer-worker',phase],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,timeout=1210)
            own=json.loads((OUT/'execution.json').read_bytes());c.need(child.returncode==own['exit']==0 and own['retired'] and not own['errors'],'complete original outer Job')
            c.need(s.source()==before and c.snapshot()==head and json.loads((OUT/'session-result.json').read_bytes())['complete'],'whole Root/source/ref/session postflight');result['complete']=True
        except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
    c.write(OUT,'result.json',result)
    if not result['complete']:
        try:c.write(m.BASE,'STOP.json',dict(phase=phase,first_refusal=result['first_refusal'],root_adopted=False))
        except FileExistsError:pass  # First refusal remains immutable; retain this phase result too.
    print(json.dumps(dict(phase=phase,**result)));raise SystemExit(0 if result['complete'] else 1)

if __name__=='__main__':
    c.need(not (m.BASE/'STOP.json').exists(),'first refusal stops unchanged dispatch')
    action=sys.argv[1]
    if action=='run':outer(sys.argv[2])
    elif action=='session':session(sys.argv[2])
    elif action=='outer-worker':
        phase=sys.argv[2];out=m.BASE/'native'/phase;intent=json.loads((out/'intent.json').read_bytes());_,env,_=s.environment();os.environ.clear();os.environ.update(env)
        own=c.owned._owned(c.owned.load_job().WindowsJob(),intent['argv'],str(m.ROOT),s.capacity,bound=1200);c.write(out,'execution.json',own);raise SystemExit(own['exit'])
    elif action=='command':
        folder=Path(sys.argv[2]);c.need(folder.resolve().is_relative_to((m.BASE/'native').resolve()),'contained original command output')
        intent=json.loads((folder/'intent.json').read_bytes());os.environ.clear();os.environ.update(intent['env'])
        group=c.load(m.ROOT/'scripts/ci/watchdog_linux_group.py','chitchat_package_group').LinuxGroup()
        own=c.owned._owned(group,intent['argv'],intent['cwd'],s.capacity,bound=intent['bound'],deadline=min(intent['deadline'],time.monotonic()+intent['bound']));c.write(folder,'execution.json',own);raise SystemExit(own['exit'])
    else:raise ValueError('fixed package commands')
