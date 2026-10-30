"""Approved private Root copies; original product tree and all source pins stay."""
import json,os,shutil,sys,tomllib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(ROOT/'tools/chitchat-public-identity-074'));import contract as abi
c=abi.c;m=abi.m;prior=abi.prior
BASE=ROOT/'target/chitchat-copied-root-20261030'
N=Path('/home/artur/.cache/hydracache-074-copied-root/20261030-a1')
PACKET=ROOT/'docs/testing/performance/0.74/local-runs/chitchat-public-identity-20261030'
PARENT_SHA='06ce2017ce7e2c0b9474914cc134025d95dcb2b2dd0ab8a6892ee87d7b4e8454'
PROPOSAL=ROOT/'docs/testing/performance/0.74/chitchat-public-identity-migration-proposal-20261030.json'
PACKAGES=('hydracache-cluster-chitchat','hydracache-cluster-testkit','hydracache-cluster-raft','hydracache-sandbox')

def variant(raw,proposal,which):
    c.need(which in ('B','O'),'exact two Root variants')
    c.need(c.sha(raw)==proposal['before_sha256'] and raw.count(proposal['exact_before'].encode())==1,'whole approved manifest preimage')
    if which=='B':return raw
    shipping=raw.replace(proposal['exact_before'].encode(),proposal['exact_after'].encode())
    c.need(c.sha(shipping)==proposal['after_sha256'],'exact approved shipping proposal')
    old=proposal['exact_after'].encode();new=old[:-2]+b', registry = "hc074-draft" }'
    result=shipping.replace(old,new)
    c.need(result.replace(new,old)==shipping and result.replace(new,proposal['exact_before'].encode())==raw,'exact locator-only simulation and complete inverse')
    return result

def source():
    values=prior.source()
    for root in (HERE,BASE/'input'):
        for path in root.rglob('*'):
            if '__pycache__' in path.parts:continue
            c.need(not path.is_symlink() and not getattr(path,'is_junction',lambda:False)(),'regular copied-Root source/input')
            if path.is_file():values[path.relative_to(ROOT).as_posix()]=c.sha(path.read_bytes())
    c.need(c.sha((PACKET/'index.json').read_bytes())==PARENT_SHA,'immutable exact identity parent')
    values[(PACKET/'index.json').relative_to(ROOT).as_posix()]=PARENT_SHA
    return values

def capacity():
    prior.capacity()
    if os.name=='posix' and N.exists() and shutil.disk_usage(N).free<4*1024**3:raise c.owned.CapacityError('original copied-Root package4GiB native floor')

def environment(compiler):
    tool,env,suffix=prior.environment(compiler)
    if os.name=='posix':env['CARGO_HOME']=str(N/'cargo-home')
    return tool,env,suffix

def actual():
    for v in ('B','O'):
        frame=BASE/'state'/('resolved-'+v+'.json')
        if not frame.exists():frame=BASE/'state'/('staged-'+v+'.json')
        if not frame.exists():continue
        expected=json.loads(frame.read_bytes());root=N/'copies'/v
        for name,digest in expected.items():c.need(c.sha((root/name).read_bytes())==digest and not (root/name).is_symlink(),'whole real copied Root source: '+v+'/'+name)
        # Runtime tests may create only the declared ignored target outputs.
        for base,dirs,files in os.walk(root,followlinks=False):
            for name in dirs+files:
                path=Path(base)/name;c.need(not path.is_symlink() and not getattr(path,'is_junction',lambda:False)(),'regular actual copied source')
            if Path(base)==root:dirs[:]=[n for n in dirs if n not in ('.git','target')]
            for name in files:c.need((Path(base)/name).relative_to(root).as_posix() in expected,'no undeclared copied source output')
        git=BASE/'state'/('git-resolved-'+v+'.json')
        if not git.exists():git=BASE/'state'/('git-staged-'+v+'.json')
        if git.exists():
            row=json.loads(git.read_bytes())
            c.need((root/'.git/HEAD').read_bytes()==b'ref: refs/heads/main\n' and (root/'.git/refs/heads/main').read_bytes().decode().strip()==row['synthetic_fixture_commit'],'fixed private synthetic Git ref')
            c.need(c.sha((root/'.git/config').read_bytes())==row['config_sha256'],'fixed private Git configuration')
    frame=BASE/'state/config.json'
    if frame.exists():c.need(c.sha((N/'cargo-home/config.toml').read_bytes())==json.loads(frame.read_bytes())['sha256'],'same original private registry configuration')
    if (BASE/'input/registry-reference.json').exists():
        row=json.loads((BASE/'input/registry-reference.json').read_bytes());root=Path('/home/artur/.cache/hydracache-074-package/20261030-a6')
        for name,digest in row['registry_files'].items():c.need(c.sha((root/name).read_bytes())==digest,'whole exact read-only qualified registry locator')
    frame=BASE/'state/owned-download.json'
    if frame.exists():
        row=json.loads(frame.read_bytes());values=c.tree(Path(row['path']));values.pop('.cargo-ok',None);values.pop('.cargo-checksum.json',None)
        c.need(values==row['files'],'whole actually downloaded maintained dependency, no overlay')

