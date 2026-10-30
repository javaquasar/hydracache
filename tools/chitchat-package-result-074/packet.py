"""Data-only package evidence; never executes an archived producer or image."""
import json, os, re, sys
from pathlib import Path, PurePosixPath
import review as r
m=r.m;ROOT=m.ROOT
OUT=ROOT/'docs/testing/performance/0.74/local-runs/chitchat-package-20261030'
DIRS=('chitchat-package-074','chitchat-package-r2-074','chitchat-package-r3-074',
      'chitchat-package-r4-074','chitchat-package-r5-074','chitchat-package-policy-074',
      'chitchat-package-policy-final-074','chitchat-package-result-074')
CONTEXTS=('chitchat-package-20261030','chitchat-package-20261030-r2',
          'chitchat-package-20261030-r3','chitchat-package-20261030-r4',
          'chitchat-package-20261030-r4-prepare-r1','chitchat-package-20261030-r4-prepare-r2',
          'chitchat-package-20261030-r5')
SKIP={'builds','vendor','cargo-home','advisory-dbs','__pycache__','.git','final','final-r2','source-only-review'}

def walk(root):
    m.need(root.is_dir() and not root.is_symlink() and not root.is_junction(),'regular existing evidence root')
    for base,dirs,files in os.walk(root,followlinks=False):
        for n in dirs+files:
            p=Path(base)/n
            m.need(not p.is_symlink() and not p.is_junction(),'regular contained evidence')
        dirs[:]=sorted(n for n in dirs if n not in SKIP)
        for n in sorted(files):yield Path(base)/n

def collect():
    data={}
    for context in CONTEXTS:
        root=ROOT/'target'/context
        for p in walk(root):
            m.need(p.suffix.lower() in ('.json','.log','.toml','.rs','.py','.md','.txt','.patch','.crate','.tar','.lock')
                   or p.name in ('Cargo.lock','Cargo.toml.orig','.cargo_vcs_info.json','.gitattributes','LICENSE'),
                   'data/source-only evidence member: '+str(p))
            data['capture/'+context+'/'+p.relative_to(root).as_posix()]=p.read_bytes()
    for name in DIRS:
        for p in walk(ROOT/'tools'/name):data['source/current/'+p.relative_to(ROOT).as_posix()]=p.read_bytes()
    for name in ('Cargo.toml','Cargo.lock','deny.toml','docs/testing/performance/0.74/qualification-manifest.toml'):
        data['reference/root/'+name]=(ROOT/name).read_bytes()
    # Retain exactly the checked-out RustSec files, independently rechecked by
    # both complete policy command receipts. No Git hooks/config or DB execution.
    sys.path.insert(0,str(ROOT/'tools/chitchat-package-policy-final-074'))
    import policy
    db=policy.database()
    for name,digest in db['files'].items():
        raw=(policy.DB/name).read_bytes();m.need(m.sha(raw)==digest,'whole checked policy cut')
        data['reference/advisory-db/'+name]=raw
    return data

