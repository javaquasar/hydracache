"""Focused actual package lanes in private copies, never the old full V6 gate."""
import hashlib,json,os,re,shutil,subprocess,sys,time,traceback
from pathlib import Path
import root_context as s
c=s.c;m=s.m;OUT=None
SHORT={'adapter':'hydracache-cluster-chitchat','testkit':'hydracache-cluster-testkit','raft':'hydracache-cluster-raft','sandbox':'hydracache-sandbox'}

def parse(phase):
    if phase in ('stage','metadata'):return phase,None,None,None
    parts=phase.split('-');c.need(len(parts)==4 and parts[0] in ('check','lint','test') and parts[1] in ('B','O') and parts[2] in ('194','188') and parts[3] in SHORT,'fixed copied-Root focused lane')
    return tuple(parts)

def invoke(label,argv,env,deadline,cwd=None,mutate=False,bound=780):
    before=s.source();head=c.snapshot();s.capacity()
    if not mutate:s.actual()
    folder=OUT/label;folder.mkdir();cwd=str(cwd or s.N);image=c.sha(Path(argv[0]).read_bytes())
    c.write(folder,'intent.json',dict(argv=argv,env=env,cwd=cwd,deadline=deadline,bound=bound,image_sha256=image,retry=False))
    with (folder/'stdout.log').open('xb') as stdout,(folder/'stderr.log').open('xb') as stderr:
        child=subprocess.run([sys.executable,'-B',str(Path(__file__)),'command',str(folder)],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,env=env,timeout=max(0,min(bound,deadline-time.monotonic()))+10)
    owner=json.loads((folder/'execution.json').read_bytes());c.need(child.returncode==owner['exit']==0 and owner['retired'] and not owner['errors'],'complete first copied-Root command: '+label)
    c.need(s.source()==before and c.snapshot()==head and c.sha(Path(argv[0]).read_bytes())==image,'whole original source/ref/image postflight')
    if not mutate:s.actual()
    return folder

def mirror(deadline):
    ledger=json.loads((s.INPUT/'registry.json').read_bytes())
    for name,digest in ledger.items():
        c.need(time.monotonic()<deadline,'original mirror validation deadline');s.capacity();folder=s.VENDOR/name
        raw=(s.INPUT/'archives'/(name+'.crate')).read_bytes()
        values=c.tree(folder);row=json.loads((folder/'.cargo-checksum.json').read_bytes());values.pop('.cargo-checksum.json')
        s.verify_mirror(raw,name,digest,values,row)

def git(v,label,args,env,deadline,mutate=False):
    argv=['/usr/bin/git','-c','core.hooksPath=/dev/null','-c','commit.gpgsign=false','-c','user.name=HydraCache Copied Root Diagnostic','-c','user.email=diagnostic@invalid','-C',str(s.N/'copies'/v),*args]
    return invoke(label,argv,env,deadline,mutate=mutate)

def git_tree(raw,files):
    """Compare actual committed blobs with all canonical source, not HEAD text."""
    rows={}
    for record in raw.split(b'\0'):
        if not record:continue
        head,name=record.split(b'\t',1);mode,kind,digest=head.split(b' ')
        c.need(mode==b'100644' and kind==b'blob','regular canonical Git source mode')
        name=name.decode();c.need(name not in rows,'no duplicate committed source');rows[name]=digest.decode()
    expected={n:hashlib.sha1(b'blob '+str(len(b)).encode()+b'\0'+b).hexdigest() for n,b in files.items()}
    c.need(rows==expected,'every actual committed source blob equals held canonical source')

