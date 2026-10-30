"""Approved private Root copies; original product tree and all source pins stay."""
import json,os,shutil,sys,tomllib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(ROOT/'tools/chitchat-public-identity-074'));import contract as abi
c=abi.c;m=abi.m;prior=abi.prior
FIRST=ROOT/'target/chitchat-copied-root-20261030'
SECOND=ROOT/'target/chitchat-copied-root-20261030-r2'
BASE=ROOT/'target/chitchat-copied-root-20261030-r3'
INPUT=FIRST/'input'
N=Path('/home/artur/.cache/hydracache-074-copied-root/20261030-a3')
DONOR=Path('/home/artur/.cache/hydracache-074-copied-root/20261030-a2')
VENDOR=Path('/home/artur/.cache/hydracache-074-copied-root/20261030-a1/vendor')
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
    for root in (HERE,INPUT,BASE/'input'):
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
    if (INPUT/'registry-reference.json').exists():
        row=json.loads((INPUT/'registry-reference.json').read_bytes());root=Path('/home/artur/.cache/hydracache-074-package/20261030-a6')
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

def verify_mirror(raw,name,digest,values,row):
    c.need(c.sha(raw)==digest,'whole original official mirror archive')
    expected={n:c.sha(b) for n,b in m.codec().unpack(raw,name).items()}
    c.need(values==row['files']==expected and row['package']==digest,'whole ordinary archive-bound read-only mirror source')

def resolve_graph(raw,before):
    old=tomllib.loads(before.decode())['package'];new=tomllib.loads(raw.decode())['package']
    old_nodes={(p['name'],p['version'],p.get('source')):p for p in old};new_nodes={(p['name'],p['version'],p.get('source')):p for p in new}
    old_edges,new_edges=lock_edges(old),lock_edges(new)
    allowed=json.loads((INPUT/'registry.json').read_bytes());reference=json.loads((INPUT/'registry-reference.json').read_bytes())
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
    # The first context is terminal, never resumed. Borrow only its immutable
    # input bytes and fully revalidate its unpacked official vendor data.
    owner=json.loads((FIRST/'native/stage/execution.json').read_bytes())
    c.need(owner['classification']=='resource-floor' and owner['exit']==125 and owner['retired'],'retained first resource refusal')
    c.need((FIRST/'STOP.json').is_file() and not (FIRST/'native/stage/session-result.json').exists(),'no invented first inner result')
    c.need(not (FIRST/'native/stage/init-B').exists(),'first attempt did not invoke Git or Cargo')
    owner2=json.loads((SECOND/'native/stage/execution.json').read_bytes())
    c.need(owner2['classification']=='resource-floor' and owner2['exit']==125 and owner2['retired'] and (SECOND/'STOP.json').is_file(),'retained second capacity refusal')
    for label in ('init-B','add-B','commit-B','head-B','init-O','add-O','commit-O'):
        row=json.loads((SECOND/'native/stage'/label/'execution.json').read_bytes())
        c.need(row['exit']==0 and row['retired'] and not row['errors'],'complete original source-only fixture command')
    BASE.mkdir(exist_ok=True);(BASE/'input').mkdir();(BASE/'state').mkdir()
    ledger=json.loads((INPUT/'registry.json').read_bytes())
    for name,digest in ledger.items():c.need(c.sha((INPUT/'archives'/(name+'.crate')).read_bytes())==digest,'whole held official Root input')
    expected=json.loads((INPUT/'source-map.json').read_bytes())
    raw=m.codec().unpack((INPUT/'source.tar').read_bytes(),'hc074-copied-root',True)
    c.need({n:c.sha(b) for n,b in raw.items()}==expected,'whole original canonical source input')
    c.need((ROOT/'Cargo.toml').read_bytes()==raw['Cargo.toml'] and (ROOT/'Cargo.lock').read_bytes()==raw['Cargo.lock'],'same unchanged actual Root')
    c.write(BASE/'input','reuse.json',dict(first_context=str(FIRST),first_owner_sha256=c.sha((FIRST/'native/stage/execution.json').read_bytes()),
        input={p.relative_to(INPUT).as_posix():c.sha(p.read_bytes()) for p in INPUT.rglob('*') if p.is_file()},
        vendor=str(VENDOR),git_source_donor=str(DONOR),second_owner_sha256=c.sha((SECOND/'native/stage/execution.json').read_bytes()),
        first_refusal_retained=True,retry_to_pass=False,root_adopted=False))
    print(json.dumps(dict(complete=True,read_only_archives=len(ledger),source_files=len(expected),native_context=str(N),root_adopted=False)))
    return