def validate(index,read):
    m.need(index['schema']=='hc074-chitchat-package-v1' and index['parent_sha256']==m.PARENT_SHA,'whole package evidence identity')
    data={}
    for name,row in index['members'].items():
        p=PurePosixPath(name)
        m.need(name and str(p)==name and not p.is_absolute() and '..' not in p.parts and '\\' not in name and ':' not in name,'contained evidence member')
        m.need(re.fullmatch('[0-9a-f]{64}',row['sha256']) and type(row['bytes']) is int and row['bytes']>=0,'whole exact evidence descriptor')
        raw=read(row['sha256']);m.need(m.sha(raw)==row['sha256'] and len(raw)==row['bytes'],'whole exact evidence bytes')
        data[name]=raw
    prefix='capture/'+m.BASE.name+'/'
    result=r.reconcile(lambda n:data[prefix+n])
    m.need(result==index['result']==json.loads(data[prefix+'analysis/result.json']),'independent complete package reconciliation')
    phases={n.removeprefix(prefix+'native/').split('/')[0] for n in data if n.startswith(prefix+'native/')}
    m.need(phases=={'stage','resolve','package','build194','build188','pure194','rest194'},'no unrecorded continuation beyond the first STOP')
    # All earlier outcomes stay distinct; no successful continuation rewrites a
    # timeout, resolver/lint, producer or capacity refusal into a successful run.
    first='capture/chitchat-package-20261030/'
    m.need(not json.loads(data[first+'native/stage/result.json'])['complete'] and first+'STOP.json' in data,'first preparation timeout remains')
    for context,phase in [('chitchat-package-20261030-r2','resolve'),('chitchat-package-20261030-r3','build194'),('chitchat-package-20261030-r4','stage')]:
        base='capture/'+context+'/'
        m.need(not json.loads(data[base+'native/'+phase+'/result.json'])['complete'] and base+'STOP.json' in data,'first separate refusal remains: '+context)
    for name in ('first-stage-refusal.json','first-format-refusal.json'):
        m.need(any(n.endswith('/'+name) for n in data),'first source producer result retained')
    for label in ('owned','upstream'):
        db=json.loads(data[prefix+'policy/commands/'+label+'/intent.json'])['database']
        actual={n.removeprefix('reference/advisory-db/'):m.sha(b) for n,b in data.items() if n.startswith('reference/advisory-db/')}
        m.need(db['commit']=='2295466c4e9df26647fd67d8fa1c705ac2fc7f81' and actual==db['files'],'whole retained same policy cut')
    for name,digest in [('Cargo.lock','8ade63f6a4aae93f27f5161798bc87804542a48093b4203f35e25e29b680a4f5'),
                        ('deny.toml','16ee708c678497358f30d772cbb843832b4e02e8f3ae4355b648620a94c44930'),
                        ('docs/testing/performance/0.74/qualification-manifest.toml','11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc')]:
        m.need(m.sha(data['reference/root/'+name])==digest,'unchanged exact Root input')
    return data

def retain():
    data=collect();result=r.reconcile(lambda n:data['capture/'+m.BASE.name+'/'+n])
    index=dict(schema='hc074-chitchat-package-v1',parent_sha256=m.PARENT_SHA,result=result,
        members={n:dict(sha256=m.sha(b),bytes=len(b)) for n,b in sorted(data.items())})
    objects={m.sha(b):b for b in data.values()};validate(index,objects.__getitem__)
    OUT.mkdir(exist_ok=False);(OUT/'objects').mkdir()
    for digest,raw in objects.items():(OUT/'objects'/digest).write_bytes(raw)
    raw=m.encoded(index);(OUT/'index.json').write_bytes(raw)
    (OUT/'.gitattributes').write_bytes(b'objects/* -text whitespace=cr-at-eol,-blank-at-eol,-blank-at-eof\nindex.json -text\n')
    print(json.dumps(dict(sha256=m.sha(raw),members=len(data),objects=len(objects),result=result)))

def verify(digest):
    raw=(OUT/'index.json').read_bytes();m.need(m.sha(raw)==digest,'external complete index digest')
    index=json.loads(raw);data=validate(index,lambda h:(OUT/'objects'/h).read_bytes())
    print(json.dumps(dict(complete=True,members=len(data),result=index['result'])))
    return data

def restore(digest,destination):
    data=verify(digest);destination=Path(destination).resolve()
    m.need(destination.is_relative_to((ROOT/'target').resolve()) and not destination.exists(),'new contained source-only destination')
    raw=data['capture/'+m.BASE.name+'/artifact/'+m.PREFIX+'.crate']
    files=m.codec().unpack(raw,m.PREFIX)
    destination.mkdir()
    for name,body in files.items():
        p=destination/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(body)
    actual={p.relative_to(destination).as_posix():m.sha(p.read_bytes()) for p in walk(destination)}
    m.need(actual=={n:m.sha(b) for n,b in files.items()},'whole exact source-only restoration')
    print(json.dumps(dict(source_members=len(files),archived_code_executed=False,build_executed=False)))

if __name__=='__main__':
    if sys.argv[1]=='retain':retain()
    elif sys.argv[1]=='verify':m.need(len(sys.argv)==3,'explicit digest');verify(sys.argv[2])
    else:m.need(sys.argv[1]=='restore' and len(sys.argv)==4,'fixed source-only restore');restore(sys.argv[2],sys.argv[3])
