"""Data-only child packet, explicitly retaining its negative R5 parent."""
import json,sys
from pathlib import Path,PurePosixPath
import review as r
s=r.s;m=r.m;c=r.c;p=r.parent
OUT=s.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-package-complete-20261030'
HERE=Path(__file__).resolve().parent
DIRS=('chitchat-package-complete-074','chitchat-package-complete-evidence-074')

def old_data():
    raw=(s.PACKET/'index.json').read_bytes();c.need(c.sha(raw)==s.SHA,'external whole R5 index digest')
    index=json.loads(raw)
    return p.validate(index,lambda h:(s.PACKET/'objects'/h).read_bytes())

def collect():
    data={}
    for prefix,root in [('capture',s.BASE),('producer-controls',s.ROOT/'target/chitchat-package-complete-controls-20261030')]:
        for path in p.walk(root):
            c.need(path.suffix.lower() in ('.json','.log','.toml','.rs','.py','.md','.txt','.patch','.crate','.tar','.lock')
                or path.name in ('Cargo.lock','Cargo.toml.orig','.gitattributes','LICENSE'),'data/source-only full-lane capture')
            data[prefix+'/'+path.relative_to(root).as_posix()]=path.read_bytes()
    for name in DIRS:
        for path in p.walk(s.ROOT/'tools'/name):data['source/current/'+path.relative_to(s.ROOT).as_posix()]=path.read_bytes()
    data['reference/parent-index.json']=(s.PACKET/'index.json').read_bytes()
    return data

def validate(index,read):
    c.need(index['schema']=='hc074-chitchat-package-complete-v1' and index['parent_sha256']==s.SHA,'whole separate full-lane identity')
    data={}
    for name,row in index['members'].items():
        path=PurePosixPath(name);c.need(str(path)==name and not path.is_absolute() and '..' not in path.parts and '\\' not in name and ':' not in name,'contained complete-lane member')
        c.need(p.re.fullmatch('[0-9a-f]{64}',row['sha256']) and type(row['bytes']) is int and row['bytes']>=0,'whole exact complete-lane descriptor')
        raw=read(row['sha256']);c.need(c.sha(raw)==row['sha256'] and len(raw)==row['bytes'],'whole exact complete-lane bytes');data[name]=raw
    c.need(c.sha(data['reference/parent-index.json'])==s.SHA,'retained complete immutable negative parent index')
    result=r.reconcile(lambda n:data['capture/'+n],old_data())
    c.need(result==index['result']==json.loads(data['capture/analysis/result.json']),'independent complete child outcomes')
    for mode in ('controls','syntax'):
        prefix='producer-controls/'+mode+'/';r.owned(lambda n:data[n],prefix)
        c.need(json.loads(data[prefix+'result.json'])['complete'],'actual original producer-control outcome')
    c.need(b'Ran 11 tests' in data['producer-controls/controls/stderr.log'] and data['producer-controls/controls/stderr.log'].rstrip().endswith(b'OK'),'all11 full-lane producer controls')
    return data

def analyze():
    result=r.reconcile(lambda n:(s.BASE/n).read_bytes(),old_data());folder=s.BASE/'analysis';folder.mkdir(exist_ok=True)
    c.write(folder,'result.json',result);print(json.dumps(result))

def retain():
    data=collect();result=r.reconcile(lambda n:data['capture/'+n],old_data())
    index=dict(schema='hc074-chitchat-package-complete-v1',parent_sha256=s.SHA,result=result,members={n:dict(sha256=c.sha(b),bytes=len(b)) for n,b in sorted(data.items())})
    objects={c.sha(b):b for b in data.values()};validate(index,objects.__getitem__)
    OUT.mkdir(exist_ok=False);(OUT/'objects').mkdir()
    for digest,raw in objects.items():(OUT/'objects'/digest).write_bytes(raw)
    raw=m.encoded(index);(OUT/'index.json').write_bytes(raw)
    (OUT/'.gitattributes').write_bytes(b'objects/* -text whitespace=cr-at-eol,-blank-at-eol,-blank-at-eof\nindex.json -text\n')
    print(json.dumps(dict(sha256=c.sha(raw),members=len(data),objects=len(objects),result=result)))

def verify(digest):
    raw=(OUT/'index.json').read_bytes();c.need(c.sha(raw)==digest,'external whole full-lane digest');index=json.loads(raw)
    data=validate(index,lambda h:(OUT/'objects'/h).read_bytes());print(json.dumps(dict(complete=True,members=len(data),result=index['result'])))
    return data

def restore(digest,destination):
    data=verify(digest);destination=Path(destination).resolve()
    c.need(destination.is_relative_to((s.ROOT/'target').resolve()) and not destination.exists(),'new contained source-only destination')
    files=m.codec().unpack(data['capture/artifact/'+m.PREFIX+'.crate'],m.PREFIX);destination.mkdir()
    for n,b in files.items():
        path=destination/n;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(b)
    c.need({f.relative_to(destination).as_posix():c.sha(f.read_bytes()) for f in p.walk(destination)}=={n:c.sha(b) for n,b in files.items()},'exact26-member restoration')
    print(json.dumps(dict(source_members=len(files),archived_code_executed=False,build_executed=False)))

if __name__=='__main__':
    if sys.argv[1]=='analyze':analyze()
    elif sys.argv[1]=='retain':retain()
    elif sys.argv[1]=='verify':verify(sys.argv[2])
    else:c.need(sys.argv[1]=='restore' and len(sys.argv)==4,'fixed restore/digest/path');restore(sys.argv[2],sys.argv[3])
