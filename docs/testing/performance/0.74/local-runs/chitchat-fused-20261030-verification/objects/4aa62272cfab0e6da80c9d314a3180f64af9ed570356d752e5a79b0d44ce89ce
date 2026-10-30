"""Content-addressed declined experiment, with externally supplied index digest."""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import review as r

ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).resolve().parent
BASE=ROOT/'target/chitchat-fused-20261030'
PACKET=ROOT/'docs/testing/performance/0.74/local-runs/chitchat-fused-20261030'
PARENT=ROOT/'docs/testing/performance/0.74/local-runs/chitchat-call-context-20261030'
PARENT_SHA='37c6f98061bb495f9cb058e40c0c9400b03ce1af69fd48cec8b5906c97533ab3'

def collect():
    data={}
    for prefix,folder in [('capture',BASE),('first-preparation',ROOT/'target/chitchat-fused-20261030-prepare-r1')]:
        for path in sorted(folder.rglob('*')):
            relative=path.relative_to(folder)
            if 'builds' in relative.parts or '__pycache__' in relative.parts:continue
            r.need(not path.is_symlink() and not getattr(path,'is_junction',lambda:False)(),'regular historical data')
            if path.is_file():
                r.need(path.suffix in ('.json','.log','.toml','.rs','.py','.md','.c','.h','.txt') or path.name in ('Cargo.lock','Cargo.toml.orig','.gitattributes','LICENSE','README','README.md','NOTICE'),'known data-only member: '+str(relative))
                data[prefix+'/'+relative.as_posix()]=path.read_bytes()
    for folder in (ROOT/'tools/chitchat-fused-074',HERE):
        for path in sorted(folder.iterdir()):
            if path.is_file():data['source/current/'+path.relative_to(ROOT).as_posix()]=path.read_bytes()
    raw=(PARENT/'index.json').read_bytes();r.need(r.sha(raw)==PARENT_SHA,'whole immutable C2 parent')
    parent=json.loads(raw)
    for row in parent['members'].values():
        body=(PARENT/'objects'/row['sha256']).read_bytes();r.need(r.sha(body)==row['sha256'] and len(body)==row['bytes'],'whole parent object')
    row=parent['members']['compat-B0/stdout.log'];data['reference/compat-B0/stdout.log']=(PARENT/'objects'/row['sha256']).read_bytes()
    for name,path in [('Cargo.lock',ROOT/'Cargo.lock'),('Cargo.toml',ROOT/'Cargo.toml'),('deny.toml',ROOT/'deny.toml'),
      ('qualification-manifest.toml',ROOT/'docs/testing/performance/0.74/qualification-manifest.toml')]:data['reference/root/'+name]=path.read_bytes()
    return data

def validate(index,read):
    r.need(index['schema']=='hc074-chitchat-fused-declined-v1' and index['parent_sha256']==PARENT_SHA,'closed archive/schema/parent')
    r.need(index['decision']=='declined-first-retained-output-regression','negative decision preserved')
    r.need(all(index[k] is False for k in ('root_adopted','release_admitted','CPU_qualified','RSS_qualified','further_measurements_allowed')),'no promotion')
    data={}
    for name,row in index['members'].items():
        path=PurePosixPath(name)
        r.need(not path.is_absolute() and '..' not in path.parts and '.' not in path.parts and '\\' not in name and ':' not in name,'contained archive path')
        r.need(re.fullmatch('[0-9a-f]{64}',row['sha256']) and type(row['bytes']) is int and row['bytes']>=0,'whole object descriptor')
        body=read(row['sha256']);r.need(r.sha(body)==row['sha256'] and len(body)==row['bytes'],'whole exact object: '+name);data[name]=body
    r.need(not any(n.startswith('capture/native-r2/') or n.startswith('capture/native-r1/observe-') for n in data),'no hidden native observations after first refusal')
    result=r.reconcile(data.__getitem__);r.need(result==index['review'],'independent complete semantic/resource review')
    return result

def retain():
    data=collect();result=r.reconcile(data.__getitem__)
    index=dict(schema='hc074-chitchat-fused-declined-v1',parent_sha256=PARENT_SHA,decision=result['decision'],
      root_adopted=False,release_admitted=False,CPU_qualified=False,RSS_qualified=False,further_measurements_allowed=False,
      review=result,members={n:dict(sha256=r.sha(raw),bytes=len(raw)) for n,raw in sorted(data.items())})
    objects={r.sha(raw):raw for raw in data.values()};validate(index,objects.__getitem__)
    PACKET.mkdir(exist_ok=False);(PACKET/'objects').mkdir()
    for digest,body in objects.items():(PACKET/'objects'/digest).write_bytes(body)
    (PACKET/'.gitattributes').write_text('* -text\n',encoding='utf8')
    raw=(json.dumps(index,indent=2,sort_keys=True)+'\n').encode();(PACKET/'index.json').write_bytes(raw)
    print(json.dumps(dict(sha256=r.sha(raw),members=len(data),objects=len(objects),review=result)))

def verify(path,digest):
    raw=(path/'index.json').read_bytes();r.need(r.sha(raw)==digest,'externally held full index digest')
    index=json.loads(raw);result=validate(index,lambda h:(path/'objects'/h).read_bytes());print(json.dumps(result));return index

def restore(path,digest,destination):
    index=verify(path,digest);root=(ROOT/'target').resolve();destination=destination.resolve()
    r.need(destination.is_relative_to(root) and not destination.exists(),'new source-only workspace destination')
    destination.mkdir()
    for name,row in index['members'].items():
        if not name.startswith('capture/fixtures/F/'):continue
        relative=PurePosixPath(name.removeprefix('capture/fixtures/F/'));target=destination/str(relative)
        target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes((path/'objects'/row['sha256']).read_bytes())
    r.need((destination/'src/serialize.rs').is_file() and (destination/'Cargo.lock').is_file(),'complete F source materialization')
    print(json.dumps(dict(source_only=str(destination),producer_executed=False,build_executed=False)))

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('action',choices=('retain','verify','restore'))
    parser.add_argument('--packet',type=Path,default=PACKET);parser.add_argument('--sha');parser.add_argument('--destination',type=Path)
    args=parser.parse_args()
    if args.action=='retain':retain()
    elif args.action=='verify':r.need(bool(args.sha),'explicit digest required');verify(args.packet,args.sha)
    else:r.need(bool(args.sha) and args.destination is not None,'explicit digest/destination required');restore(args.packet,args.sha,args.destination)
