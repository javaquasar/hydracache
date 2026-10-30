"""Pinned source census and real-package nominal controls; no Root adoption."""
import json,os,re,shutil,sys,tomllib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(ROOT/'tools/chitchat-package-r5-074'));import common as prior
m=prior.m;c=prior.c
BASE=ROOT/'target/chitchat-public-identity-20261030'
N=Path('/home/artur/.cache/hydracache-074-identity/20261030-a1')
PACKET=ROOT/'docs/testing/performance/0.74/local-runs/chitchat-package-complete-20261030'
PARENT_SHA='28fad46b76c3f0702decfe4f0e816c66fd2d722cdbc4b5e6805ebbec99ca5a69'
CRATE_SHA='0296600cf25af830741ee575df5da4e050d48e7bc561ffbdcda63ebab12877c2'
VARIANTS=('positive','config','id','transport')
ERRORS={'config':'E0308','id':'E0308','transport':'E0277'}
EXPOSED={
 'failure_detector_config':'pub fn failure_detector_config(mut self, config: FailureDetectorConfig) -> Self',
 'spawn_with_transport':'pub async fn spawn_with_transport( config: ChitchatDiscoveryConfig, transport: &dyn Transport, ) -> CacheResult<Self>',
 'chitchat_id':'pub fn chitchat_id(&self) -> &ChitchatId'}
DEPENDENTS={'crates/hydracache-cluster-chitchat':'dependencies','crates/hydracache-cluster-raft':'dev-dependencies',
 'crates/hydracache-cluster-testkit':'dependencies','crates/hydracache-sandbox':'dependencies'}

def signatures(text):
    heads=re.findall(r'(?m)^    pub (?:async )?fn [^{]+',text)
    c.need(len(heads)==21,'all21 adapter public methods');found={}
    for head in heads:
        if not re.search(r'\b(?:FailureDetectorConfig|ChitchatId|Transport|Chitchat|ChitchatHandle|NodeState|UdpTransport)\b',head):continue
        name=re.search(r'\bfn (\w+)',head)[1];found[name]=' '.join(head.split())
    c.need(found==EXPOSED,'all three exact exposed nominal signatures');return found

def audit(root):
    workspace=tomllib.loads((root/'Cargo.toml').read_bytes().decode())['workspace'];direct=[];references={}
    c.need(workspace['dependencies']['chitchat']=='0.11.0','unchanged actual shared Root dependency')
    for member in workspace['members']:
        doc=tomllib.loads((root/member/'Cargo.toml').read_bytes().decode())
        def visit(value,path=()):
            if not isinstance(value,dict):return
            for key,row in value.items():
                if key=='chitchat' and path and path[-1] in ('dependencies','dev-dependencies','build-dependencies'):
                    direct.append(dict(member=member,section='.'.join(path),value=row))
                visit(row,(*path,key))
        visit(doc)
    c.need({r['member']:r['section'] for r in direct}==DEPENDENTS and len(direct)==4 and all(r['value']=={'workspace':True} for r in direct),'all four inherited direct consumers; no hidden target/alias override')
    path=root/'crates/hydracache-cluster-chitchat/src/lib.rs';text=path.read_bytes().decode()
    found=signatures(text)
    # Conservative textual reference inventory. This does not claim a syntax
    # census or compilation of the full adapter/reverse dependency closure.
    for raw in c.git('ls-files','-z','--','crates').split(b'\0'):
        if not raw or not raw.endswith(b'.rs'):continue
        name=raw.decode();body=(root/name).read_bytes().decode();rows=[]
        for line,value in enumerate(body.splitlines(),1):
            if 'chitchat::' in value:rows.append(dict(line=line,text=value))
        if rows:references[name]=rows
    return dict(public_methods=21,exposed=found,direct=direct,references=references,
        adapter_source_sha256=c.sha(path.read_bytes()),full_adapter_compiled=False,root_adopted=False)

def manifest(variant):
    c.need(variant in VARIANTS,'fixed real-package variant')
    return f'''[package]
name="hc074-identity-{variant}"
version="0.0.0"
edition="2024"
rust-version="1.88"
publish=false
[dependencies]
legacy={{package="chitchat",version="=0.11.1"}}
owned={{package="hydra-chitchat",version="=0.11.1-hydra.1",registry="hc074-draft"}}
[workspace]
resolver="2"
'''.encode()