def lock_edges(rows):
    result={}
    for row in rows:
        children=set()
        for token in row.get('dependencies',[]):
            parts=token.split(' ',2);name=parts[0];version=parts[1] if len(parts)>1 else None
            origin=parts[2][1:-1] if len(parts)>2 else None
            matches=[p for p in rows if p['name']==name and (version is None or p['version']==version) and (origin is None or p.get('source')==origin)]
            c.need(len(matches)==1,'unambiguous whole semantic Root edge')
            p=matches[0];children.add((p['name'],p['version'],p.get('source')))
        result[row['name'],row['version'],row.get('source')]=children
    return result

def resolve_graph(raw,before):
    old=tomllib.loads(before.decode())['package'];new=tomllib.loads(raw.decode())['package']
    old_nodes={(p['name'],p['version'],p.get('source')):p for p in old};new_nodes={(p['name'],p['version'],p.get('source')):p for p in new}
    old_edges,new_edges=lock_edges(old),lock_edges(new)
    allowed=json.loads((BASE/'input/registry.json').read_bytes());reference=json.loads((BASE/'input/registry-reference.json').read_bytes())
    for p in new:
        if p.get('source'):
            if p['name']==m.NAME:c.need(p['version']==m.VERSION and p['source']==reference['registry'] and p['checksum']==abi.CRATE_SHA,'exact maintained owned lock identity')
            else:c.need(p['source']=='registry+'+m.CRATE_INDEX and allowed[p['name']+'-'+p['version']]==p['checksum'],'no unrelated registry version/source/checksum')
    removed=set(old_nodes)-set(new_nodes);added=set(new_nodes)-set(old_nodes)
    c.need(all(name in ('chitchat','lru','zstd','zstd-safe') for name,_,_ in removed) and all(name in (m.NAME,'lru') for name,_,_ in added),'only approved identity/LRU and obsolete Zstd nodes change')
    c.need(any(name==m.NAME for name,_,_ in added) and any(name=='chitchat' for name,_,_ in removed),'no remaining legacy identity')
    for key in set(old_nodes)&set(new_nodes):
        a,b=old_nodes[key],new_nodes[key];c.need({n:v for n,v in a.items() if n!='dependencies'}=={n:v for n,v in b.items() if n!='dependencies'},'retained whole node identity')
        if old_edges[key]!=new_edges[key]:
            c.need(a['name'] in PACKAGES,'only the four direct alias consumers change edges')
            expected={(m.NAME,m.VERSION,reference['registry']) if name=='chitchat' else (name,version,source) for name,version,source in old_edges[key]}
            c.need(expected==new_edges[key],'only exact Chitchat identity semantic edge substitution')
    return dict(complete=True,removed=[list(x) for x in sorted(removed,key=str)],added=[list(x) for x in sorted(added,key=str)],root_adopted=False)

