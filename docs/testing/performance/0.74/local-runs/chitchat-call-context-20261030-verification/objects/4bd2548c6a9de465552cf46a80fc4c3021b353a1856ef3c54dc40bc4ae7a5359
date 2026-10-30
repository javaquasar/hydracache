"""Retain all C2 sources/first outcomes; exclude build caches and images."""
import json
from pathlib import Path
import finalize as f
r=f.r
OUT=r.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-call-context-20261030'


def main():
    OUT.mkdir(exist_ok=False);(OUT/'objects').mkdir();members={}
    def add(name,raw):
        r.need(name not in members,'unique logical evidence');digest=r.sha(raw);path=OUT/'objects'/digest
        if path.exists():r.need(path.read_bytes()==raw,'same stored object')
        else:path.write_bytes(raw)
        members[name]=dict(sha256=digest,bytes=len(raw))
    for path in r.BASE.rglob('*'):
        relative=path.relative_to(r.BASE)
        if any(n in ('builds','__pycache__','.git') for n in relative.parts):continue
        r.need(not path.is_symlink() and not getattr(path,'is_junction',lambda:False)(),'regular own capture')
        if path.is_file():add(relative.as_posix(),path.read_bytes())
    for folder,prefix in [('chitchat-call-context-074','source/current'),('chitchat-call-result-074','source/review')]:
        for path in (r.ROOT/'tools'/folder).iterdir():
            if path.is_file():add(prefix+'/'+path.name,path.read_bytes())
    for name in ('parents/c1-index.json','parents/c1-analysis.json','parents/c1-catalog.json','parents/c1-wrapper.c'):
        add(name,r.live_read(name))
    for name in ('Cargo.lock','deny.toml','docs/testing/performance/0.74/qualification-manifest.toml',
       'scripts/ci/watchdog_windows_job.py','scripts/ci/watchdog_linux_group.py','tools/dbus-root-package-074/source_guard.py'):
        add('root/'+name,(r.ROOT/name).read_bytes())
    def read(name):return (OUT/'objects'/members[name]['sha256']).read_bytes()
    result=f.full(read);r.need(result==json.loads((r.BASE/'final-analysis/result.json').read_bytes()),'complete independent stored replay')
    index=dict(schema='hc074-chitchat-call-context-v1',members=members,parent_index_sha256=r.PARENT_SHA,
       archived_producer_executed=False,prerequisite_passed=result['prerequisite_passed'],
       root_adopted=False,release_admitted=False,mtu_repaired=False,full_resource_contract_passed=False)
    (OUT/'index.json').write_text(json.dumps(index,indent=2,sort_keys=True)+'\n',encoding='utf8',newline='\n')
    (OUT/'.gitattributes').write_text('objects/* -text whitespace=cr-at-eol,-blank-at-eol,-blank-at-eof\nindex.json -text\n',encoding='utf8',newline='\n')
    print(json.dumps(dict(index_sha256=r.sha((OUT/'index.json').read_bytes()),members=len(members),objects=len(list((OUT/'objects').iterdir())))))


if __name__=='__main__':main()
