"""Separate immutable final proof, preserving the raw nominal-control packet."""
import json,sys
import gates as g
p=g.p;s=p.s;c=p.c;m=p.m
OUT=p.OUT.with_name(p.OUT.name+'-verification')

def review(data,digest):
    for mode in g.MODES:
        get=lambda n:json.loads(data['final/'+mode+'/'+n])
        result,owner,intent=(get(n) for n in ('result.json','execution.json','intent.json'))
        c.need(result['complete'] and all(result[k] is False for k in ('root_adopted','release_admitted','product_execution')),'complete exact offline scope')
        c.need(owner['exit']==0 and owner['retired'] and owner['errors']==[] and owner['bound_seconds']==intent['bound']==1200,'complete original offline owner')
        c.need(owner['capacity_poll_seconds']==owner['cleanup_seconds']==5 and intent['floor']==4*1024**3 and intent['retry'] is False and intent['argv']==g.commands(digest)[mode],'exact original offline commands/bounds')
    c.need(b'Ran 13 tests' in data['final/controls/stderr.log'] and data['final/controls/stderr.log'].rstrip().endswith(b'OK'),'all13 independent evidence controls')
    c.need(b'OK (local, non-promotable)' in data['final/performance-contract/stdout.log'],'no full qualification promotion')
    return dict(complete=True,primary_sha256=digest,gates=list(g.MODES),evidence_controls=13,root_adopted=False,release_admitted=False,product_execution=False)

def retain(digest):
    p.verify(digest);data={}
    for path in p.walk(g.FINAL):data['final/'+path.relative_to(g.FINAL).as_posix()]=path.read_bytes()
    for n in g.DOCS:data['docs/'+n]=(s.ROOT/n).read_bytes()
    for n in p.DIRS:
        for path in p.walk(s.ROOT/'tools'/n):data['source/current/'+path.relative_to(s.ROOT).as_posix()]=path.read_bytes()
    result=review(data,digest);objects={c.sha(b):b for b in data.values()};OUT.mkdir(exist_ok=False);(OUT/'objects').mkdir()
    for h,b in objects.items():(OUT/'objects'/h).write_bytes(b)
    index=dict(schema='hc074-chitchat-public-identity-offline-v1',result=result,members={n:dict(sha256=c.sha(b),bytes=len(b)) for n,b in sorted(data.items())})
    raw=m.encoded(index);(OUT/'index.json').write_bytes(raw)
    (OUT/'.gitattributes').write_bytes(b'objects/* -text whitespace=cr-at-eol,-blank-at-eol,-blank-at-eof\nindex.json -text\n')
    print(json.dumps(dict(sha256=c.sha(raw),members=len(data),objects=len(objects),result=result)))

def verify(digest):
    raw=(OUT/'index.json').read_bytes();c.need(c.sha(raw)==digest,'external complete verification digest');index=json.loads(raw);data={}
    c.need(index['schema']=='hc074-chitchat-public-identity-offline-v1','exact verification schema')
    for n,row in index['members'].items():
        path=p.PurePosixPath(n);c.need(str(path)==n and not path.is_absolute() and '..' not in path.parts and ':' not in n and '\\' not in n,'contained final member')
        c.need(p.re.fullmatch('[0-9a-f]{64}',row['sha256']) and type(row['bytes']) is int and row['bytes']>=0,'whole exact final descriptor')
        b=(OUT/'objects'/row['sha256']).read_bytes();c.need(c.sha(b)==row['sha256'] and len(b)==row['bytes'],'whole exact final object');data[n]=b
    c.need(review(data,index['result']['primary_sha256'])==index['result'],'independent final outcomes');print(json.dumps(index['result']))

if __name__=='__main__':
    if sys.argv[1]=='retain':retain(sys.argv[2])
    else:c.need(sys.argv[1]=='verify' and len(sys.argv)==3,'fixed verify');verify(sys.argv[2])