def stage(env,deadline):
    c.need(not s.N.exists(),'exclusive copied Root directory');s.N.mkdir(parents=True,mode=0o700)
    source=m.codec().unpack((s.INPUT/'source.tar').read_bytes(),'hc074-copied-root',True);expected=json.loads((s.INPUT/'source-map.json').read_bytes())
    c.need({n:c.sha(b) for n,b in source.items()}==expected,'whole canonical frozen Root source archive')
    proposal=json.loads((s.INPUT/'proposal.json').read_bytes())
    for v in ('B','O'):
        before=s.source();head=c.snapshot();s.capacity()
        files=dict(source);files['Cargo.toml']=s.variant(source['Cargo.toml'],proposal,v)
        root=s.N/'copies'/v;donor=s.DONOR/'copies'/v
        for base,dirs,names in os.walk(donor,followlinks=False):
            for name in dirs+names:
                p=Path(base)/name;c.need(not p.is_symlink(),'regular contained donor source/Git data')
        actual=c.tree(donor);expected={n:c.sha(b) for n,b in files.items()}
        c.need({n:h for n,h in actual.items() if not n.startswith('.git/')}==expected,'whole exact failed-context donor source, no product execution')
        shutil.copytree(donor,root);c.need(c.tree(donor)==actual==c.tree(root),'whole read-only donor and exact exclusive copied bytes')
        c.write(s.BASE/'state','staged-'+v+'.json',{n:c.sha(b) for n,b in files.items()})
        c.need(s.source()==before and c.snapshot()==head,'whole original source/ref frame around private copy effect');s.capacity()
    (s.N/'cargo-home').mkdir();reference=json.loads((s.INPUT/'registry-reference.json').read_bytes())
    config='[source.crates-io]\nreplace-with="copied-root-reviewed"\n[source.copied-root-reviewed]\ndirectory="'+s.VENDOR.as_posix()+'"\n[registries.hc074-draft]\nindex="'+reference['registry'].removeprefix('registry+')+'"\n'
    (s.N/'cargo-home/config.toml').write_text(config,encoding='utf8');c.write(s.BASE/'state','config.json',dict(sha256=c.sha(config.encode())))
    s.actual();mirror(deadline)
    for v in ('B','O'):
        out=git(v,'committed-source-'+v,['ls-tree','-r','-z','HEAD'],env,deadline)
        files=dict(source);files['Cargo.toml']=s.variant(source['Cargo.toml'],proposal,v);git_tree((out/'stdout.log').read_bytes(),files)
        root=s.N/'copies'/v;c.need((root/'.git/HEAD').read_bytes()==b'ref: refs/heads/main\n','same independent synthetic branch')
        c.write(s.BASE/'state','git-staged-'+v+'.json',dict(synthetic_fixture_commit=(root/'.git/refs/heads/main').read_bytes().decode().strip(),product_identity=False,
            config_sha256=c.sha((s.N/'copies'/v/'.git/config').read_bytes())))

def metadata(tool,env,deadline):
    for v in ('B','O'):
        root=s.N/'copies'/v;argv=[str(tool/'cargo'),'metadata','--manifest-path',str(root/'Cargo.toml'),'--all-features','--format-version','1']
        if v=='B':argv+=['--locked','--offline']
        out=invoke('metadata-'+v,argv,dict(env,CARGO_NET_OFFLINE='false',CARGO_NET_RETRY='0') if v=='O' else env,deadline,cwd=root,mutate=v=='O')
        data=json.loads((out/'stdout.log').read_bytes());c.need(data['workspace_root']==str(root),'actual copied full workspace')
        raw=(root/'Cargo.lock').read_bytes()
        if v=='B':c.need(raw==(s.INPUT/'base.lock').read_bytes(),'exact unchanged baseline lock')
        else:
            review=s.resolve_graph(raw,(s.INPUT/'base.lock').read_bytes());c.write(out,'graph-review.json',review)
            expected=json.loads((s.BASE/'state/staged-O.json').read_bytes())
            for name,digest in expected.items():
                if name!='Cargo.lock':c.need(c.sha((root/name).read_bytes())==digest,'only copied owned lock resolution')
            expected['Cargo.lock']=c.sha(raw);c.write(s.BASE/'state','resolved-O.json',expected)
            git(v,'lock-add-O',['add','--force','--','Cargo.lock'],env,deadline)
            git(v,'lock-commit-O',['commit','--quiet','-m','normalized private maintained identity lock'],env,deadline,mutate=True)
            row=json.loads((s.BASE/'state/git-staged-O.json').read_bytes());row['synthetic_fixture_commit']=(root/'.git/refs/heads/main').read_bytes().decode().strip()
            c.write(s.BASE/'state','git-resolved-O.json',row)
        (s.BASE/'state'/('lock-'+v+'.toml')).write_bytes(raw)
        (s.BASE/'state'/('metadata-'+v+'.json')).write_bytes((out/'stdout.log').read_bytes())
        packages=[p for p in data['packages'] if p['name'] in s.PACKAGES];c.need({p['name'] for p in packages}==set(s.PACKAGES),'all actual four direct caller packages')
        owned=[p for p in data['packages'] if p['name']==m.NAME];legacy=[p for p in data['packages'] if p['name']=='chitchat']
        c.need((v=='B' and len(legacy)==1 and not owned) or (v=='O' and len(owned)==1 and not legacy),'one coherent selected identity, no mixed Chitchat graph')
        chosen=(legacy or owned)[0]['id']
        if v=='O':
            reference=json.loads((s.INPUT/'registry-reference.json').read_bytes());package=owned[0]
            c.need(package['version']==m.VERSION and package['source']==reference['registry'],'actual exact private registry dependency')
            folder=Path(package['manifest_path']).parent;files=m.codec().unpack((s.INPUT/'owned.crate').read_bytes(),m.PREFIX)
            values=c.tree(folder);values.pop('.cargo-ok',None);values.pop('.cargo-checksum.json',None)
            c.need(values=={n:c.sha(b) for n,b in files.items()},'whole real maintained dependency source')
            c.write(s.BASE/'state','owned-download.json',dict(path=str(folder),files=values,source=package['source'],version=package['version']))
        for pkg in packages:
            node=next(n for n in data['resolve']['nodes'] if n['id']==pkg['id']);edge=[d for d in node['deps'] if d['name']=='chitchat']
            c.need(len(edge)==1 and edge[0]['pkg']==chosen,'actual caller alias resolves exact same package identity')
    c.write(OUT,'metadata-review.json',dict(complete=True,actual_caller_packages=4,root_adopted=False,public_registry_qualified=False))

