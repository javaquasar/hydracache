"""Content-addressed sources and first outcomes, excluding all build caches."""
import json
from pathlib import Path
import analyze as a
OUT=a.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-context-20261030'


def main():
    out=OUT;out.mkdir(exist_ok=False);(out/'objects').mkdir();members={}
    def add(name,raw):
        a.need(name not in members,'unique retained logical member');digest=a.hashlib.sha256(raw).hexdigest();path=out/'objects'/digest
        if path.exists():a.need(path.read_bytes()==raw,'same stored object')
        else:path.write_bytes(raw)
        members[name]=dict(sha256=digest,bytes=len(raw))
    for p in a.BASE.rglob('*'):
        relative=p.relative_to(a.BASE)
        if any(n in ('builds','__pycache__','.git') for n in relative.parts):continue
        a.need(not p.is_symlink() and not getattr(p,'is_junction',lambda:False)(),'regular own capture')
        if p.is_file():add(relative.as_posix(),p.read_bytes())
    for folder,prefix in [('chitchat-borrowed-074','source/current'),('chitchat-context-result-074','source/analysis')]:
        for p in (a.ROOT/'tools'/folder).iterdir():
            if p.is_file():add(prefix+'/'+p.name,p.read_bytes())
    for n in ('wrap.c','wrap.h','control.c'):add('source/native/'+n,(a.ROOT/'tools/chitchat-native-cost-074'/n).read_bytes())
    for n in ('Cargo.lock','deny.toml','docs/testing/performance/0.74/qualification-manifest.toml',
              'tools/dbus-root-package-074/source_guard.py','scripts/ci/watchdog_windows_job.py','scripts/ci/watchdog_linux_group.py'):
        add('root/'+n,(a.ROOT/n).read_bytes())
    parent=a.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-security-only-20261030/index.json'
    raw=parent.read_bytes();a.need(a.hashlib.sha256(raw).hexdigest()=='e6dd71ebb6818b13587b4750360ade76db5484e5083021ca6505699c039373b3','unchanged dependency-only parent')
    result=a.analyze(lambda n:(out/'objects'/members[n]['sha256']).read_bytes())
    a.need(result==json.loads((a.BASE/'analysis-r5-final/result.json').read_bytes()),'independent complete stored replay')
    index=dict(schema='hc074-chitchat-context-prerequisite-v1',members=members,parent_index_sha256=a.hashlib.sha256(raw).hexdigest(),
      archived_code_executed=False,root_adopted=False,release_admitted=False,mtu_repaired=False,
      prerequisite_passed=result['prerequisite_passed'])
    (out/'index.json').write_text(json.dumps(index,indent=2,sort_keys=True)+'\n',encoding='utf8',newline='\n')
    (out/'.gitattributes').write_text('objects/* -text whitespace=cr-at-eol,-blank-at-eol,-blank-at-eof\nindex.json -text\n',encoding='utf8',newline='\n')
    print(json.dumps(dict(index_sha256=a.hashlib.sha256((out/'index.json').read_bytes()).hexdigest(),members=len(members),objects=len(list((out/'objects').iterdir())))))


if __name__=='__main__':main()
