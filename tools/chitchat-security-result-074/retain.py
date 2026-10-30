"""Keep all contexts and refusals, complete code/data, no environment secrets."""
import json
import os
from pathlib import Path
import analyze as a
c=a.c
OUT=c.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-security-only-20261030'

def main():
    result=a.live();OUT.mkdir(exist_ok=False);(OUT/'objects').mkdir();members={}
    def add(name,body):
        c.need(name not in members,'unique logical evidence member');digest=c.sha(body);p=OUT/'objects'/digest
        if p.exists():c.need(p.read_bytes()==body,'identical content object')
        else:p.write_bytes(body)
        members[name]=dict(sha256=digest,bytes=len(body))
    for folder,dirs,names in os.walk(c.CAPTURE,followlinks=False):
        folder=Path(folder)
        dirs[:]=[n for n in dirs if n not in ('builds','__pycache__','.git')]
        for n in dirs:c.need(not (folder/n).is_symlink() and not getattr(folder/n,'is_junction',lambda:False)(),'regular captured directory')
        for n in sorted(names):
            p=folder/n;c.need(p.is_file() and not p.is_symlink(),'regular capture')
            add('capture/'+p.relative_to(c.CAPTURE).as_posix(),p.read_bytes())
    add('root/Cargo.lock',(c.ROOT/'Cargo.lock').read_bytes())
    add('root/deny.toml',(c.ROOT/'deny.toml').read_bytes())
    for tool in ('chitchat-security-only-074','chitchat-security-result-074'):
        for p in sorted((c.ROOT/'tools'/tool).iterdir()):
            if p.is_file():add('source/tools/'+tool+'/'+p.name,p.read_bytes())
    add('analysis.json',(json.dumps(result,indent=2,sort_keys=True)+'\n').encode())
    c.write(OUT,'index.json',dict(schema='hc074-chitchat-dependency-only-packet-v1',members=members,result=result,
      source_head_observation=c.git('rev-parse','HEAD').decode().strip(),compiled_images_excluded=True,
      native_cache_copies_excluded=True,archived_code_executed=False,root_adopted=False,release_admitted=False))
    (OUT/'.gitattributes').write_text('objects/* -text whitespace=cr-at-eol,-blank-at-eol,-blank-at-eof\nindex.json -text\n',encoding='utf8',newline='\n')
    print(json.dumps(dict(index_sha256=c.sha((OUT/'index.json').read_bytes()),members=len(members),objects=len(list((OUT/'objects').iterdir())))))

if __name__=='__main__':main()