def test_outcome(raw):
    text=raw.decode();runs=re.findall(r'^running (\d+) tests?$',text,re.M)
    rows=re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',text,re.M)
    c.need(runs and len(runs)==len(rows),'whole default actual test-profile outcomes')
    total=ignored=0
    for count,row in zip(runs,rows,strict=True):
        passed,failed,skip,measured,filtered=map(int,row);c.need(failed==measured==filtered==0 and passed+skip==int(count),'complete unfiltered default case accounting');total+=passed;ignored+=skip
    return dict(passed=total,ignored=ignored,targets=len(rows),complete=True,root_adopted=False,release_admitted=False)

def lane(action,v,compiler,short,tool,env,deadline):
    package=SHORT[short];root=s.N/'copies'/v;base=['--manifest-path',str(root/'Cargo.toml'),'--target-dir',str(s.N/'builds'/compiler),'-p',package,'--locked','--offline']
    argv=[str(tool/'cargo'),'clippy' if action=='lint' else action,*base]
    if action in ('check','lint'):
        argv+=['--all-targets','--all-features']
        if action=='lint':argv+=['--','-D','warnings']
    else:argv+=['--tests','--all-features','--no-fail-fast','--message-format=json','--','--test-threads=4']
    out=invoke('cargo-'+action,argv,env,deadline,cwd=root)
    if action=='test':
        result=test_outcome((out/'stdout.log').read_bytes());c.write(out,'test-review.json',result)
        records=[json.loads(line) for line in (out/'stdout.log').read_bytes().decode().splitlines() if line.startswith('{')]
        data=json.loads((s.BASE/'state'/('metadata-'+v+'.json')).read_bytes());pkg=next(p for p in data['packages'] if p['name']==package)
        artifacts=[r for r in records if r.get('reason')=='compiler-artifact' and r.get('executable') and r['profile']['test'] and r['package_id']==pkg['id']]
        c.need(len(artifacts)==result['targets'],'all actual original package test-profile artifacts, no filtered target')
        c.write(out,'images.json',[dict(path=r['executable'],sha256=c.sha(Path(r['executable']).read_bytes()),src=r['target']['src_path'],fresh=r['fresh']) for r in artifacts])

