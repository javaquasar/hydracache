"""Immutable data-only identity evidence; source restoration never builds/runs."""
import json,os,re,sys,tomllib
from pathlib import Path,PurePosixPath
import accounting as a
s=a.s;c=a.c;m=a.m
HERE=Path(__file__).resolve().parent
OUT=s.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-public-identity-20261030'
DIRS=('chitchat-public-identity-074','chitchat-public-identity-evidence-074')

def walk(root):
    c.need(root.is_dir() and not root.is_symlink() and not root.is_junction(),'regular evidence root')
    for base,dirs,files in os.walk(root,followlinks=False):
        for name in dirs+files:
            p=Path(base)/name;c.need(not p.is_symlink() and not p.is_junction(),'regular contained evidence')
        dirs[:]=sorted(n for n in dirs if n not in ('__pycache__','builds','vendor','cargo-home','.git','final','source-only-review'))
        for name in sorted(files):yield Path(base)/name

def collect():
    data={}
    for path in walk(s.BASE):
        c.need(path.suffix.lower() in ('.json','.log','.toml','.rs','.py','.md','.txt','.lock','.crate','.body') or path.name in ('Cargo.lock','.gitattributes','LICENSE'),'data/source-only capture')
        data[path.relative_to(s.BASE).as_posix()]=path.read_bytes()
    for name in DIRS:
        for path in walk(s.ROOT/'tools'/name):data['source/current/'+path.relative_to(s.ROOT).as_posix()]=path.read_bytes()
    audit=json.loads(data['input/audit.json']);workspace=tomllib.loads((s.ROOT/'Cargo.toml').read_bytes().decode())['workspace']
    names={'Cargo.toml','Cargo.lock','deny.toml','docs/testing/performance/0.74/qualification-manifest.toml',*[n+'/Cargo.toml' for n in workspace['members']],*audit['references']}
    for name in names:data['reference/root/'+name]=(s.ROOT/name).read_bytes()
    data['reference/frozen-root-map.json']=(m.BASE/'input/frozen-root-map.json').read_bytes()
    data['reference/root-map-origin-index.json']=(s.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-package-20261030/index.json').read_bytes()
    for suffix in ('json','md'):
        data['proposal/root-change.'+suffix]=(s.ROOT/('docs/testing/performance/0.74/chitchat-public-identity-migration-proposal-20261030.'+suffix)).read_bytes()
    return data

def validate(index,read):
    c.need(index['schema']=='hc074-chitchat-public-identity-v1' and index['parent_sha256']==s.PARENT_SHA,'whole public-identity packet scope')
    data={}
    for name,row in index['members'].items():
        p=PurePosixPath(name);c.need(str(p)==name and not p.is_absolute() and '..' not in p.parts and ':' not in name and '\\' not in name,'contained identity evidence member')
        c.need(re.fullmatch('[0-9a-f]{64}',row['sha256']) and type(row['bytes']) is int and row['bytes']>=0,'whole exact object descriptor')
        raw=read(row['sha256']);c.need(c.sha(raw)==row['sha256'] and len(raw)==row['bytes'],'whole exact evidence bytes');data[name]=raw
    result=a.reconcile(data.__getitem__);c.need(result==index['result']==json.loads(data['analysis/result.json']),'independent whole nominal outcomes')
    for name,digest in c.PINS.items():
        if name in ('Cargo.lock','deny.toml','docs/testing/performance/0.74/qualification-manifest.toml'):c.need(c.sha(data['reference/root/'+name])==digest,'unchanged original Root/qualification')
    return data

def analyze():
    data=collect();result=a.reconcile(data.__getitem__);out=s.BASE/'analysis';out.mkdir(exist_ok=True);c.write(out,'result.json',result);print(json.dumps(result))

def retain():
    data=collect();result=a.reconcile(data.__getitem__);index=dict(schema='hc074-chitchat-public-identity-v1',parent_sha256=s.PARENT_SHA,
        result=result,members={n:dict(sha256=c.sha(b),bytes=len(b)) for n,b in sorted(data.items())})
    objects={c.sha(b):b for b in data.values()};validate(index,objects.__getitem__)
    OUT.mkdir(exist_ok=False);(OUT/'objects').mkdir()
    for digest,raw in objects.items():(OUT/'objects'/digest).write_bytes(raw)
    raw=m.encoded(index);(OUT/'index.json').write_bytes(raw)
    (OUT/'.gitattributes').write_bytes(b'objects/* -text whitespace=cr-at-eol,-blank-at-eol,-blank-at-eof\nindex.json -text\n')
    print(json.dumps(dict(sha256=c.sha(raw),members=len(data),objects=len(objects),result=result)))

def verify(digest):
    raw=(OUT/'index.json').read_bytes();c.need(c.sha(raw)==digest,'external whole identity digest');index=json.loads(raw)
    data=validate(index,lambda h:(OUT/'objects'/h).read_bytes());print(json.dumps(dict(complete=True,members=len(data),result=index['result'])))
    return data

def restore(digest,destination):
    data=verify(digest);destination=Path(destination).resolve()
    c.need(destination.is_relative_to((s.ROOT/'target').resolve()) and not destination.exists(),'new contained source-only destination')
    destination.mkdir();expected={}
    for variant in s.VARIANTS:
        for name,body in [('Cargo.toml',data['source/'+variant+'/Cargo.toml']),('src/lib.rs',data['source/'+variant+'/src/lib.rs']),('Cargo.lock',data['state/lock-'+variant+'.toml'])]:
            relative=variant+'/'+name;path=destination/relative;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(body);expected[relative]=c.sha(body)
    c.need({p.relative_to(destination).as_posix():c.sha(p.read_bytes()) for p in walk(destination)}==expected,'whole exact probe source restoration')
    print(json.dumps(dict(source_members=len(expected),archived_code_executed=False,build_executed=False)))

if __name__=='__main__':
    if sys.argv[1]=='analyze':analyze()
    elif sys.argv[1]=='retain':retain()
    elif sys.argv[1]=='verify':verify(sys.argv[2])
    else:c.need(sys.argv[1]=='restore' and len(sys.argv)==4,'fixed restore/digest/path');restore(sys.argv[2],sys.argv[3])