def prepare():
    capacity()
    for mode in ('controls','syntax'):c.need(json.loads((BASE/'offline'/mode/'result.json').read_bytes())['complete'],'complete copied-Root pure preflight')
    BASE.mkdir(exist_ok=True);(BASE/'input/archives').mkdir(parents=True);(BASE/'state').mkdir()
    raw=(PACKET/'index.json').read_bytes();c.need(c.sha(raw)==PARENT_SHA,'whole identity control parent')
    index=json.loads(raw);c.need(index['result']['nominal_control_gate_pass'] and index['result']['mixed_identity_assignable'] is False,'explicit confirmed migration boundary')
    def get(name):
        row=index['members'][name];body=(PACKET/'objects'/row['sha256']).read_bytes();c.need(c.sha(body)==row['sha256'] and len(body)==row['bytes'],'whole qualified parent input');return body
    proposal=json.loads(PROPOSAL.read_bytes());c.need((ROOT/'Cargo.toml').read_bytes()==get('reference/root/Cargo.toml'),'same whole current Root preimage')
    source_map=json.loads(get('reference/frozen-root-map.json'))
    names=set(source_map)
    for prefix in ('README*','LICENSE*','.gitignore','.gitattributes','.cargo','docs/testing/compat'):
        names.update(n.decode() for n in c.git('ls-files','-z','--',prefix).split(b'\0') if n)
    files={}
    for name in sorted(names):
        m.codec().safe_name(name);path=ROOT/name;c.need(path.is_file() and path.resolve().is_relative_to(ROOT.resolve()) and not path.is_symlink() and not path.is_junction(),'exact regular tracked copied input')
        body=path.read_bytes()
        if name in source_map:c.need(c.sha(body)==source_map[name],'whole original Root source map')
        files[name]=body
    c.write(BASE/'input','source-map.json',{n:c.sha(b) for n,b in files.items()})
    (BASE/'input/source.tar').write_bytes(m.codec().pack(files,'hc074-copied-root'))
    (BASE/'input/base.lock').write_bytes(files['Cargo.lock'])
    (BASE/'input/proposal.json').write_bytes(PROPOSAL.read_bytes())
    (BASE/'input/parent-index.json').write_bytes(raw)
    (BASE/'input/owned.crate').write_bytes(get('input/owned.crate'))
    (BASE/'input/registry-reference.json').write_bytes(get('input/registry-reference.json'))
    ledger={p['name']+'-'+p['version']:p['checksum'] for p in tomllib.loads(files['Cargo.lock'].decode())['package'] if p.get('source')}
    cache=Path.home()/'.cargo/registry/cache/index.crates.io-1949cf8c6b5b557f'
    parent_ledger=json.loads(get('input/registry.json'));name='lru-0.18.2';ledger[name]=parent_ledger[name]
    total=0
    for name,digest in ledger.items():
        body=(cache/(name+'.crate')).read_bytes();c.need(c.sha(body)==digest,'whole lock-bound ordinary registry archive: '+name)
        (BASE/'input/archives'/(name+'.crate')).write_bytes(body);total+=len(body)
    c.write(BASE/'input','registry.json',ledger)
    c.write(BASE/'input','scope.json',dict(operator_approved_coordinated_migration=True,parent_sha256=PARENT_SHA,product_Root_changed=False,
        root_adopted=False,publication_authorized=False,public_registry_qualified=False,release_admitted=False,
        package_bound=780,test_threads=4,outer_bound=1200,native_bound=1100,floor=4*1024**3,poll=5,retirement=5,
        full_workspace_dispatched=False,workspace_bound=4800,workspace_start_floor=16*1024**3,retry=False))
    c.write(BASE/'input','variants.json',{v:dict(manifest_sha256=c.sha(variant(files['Cargo.toml'],proposal,v))) for v in ('B','O')})
    print(json.dumps(dict(complete=True,source_files=len(files),registry_archives=len(ledger),archive_bytes=total,root_adopted=False)))