def session(phase):
    global OUT
    OUT=s.BASE/'native'/phase;deadline=time.monotonic()+1100;result=dict(complete=False,root_adopted=False,release_admitted=False)
    try:
        action,v,compiler,short=parse(phase);compiler=compiler or '194';tool,env,_=s.environment(compiler)
        c.need(os.getuid()==os.getgid()==1000,'original nonroot GNU owner');s.capacity();before=s.source();boot=Path('/proc/sys/kernel/random/boot_id').read_bytes()
        if action=='stage':stage(env,deadline)
        else:
            required='stage' if action=='metadata' else 'metadata'
            c.need(json.loads((s.BASE/'native'/required/'result.json').read_bytes())['complete'],'complete source/metadata prerequisite')
            if action not in ('metadata',):
                if v=='O':c.need(json.loads((s.BASE/'native'/f'{action}-B-{compiler}-{short}'/'result.json').read_bytes())['complete'],'same exact baseline lane before candidate')
                if action=='lint':c.need(json.loads((s.BASE/'native'/f'check-{v}-{compiler}-{short}'/'result.json').read_bytes())['complete'],'same compiler/package check before lint')
                if action=='test':c.need(json.loads((s.BASE/'native'/f'lint-{v}-{compiler}-{short}'/'result.json').read_bytes())['complete'],'same compiler/package lint before runtime')
            s.actual();mirror(deadline)
            if action=='metadata':metadata(tool,env,deadline)
            else:
                version=invoke('rustc-version',[str(tool/'rustc'),'-vV'],env,deadline,bound=10)
                c.need(('release: '+('1.88.0' if compiler=='188' else '1.94.0')+'\n').encode() in (version/'stdout.log').read_bytes(),'actual pinned compiler')
                lane(action,v,compiler,short,tool,env,deadline)
            mirror(deadline);s.actual()
        c.need(s.source()==before and Path('/proc/sys/kernel/random/boot_id').read_bytes()==boot and time.monotonic()<deadline,'whole original source/boot/deadline postflight');result['complete']=True
    except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
    c.write(OUT,'session-result.json',result);raise SystemExit(0 if result['complete'] else 1)

def outer(phase):
    parse(phase);c.need(not (s.BASE/'STOP.json').exists(),'first unexpected refusal stops this context')
    out=s.BASE/'native'/phase;out.mkdir(parents=True,exist_ok=False);before=s.source();head=c.snapshot()
    argv=[str(Path(os.environ['SystemRoot'])/'System32/wsl.exe'),'-d','Ubuntu','-u','artur','--cd','/','--exec','/usr/bin/python3','-B','/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/chitchat-copied-root-074/runner.py','session',phase]
    c.write(out,'intent.json',dict(argv=argv,source=before,git=head,bound=1200,session_bound=1100,floor=4*1024**3,poll=5,cleanup=5,retry=False))
    result=dict(complete=False,root_adopted=False,release_admitted=False)
    with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
        try:
            child=subprocess.run([sys.executable,'-B',str(Path(__file__)),'outer-worker',phase],stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,timeout=1210)
            owner=json.loads((out/'execution.json').read_bytes());c.need(child.returncode==owner['exit']==0 and owner['retired'] and not owner['errors'],'complete original copied-Root Job')
            c.need(s.source()==before and c.snapshot()==head and json.loads((out/'session-result.json').read_bytes())['complete'],'whole source/ref/session postflight');result['complete']=True
        except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc())
    c.write(out,'result.json',result)
    if not result['complete']:
        try:c.write(s.BASE,'STOP.json',dict(phase=phase,first_refusal=result['first_refusal'],root_adopted=False))
        except FileExistsError:pass
    print(json.dumps(dict(phase=phase,**result)));raise SystemExit(0 if result['complete'] else 1)

if __name__=='__main__':
    action=sys.argv[1];c.need(not (s.BASE/'STOP.json').exists(),'first refusal stops unchanged dispatch')
    if action=='run':outer(sys.argv[2])
    elif action=='session':session(sys.argv[2])
    elif action=='outer-worker':
        phase=sys.argv[2];out=s.BASE/'native'/phase;intent=json.loads((out/'intent.json').read_bytes());_,env,_=s.environment('194');os.environ.clear();os.environ.update(env)
        owner=c.owned._owned(c.owned.load_job().WindowsJob(),intent['argv'],str(s.ROOT),s.capacity,bound=1200);c.write(out,'execution.json',owner);raise SystemExit(owner['exit'])
    elif action=='command':
        out=Path(sys.argv[2]);c.need(out.resolve().is_relative_to((s.BASE/'native').resolve()),'contained original output');intent=json.loads((out/'intent.json').read_bytes());os.environ.clear();os.environ.update(intent['env'])
        group=c.load(s.ROOT/'scripts/ci/watchdog_linux_group.py','copied_root_original_group').LinuxGroup()
        owner=c.owned._owned(group,intent['argv'],intent['cwd'],s.capacity,bound=intent['bound'],deadline=min(intent['deadline'],time.monotonic()+intent['bound']));c.write(out,'execution.json',owner);raise SystemExit(owner['exit'])
    else:raise ValueError('fixed copied-Root command')
