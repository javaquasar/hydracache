"""Retain final offline gates and documentation without changing primary data."""
import ast,copy,json,sys
from pathlib import Path,PurePosixPath
ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(ROOT/'tools/chitchat-frame-evidence-074'))
import packet as a
import gates as g
c=g.c
SHA='41a5a9ef83a599c6d2cbcd519dd5070f8527b7994c04618741e06b0038fbcb09'
OUT=a.PACKET.with_name(a.PACKET.name+'-verification')
MODES=('controls','syntax','format','byte-verify','restore-verify','doc-check','performance-contract')

def collect():
    data={}
    for path in (g.BASE/'final').rglob('*'):
        c.need(not path.is_symlink() and not getattr(path,'is_junction',lambda:False)(),'regular final data')
        if path.is_file():data['final/'+path.relative_to(g.BASE/'final').as_posix()]=path.read_bytes()
    for n in g.DOCS:data['docs/'+n]=(ROOT/n).read_bytes()
    for name in (*g.DIRS,HERE.name):
        for path in (ROOT/'tools'/name).iterdir():
            if path.is_file():data['source/current/'+path.relative_to(ROOT).as_posix()]=path.read_bytes()
    return data

def check(data):
    for mode in MODES:
        get=lambda name:json.loads(data['final/'+mode+'/'+name])
        result=get('result.json');owner=get('execution.json');intent=get('intent.json')
        c.need(result['complete'] and all(result[k] is False for k in ('root_adopted','release_admitted','product_execution')),'closed offline scope')
        c.need(owner['exit']==0 and owner['retired'] and owner['errors']==[],'complete exact offline retirement')
        c.need(intent['argv']==g.commands(SHA)[mode] and intent['bound']==owner['bound_seconds']==1200 and intent['floor']==4*1024**3,'exact offline command/bounds')
        c.need(owner['capacity_poll_seconds']==owner['cleanup_seconds']==5,'same poll/retirement')
        if mode=='controls':c.need(b'Ran 24 tests' in data['final/controls/stderr.log'] and data['final/controls/stderr.log'].rstrip().endswith(b'OK'),'all24 controls')
    c.need(b'OK (local, non-promotable)' in data['final/performance-contract/stdout.log'],'no ship-gate promotion')
    for n in g.DIRS:
        for path in (ROOT/'tools'/n).glob('*.py'):ast.parse(data['source/current/'+path.relative_to(ROOT).as_posix()],filename=str(path))
    ast.parse(data['source/current/'+(HERE/'retain.py').relative_to(ROOT).as_posix()])
    return dict(complete=True,primary_sha256=SHA,decision='declined-first-full-resource-gate',gates=list(MODES),offline_controls=24,
        root_adopted=False,release_admitted=False,CPU_qualified=False,RSS_qualified=False,product_execution=False)

def controls(data):
    mutations=(('controls','execution.json','retired',False),('format','execution.json','bound_seconds',1201),
      ('syntax','intent.json','argv',['wrong-command']),('doc-check','result.json','release_admitted',True),('byte-verify','result.json','complete',False))
    for mode,name,key,value in mutations:
        modified=dict(data);path='final/'+mode+'/'+name;row=json.loads(modified[path]);row[key]=value;modified[path]=json.dumps(row).encode()
        try:check(modified)
        except ValueError:pass
        else:raise ValueError('accepted mutated offline receipt')
    return dict(complete=True,mutation_controls=5,product_execution=False)

def retain():
    index=a.verify(SHA);restored=g.BASE/'source-only-review'
    expected={n.removeprefix('capture/fixtures/G/'):r['sha256'] for n,r in index['members'].items() if n.startswith('capture/fixtures/G/')}
    actual={p.relative_to(restored).as_posix():c.sha(p.read_bytes()) for p in restored.rglob('*') if p.is_file()}
    c.need(actual==expected,'exact complete source-only export, no build')
    data=collect();summary=check(data);proof=controls(data)
    OUT.mkdir(exist_ok=False);(OUT/'objects').mkdir()
    objects={c.sha(raw):raw for raw in data.values()}
    for digest,raw in objects.items():(OUT/'objects'/digest).write_bytes(raw)
    result=dict(schema='hc074-chitchat-frame-offline-v1',summary=summary,controls=proof,
        members={n:dict(sha256=c.sha(raw),bytes=len(raw)) for n,raw in sorted(data.items())})
    raw=(json.dumps(result,indent=2,sort_keys=True)+'\n').encode();(OUT/'index.json').write_bytes(raw)
    (OUT/'.gitattributes').write_bytes(b'objects/* -text whitespace=cr-at-eol,-blank-at-eol,-blank-at-eof\nindex.json -text\n')
    print(json.dumps(dict(sha256=c.sha(raw),members=len(data),objects=len(objects),summary=summary,controls=proof)))

def verify(digest):
    raw=(OUT/'index.json').read_bytes();c.need(c.sha(raw)==digest,'external whole final index');index=json.loads(raw)
    c.need(index['schema']=='hc074-chitchat-frame-offline-v1','exact final schema');data={}
    for name,row in index['members'].items():
        path=PurePosixPath(name);c.need(not path.is_absolute() and '..' not in path.parts and '\\' not in name and ':' not in name,'contained final member')
        raw=(OUT/'objects'/row['sha256']).read_bytes();c.need(c.sha(raw)==row['sha256'] and len(raw)==row['bytes'],'exact whole final object');data[name]=raw
    c.need(check(data)==index['summary'] and controls(data)==index['controls'],'independent final outcomes');print(json.dumps(index['summary']))

if __name__=='__main__':
    if sys.argv[1]=='retain':retain()
    else:c.need(sys.argv[1]=='verify' and len(sys.argv)==3,'fixed verify/digest');verify(sys.argv[2])