def parent():
    raw=(PACKET/'index.json').read_bytes();c.need(c.sha(raw)==PARENT_SHA,'whole qualified package parent')
    index=json.loads(raw);result=index['result']
    c.need(result['full_library_gate_pass'] and result['full_library_cases']==200 and result['consumer_cases']==12 and result['local_registry_qualified'] and not result['root_adopted'],'actual qualified local package scope')
    def get(name):
        row=index['members'][name];body=(PACKET/'objects'/row['sha256']).read_bytes();c.need(c.sha(body)==row['sha256'] and len(body)==row['bytes'],'whole parent member');return body
    raw=get('capture/artifact/'+m.PREFIX+'.crate');c.need(c.sha(raw)==CRATE_SHA,'unchanged maintained archive')
    return get,raw

def source():
    value=prior.source()
    for root in (HERE,BASE/'source',BASE/'input'):
        for p in root.rglob('*'):
            if '__pycache__' in p.parts:continue
            c.need(not p.is_symlink() and not getattr(p,'is_junction',lambda:False)(),'regular nominal source/input')
            if p.is_file():value[p.relative_to(ROOT).as_posix()]=c.sha(p.read_bytes())
    c.need(c.sha((PACKET/'index.json').read_bytes())==PARENT_SHA,'immutable complete parent')
    value[(PACKET/'index.json').relative_to(ROOT).as_posix()]=PARENT_SHA
    return value

def capacity():
    prior.capacity()
    if os.name=='posix' and N.exists() and shutil.disk_usage(N).free<4*1024**3:raise c.owned.CapacityError('original nominal native4GiB floor')

def environment(compiler='194'):
    tool,env,suffix=prior.environment(compiler)
    if os.name=='posix':env['CARGO_HOME']=str(N/'cargo-home')
    return tool,env,suffix

def actual():
    for name in VARIANTS:
        frame=BASE/'state'/('resolved-'+name+'.json')
        if not frame.exists():frame=BASE/'state'/('stage-'+name+'.json')
        if frame.exists():c.need(c.tree(N/'source'/name)==json.loads(frame.read_bytes()),'whole actual fixed nominal source/lock')
    frame=BASE/'state/config.json'
    if frame.exists():c.need(c.sha((N/'cargo-home/config.toml').read_bytes())==json.loads(frame.read_bytes())['sha256'],'same private Cargo config')
    frame=BASE/'input/registry-reference.json'
    if frame.exists():
        reference=json.loads(frame.read_bytes())
        root=Path('/home/artur/.cache/hydracache-074-package/20261030-a6')
        for name,digest in reference['registry_files'].items():c.need(c.sha((root/name).read_bytes())==digest,'original read-only registry/index/archive')
    frame=BASE/'state/downloads.json'
    if frame.exists():
        for row in json.loads(frame.read_bytes()).values():
            values=c.tree(Path(row['path']));values.pop('.cargo-ok',None);values.pop('.cargo-checksum.json',None)
            c.need(values==row['files'],'whole actual real-package download')

def edges(raw):
    rows=tomllib.loads(raw.decode())['package'];result={}
    for row in rows:
        children=set()
        for token in row.get('dependencies',[]):
            parts=token.split();matches=[v for v in rows if v['name']==parts[0] and (len(parts)==1 or v['version']==parts[1])]
            c.need(len(matches)==1,'unambiguous approved dependency edge');v=matches[0];children.add((v['name'],v['version']))
        result[row['name'],row['version']]=children
    return result

def graph(metadata,variant):
    packages=metadata['packages'];owned=[r for r in packages if r['name']==m.NAME];legacy=[r for r in packages if r['name']=='chitchat']
    reference=json.loads((BASE/'input/registry-reference.json').read_bytes())
    c.need(len(owned)==len(legacy)==1 and owned[0]['version']==m.VERSION and owned[0]['source']==reference['registry'] and legacy[0]['version']=='0.11.1' and legacy[0]['source']=='registry+'+m.CRATE_INDEX,'both actual real registry identities')
    root=next(r for r in packages if r['id']==metadata['resolve']['root'])
    c.need(root['name']=='hc074-identity-'+variant and root['source'] is None,'exact local probe root')
    deps={r['rename']:r for r in root['dependencies']}
    c.need(set(deps)=={'legacy','owned'} and deps['legacy']['name']=='chitchat' and deps['owned']['name']==m.NAME,'exact caller aliases, no patch/path fallback')
    lock=(N/'source'/variant/'Cargo.lock').read_bytes();nodes=m.registry_nodes(lock);ledger=json.loads((BASE/'input/registry.json').read_bytes())
    for row in nodes:
        if row['name']==m.NAME:c.need(row['checksum']==CRATE_SHA and row['source']==reference['registry'],'exact maintained source/checksum')
        else:c.need(row['source']=='registry+'+m.CRATE_INDEX and ledger[row['name']+'-'+row['version']]==row['checksum'],'all ordinary nodes exact approved union')
    actual_edges=edges(lock);approved={}
    for v in ('B','Z','U'):
        for key,values in edges((BASE/'input'/('accepted-'+v+'.lock')).read_bytes()).items():approved.setdefault(key,set()).update(values)
    old=edges((BASE/'input/accepted-B.lock').read_bytes())['chitchat','0.11.1']
    new=edges((BASE/'input/accepted-U.lock').read_bytes())['chitchat','0.11.1']
    for row in nodes:
        key=row['name'],row['version'];allowed=new if row['name']==m.NAME else old if row['name']=='chitchat' else approved[key]
        c.need(actual_edges[key]<=allowed,'no new registry/feature edge outside the accepted graph')
    return nodes

