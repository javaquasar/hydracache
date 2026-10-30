"""Data-only copied-Root accounting; first results remain immutable."""
import gzip,hashlib,io,json,os,re,sys,tarfile
from pathlib import Path,PurePosixPath
ROOT=Path(__file__).resolve().parents[2]
FIRST=ROOT/'target/chitchat-copied-root-20261030'
SECOND=ROOT/'target/chitchat-copied-root-20261030-r2'
THIRD=ROOT/'target/chitchat-copied-root-20261030-r3'
OUT=ROOT/'docs/testing/performance/0.74/local-runs/chitchat-copied-root-20261030'
sha=lambda b:hashlib.sha256(b).hexdigest()
encoded=lambda v:(json.dumps(v,sort_keys=True,indent=2)+'\n').encode()
PINS={'Cargo.toml':'84b1986d8a2014be985460b3ecc4a809f3ae3e8f9822ef6c7134b3ecbe9af114',
 'Cargo.lock':'8ade63f6a4aae93f27f5161798bc87804542a48093b4203f35e25e29b680a4f5',
 'deny.toml':'16ee708c678497358f30d772cbb843832b4e02e8f3ae4355b648620a94c44930',
 'docs/testing/performance/0.74/qualification-manifest.toml':'11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'}

def need(ok,why):
    if not ok:raise ValueError(why)

def safe(name):
    p=PurePosixPath(name)
    need(str(p)==name and not p.is_absolute() and '..' not in p.parts and ':' not in name and '\\' not in name,'contained exact data name')
    return p

def walk(root):
    for base,dirs,files in os.walk(root,followlinks=False):
        for name in dirs+files:
            p=Path(base)/name;need(not p.is_symlink() and not p.is_junction(),'regular source/evidence data')
        dirs[:]=[n for n in sorted(dirs) if n!='__pycache__']
        for n in sorted(files):yield Path(base)/n

def source_files(body):
    result={}
    with tarfile.open(fileobj=io.BytesIO(gzip.decompress(body)),mode='r:') as archive:
        for member in archive:
            p=safe(member.name);need(p.parts[0]=='hc074-copied-root' and len(p.parts)>1 and member.isfile(),'regular source tar member')
            name='/'.join(p.parts[1:]);need(name not in result,'no duplicate source member')
            result[name]=archive.extractfile(member).read()
    return result

def collect():
    data={}
    for base,prefix in ((FIRST,'first'),(SECOND,'second'),(THIRD,'third')):
        for p in walk(base):
            rel=p.relative_to(base).as_posix()
            if rel.startswith('input/archives/') or rel=='input/source.tar':continue
            need(p.suffix.lower() in ('.json','.log','.py','.md','.toml','.rs','.crate','.lock') or p.name=='.gitattributes','declared source/data extension')
            data[prefix+'/'+rel]=p.read_bytes()
    data['first/input/source.tar.gz']=gzip.compress((FIRST/'input/source.tar').read_bytes(),mtime=0)
    for folder in ('chitchat-copied-root-074','chitchat-copied-root-evidence-074','chitchat-copied-root-caller-074'):
        for p in walk(ROOT/'tools'/folder):data['source/current/'+p.relative_to(ROOT).as_posix()]=p.read_bytes()
    for name,digest in PINS.items():
        b=(ROOT/name).read_bytes();need(sha(b)==digest,'unchanged actual Root admission input');data['reference/'+name]=b
    return data

def phase(data,prefix):
    get=lambda n:json.loads(data[prefix+'/'+n])
    result=get('result.json');owner=get('execution.json');intent=get('intent.json')
    need(intent['bound']==1200 and intent['session_bound']==1100 and intent['floor']==4*1024**3 and intent['poll']==intent['cleanup']==5 and intent['retry'] is False,'unchanged outer admission')
    need(owner['retired'],'retired outer owner')
    if result['complete']:
        need(owner['exit']==0 and not owner['errors'] and get('session-result.json')['complete'],'complete original inner and outer outcome')
    else:need(result.get('first_refusal') and (owner['exit']!=0 or owner['errors'] or not get('session-result.json')['complete']),'truthful first refusal')
    commands=[]
    for name,body in sorted(data.items()):
        tail=name.removeprefix(prefix+'/')
        if name.startswith(prefix+'/') and tail.count('/')==1 and tail.endswith('/execution.json'):
            own=json.loads(body);label=tail.split('/')[0];command=get(label+'/intent.json')
            bound=10 if label=='rustc-version' else 780
            need(own['retired'] and own['bound_seconds']==command['bound']==bound and own['capacity_poll_seconds']==own['cleanup_seconds']==5 and command['retry'] is False,'retired original command and exact bounds')
            commands.append(dict(label=label,exit=own['exit'],errors=own['errors']))
            if result['complete']:need(own['exit']==0 and not own['errors'],'complete first actual command')
    return dict(complete=result['complete'],classification=owner['classification'],commands=commands)

