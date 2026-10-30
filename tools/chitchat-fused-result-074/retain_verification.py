"""Retain/verify final offline checks; no product execution or promotion."""
import json
from pathlib import Path
import sys
import archive as a
import gates as g
r=a.r
OUT=a.PACKET.with_name(a.PACKET.name+'-verification')
MODES=('controls','syntax','format','byte-verify','restore-verify','doc-check','performance-contract','seal-controls','seal-syntax')

def data():
    bodies={}
    for folder,prefix in [(a.BASE/'final','final'),(a.BASE/'contexts/offline-final-r1','first-final-producer')]:
        for path in folder.rglob('*'):
            r.need(not path.is_symlink() and not getattr(path,'is_junction',lambda:False)(),'regular offline evidence')
            if path.is_file():bodies[prefix+'/'+path.relative_to(folder).as_posix()]=path.read_bytes()
    for folder in (a.ROOT/'tools/chitchat-fused-074',a.HERE):
        for path in folder.iterdir():
            if path.is_file():bodies['source/current/'+path.relative_to(a.ROOT).as_posix()]=path.read_bytes()
    for name in g.DOCS:bodies['docs/'+name]= (a.ROOT/name).read_bytes()
    return bodies

def check(bodies):
    for mode in MODES:
        read=lambda name:json.loads(bodies['final/'+mode+'/'+name])
        result=read('result.json');owner=read('execution.json');intent=read('intent.json')
        r.need(result['complete'] and all(result[k] is False for k in ('root_adopted','release_admitted','product_execution')),'complete offline result')
        r.need(owner['exit']==0 and owner['retired'] and owner['errors']==[],'retired original offline owner')
        r.need(intent['argv']==g.COMMANDS[mode] and intent['bound']==owner['bound_seconds']==1200 and intent['floor']==4*1024**3,'fixed original offline command/bounds')
        r.need(owner['capacity_poll_seconds']==owner['cleanup_seconds']==5,'same poll/retirement')
        if mode in ('controls','seal-controls'):r.need(b'Ran 13 tests' in bodies['final/'+mode+'/stderr.log'] and bodies['final/'+mode+'/stderr.log'].rstrip().endswith(b'OK'),'all thirteen offline controls')
    r.need(b'OK (local, non-promotable)' in bodies['final/performance-contract/stdout.log'],'nonship performance scope')
    return dict(complete=True,gates=list(MODES),offline_controls=13,product_execution=False,
      primary_sha256=g.SHA,decision='declined-first-retained-output-regression',root_adopted=False,release_admitted=False)

def retain():
    a.verify(a.PACKET,g.SHA)
    restored=a.ROOT/'target/chitchat-fused-source-review-20261030'
    index=json.loads((a.PACKET/'index.json').read_bytes())
    expected={n.removeprefix('capture/fixtures/F/'):row['sha256'] for n,row in index['members'].items() if n.startswith('capture/fixtures/F/')}
    actual={p.relative_to(restored).as_posix():r.sha(p.read_bytes()) for p in restored.rglob('*') if p.is_file()}
    r.need(actual==expected,'exact complete source-only export, no build output')
    bodies=data();summary=check(bodies);OUT.mkdir(exist_ok=False);(OUT/'objects').mkdir()
    objects={r.sha(raw):raw for raw in bodies.values()}
    for digest,raw in objects.items():(OUT/'objects'/digest).write_bytes(raw)
    result=dict(schema='hc074-chitchat-fused-offline-verification-v1',summary=summary,
      members={n:dict(sha256=r.sha(raw),bytes=len(raw)) for n,raw in sorted(bodies.items())})
    raw=(json.dumps(result,indent=2,sort_keys=True)+'\n').encode();(OUT/'index.json').write_bytes(raw)
    (OUT/'.gitattributes').write_text('* -text\n',encoding='utf8')
    print(json.dumps(dict(sha256=r.sha(raw),members=len(bodies),objects=len(objects),summary=summary)))

def verify(digest):
    raw=(OUT/'index.json').read_bytes();r.need(r.sha(raw)==digest,'external whole verification index')
    index=json.loads(raw);r.need(index['schema']=='hc074-chitchat-fused-offline-verification-v1','fixed verification schema');bodies={}
    for name,row in index['members'].items():
        raw=(OUT/'objects'/row['sha256']).read_bytes();r.need(r.sha(raw)==row['sha256'] and len(raw)==row['bytes'],'whole verification object');bodies[name]=raw
    r.need(check(bodies)==index['summary'],'whole offline outcome accounting');print(json.dumps(index['summary']))

if __name__=='__main__':
    if sys.argv[1]=='retain':retain()
    else:r.need(sys.argv[1]=='verify' and len(sys.argv)==3,'fixed verify/digest');verify(sys.argv[2])