def negative(rows,variant,src):
    c.need(variant in ERRORS,'fixed negative control')
    errors=[r for r in rows if r.get('reason')=='compiler-message' and r['message']['level']=='error']
    c.need(len(errors)==1 and errors[0]['message']['code']['code']==ERRORS[variant],'exact single nominal type refusal, not dependency failure')
    row=errors[0]
    def same_span(span):
        p=Path(span['file_name']);p=p if p.is_absolute() else src.parent.parent/p
        return span['is_primary'] and p.resolve()==src.resolve() and span['line_start']==2
    c.need(row['target']['src_path']==str(src) and any(same_span(x) for x in row['message']['spans']),'exact real probe diagnostic span')
    c.need([r['success'] for r in rows if r.get('reason')=='build-finished']==[False],'actual failed Cargo compilation')
    return dict(complete=True,expected_code=ERRORS[variant],actual_code=row['message']['code']['code'],probe=str(src),root_adopted=False)

def prepare():
    capacity()
    for mode in ('controls','syntax','format'):c.need(json.loads((BASE/'offline'/mode/'result.json').read_bytes())['complete'],'complete original nominal source control: '+mode)
    BASE.mkdir(exist_ok=True);(BASE/'input').mkdir();(BASE/'source').mkdir();(BASE/'state').mkdir()
    get,archive=parent();(BASE/'input/owned.crate').write_bytes(archive)
    (BASE/'input/registry-reference.json').write_bytes(get('capture/state/consumer.json'))
    (BASE/'input/parent-index.json').write_bytes((PACKET/'index.json').read_bytes())
    original=(m.PARENT/'index.json').read_bytes();c.need(c.sha(original)==m.PARENT_SHA,'whole original approved union ledger')
    original=json.loads(original)
    def original_bytes(name):
        row=original['members'][name];body=(m.PARENT/'objects'/row['sha256']).read_bytes()
        c.need(c.sha(body)==row['sha256'] and len(body)==row['bytes'],'whole original official source');return body
    ledger=json.loads(original_bytes('capture/r4/input/registry.json'))
    c.need(len(ledger)==94 and 'chitchat-0.11.1' not in ledger,'original94 transitive archives')
    legacy=original_bytes('capture/r4/input/archives/chitchat-0.11.1.crate');c.need(c.sha(legacy)==m.UPSTREAM_SHA,'exact official legacy root archive')
    (BASE/'input/legacy.crate').write_bytes(legacy);ledger['chitchat-0.11.1']=m.UPSTREAM_SHA
    c.write(BASE/'input','registry.json',ledger)
    for variant in ('B','Z','U'):(BASE/'input'/('accepted-'+variant+'.lock')).write_bytes(original_bytes('capture/r4/fixtures/'+variant+'/Cargo.lock'))
    for variant in VARIANTS:
        folder=BASE/'source'/variant;(folder/'src').mkdir(parents=True)
        (folder/'Cargo.toml').write_bytes(manifest(variant));(folder/'src/lib.rs').write_bytes((HERE/(variant+'.rs')).read_bytes())
    c.write(BASE/'input','audit.json',audit(ROOT));c.write(BASE/'input','scope.json',dict(parent_sha256=PARENT_SHA,crate_sha256=CRATE_SHA,
        package_bound=780,short_case_bound=5,catalog_bound=10,local_bound=1200,native_bound=1100,floor=4*1024**3,poll=5,retirement=5,
        real_packages=True,adapter_compiled=False,root_adopted=False,public_registry_qualified=False,release_admitted=False,retry=False))
    print(json.dumps(dict(complete=True,public_methods=21,public_boundaries=3,direct_consumers=4,root_adopted=False)))