def reconcile(data):
    get=lambda n:json.loads(data[n])
    first=get('first/native/stage/execution.json')
    need(first['classification']=='resource-floor' and first['exit']==125 and first['retired'],'exact first resource-floor refusal')
    need('first/native/stage/session-result.json' not in data and 'first/native/stage/init-B/intent.json' not in data,'no first inner result or invoked Git/Cargo')
    need(not get('first/native/stage/result.json')['complete'] and get('first/STOP.json')['phase']=='stage','first context remains terminal')
    old_source={n.removeprefix('first/first-source/'):sha(b) for n,b in data.items() if n.startswith('first/first-source/')}
    need(len(old_source)==7,'exact original runner source retained')
    recorded=get('first/native/stage/intent.json')['source']
    for n,h in old_source.items():need(recorded['tools/chitchat-copied-root-074/'+n]==h,'whole first dispatched producer source')
    source=source_files(data['first/input/source.tar.gz']);source_map=get('first/input/source-map.json')
    need({n:sha(b) for n,b in source.items()}==source_map,'whole original frozen Root source')
    for n,h in PINS.items():need(sha(data['reference/'+n])==h and sha(source[n])==h,'same actual Root and source snapshot')
    controls={}
    for n in ('controls','syntax','prepare'):
        p='second/offline/'+n;value=get(p+'/result.json');owner=get(p+'/execution.json')
        need(value['complete'] and owner['exit']==0 and owner['retired'] and not owner['errors'],'complete separate offline prerequisite');controls[n]=True
    need(b'Ran 16 tests' in data['second/offline/controls/stderr.log'],'whole pure control inventory')
    phases={}
    for name in sorted(data):
        if re.fullmatch(r'(second|third)/native/[^/]+/result\.json',name):
            context,_,label,_=name.split('/');phases[context+'/'+label]=phase(data,name.rsplit('/',1)[0])
    need(phases,'at least one actual capacity-context outcome')
    for name,row in phases.items():
        context,label=name.split('/')
        if not row['complete']:need(get(context+'/STOP.json')['phase']==label,'first unexpected refusal latches exact phase')
    for n in ('controls','syntax','prepare'):
        value=get('third/offline/'+n+'/result.json');owner=get('third/offline/'+n+'/execution.json')
        need(value['complete'] and owner['exit']==0 and owner['retired'] and not owner['errors'],'complete changed-stage pure prerequisite')
    need(b'Ran 17 tests' in data['third/offline/controls/stderr.log'],'whole changed-stage control catalog')
    removed=0
    for suffix in ('','-second','-third'):
        review=get('first/cleanup-reviewed'+suffix+'.json');result=get('first/cleanup-result'+suffix+'.json')
        need(len(review['caches'])==len(result['deleted']),'whole exact cleanup set')
        for a,b in zip(review['caches'],result['deleted'],strict=True):need(b['absent'] and a['bytes']==b['bytes'] and b['path'].replace('\\','/').endswith('/'+a['path']),'exact completed generated cache cleanup');removed+=b['bytes']
    return dict(first_context='resource-floor-before-Git-or-Cargo',second_context='resource-floor-after-seven-source-only-Git-commands',pure_controls=17,previous_pure_controls=16,source_files=len(source),
        read_only_official_archives=len(get('first/input/registry.json')),offline=controls,phases=phases,
        generated_cache_bytes_removed=removed,actual_Root_changed=False,root_adopted=False,
        public_registry_qualified=False,release_admitted=False,server_launched=False,performance_claim=False)

def validate(index,read):
    need(index['schema']=='hc074-chitchat-copied-root-v1','exact packet schema');data={}
    for name,row in index['members'].items():
        safe(name);need(re.fullmatch('[0-9a-f]{64}',row['sha256']) and type(row['bytes']) is int and row['bytes']>=0,'exact object descriptor')
        b=read(row['sha256']);need(sha(b)==row['sha256'] and len(b)==row['bytes'],'whole exact evidence object');data[name]=b
    need(reconcile(data)==index['result'],'independent copied-Root accounting')
    return data

def retain():
    data=collect();result=reconcile(data);index=dict(schema='hc074-chitchat-copied-root-v1',result=result,
        ordinary_archives_excluded=True,compiled_images_excluded=True,members={n:dict(sha256=sha(b),bytes=len(b)) for n,b in sorted(data.items())})
    objects={sha(b):b for b in data.values()};validate(index,objects.__getitem__)
    OUT.mkdir(exist_ok=False);(OUT/'objects').mkdir()
    for h,b in objects.items():(OUT/'objects'/h).write_bytes(b)
    raw=encoded(index);(OUT/'index.json').write_bytes(raw)
    (OUT/'.gitattributes').write_bytes(b'objects/* -text whitespace=cr-at-eol,-blank-at-eol,-blank-at-eof\nindex.json -text\n')
    print(json.dumps(dict(sha256=sha(raw),members=len(data),objects=len(objects),result=result)))

def verify(digest):
    raw=(OUT/'index.json').read_bytes();need(sha(raw)==digest,'external whole packet digest');index=json.loads(raw)
    data=validate(index,lambda h:(OUT/'objects'/h).read_bytes());print(json.dumps(dict(complete=True,result=index['result'])))
    return data

def restore(digest,destination):
    data=verify(digest);path=Path(destination).resolve();need(path.is_relative_to((ROOT/'target').resolve()) and not path.exists(),'new contained source-only destination')
    source=source_files(data['first/input/source.tar.gz']);proposal=json.loads(data['first/input/proposal.json']);expected={}
    for v in ('B','O'):
        files=dict(source)
        if v=='O':files['Cargo.toml']=files['Cargo.toml'].replace(proposal['exact_before'].encode(),proposal['exact_after'].encode()[:-2]+b', registry = "hc074-draft" }')
        lock=data.get('third/state/lock-'+v+'.toml')
        if lock:files['Cargo.lock']=lock
        for n,b in files.items():
            safe(n);target=path/v/n;target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(b);expected[v+'/'+n]=sha(b)
    need({p.relative_to(path).as_posix():sha(p.read_bytes()) for p in walk(path)}==expected,'complete exact source-only restoration')
    print(json.dumps(dict(source_files=len(expected),archived_code_executed=False,build_executed=False)))

if __name__=='__main__':
    action=sys.argv[1]
    if action=='retain':retain()
    elif action=='verify':verify(sys.argv[2])
    else:need(action=='restore' and len(sys.argv)==4,'fixed data-only action');restore(sys.argv[2],sys.argv[3])
