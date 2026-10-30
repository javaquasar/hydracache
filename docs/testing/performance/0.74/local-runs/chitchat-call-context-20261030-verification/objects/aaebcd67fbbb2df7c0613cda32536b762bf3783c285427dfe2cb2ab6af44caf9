"""Reconcile the existing source-only export; no build or archive execution."""
import json
import verify as v

index=json.loads((v.OUT/'index.json').read_bytes())
v.validate(index,lambda h:(v.OUT/'objects'/h).read_bytes())
root=v.r.ROOT/'target/chitchat-call-source-review-20261030'
value=json.loads((root/'SOURCE_REVIEW_ONLY.json').read_bytes())
v.r.need(value['index_sha256']==v.r.hashlib.sha256((v.OUT/'index.json').read_bytes()).hexdigest(),'original verified source packet')
v.r.need(all(value[k] is False for k in ('root_adopted','release_admitted','archived_producer_executed','build_dispatched')),'source review only')
expected={n.removeprefix('fixtures/'):r for n,r in index['members'].items() if n.startswith(('fixtures/B0/','fixtures/T1/','fixtures/C2/'))}
v.r.need(value['members']==expected and len(expected)==78,'all complete original source members')
actual={}
for path in root.rglob('*'):
    v.r.need(not path.is_symlink() and not getattr(path,'is_junction',lambda:False)() and path.resolve().is_relative_to(root.resolve()),'regular contained exported source')
    if path.is_file():actual[path.relative_to(root).as_posix()]=path
v.r.need(set(actual)==set(expected)|{'SOURCE_REVIEW_ONLY.json'},'no missing or extra export')
for name,row in expected.items():
    raw=actual[name].read_bytes()
    v.r.need(len(raw)==row['bytes'] and v.r.hashlib.sha256(raw).hexdigest()==row['sha256'],'fresh whole exported bytes')
print(json.dumps(dict(complete=True,members=78,archived_producer_executed=False)))
