"""Keep current docs/verifier/owned final checks separate from immutable primary."""
import json
from pathlib import Path
import gates
import verify as v
a=v.a


def main():
    raw=(v.OUT/'index.json').read_bytes()
    v.validate(json.loads(raw),lambda h:(v.OUT/'objects'/h).read_bytes())
    for mode in gates.COMMANDS:
        result=json.loads((a.BASE/'final'/mode/'result.json').read_bytes())
        a.need(result['complete'],'all final checks complete: '+mode)
    out=v.OUT.with_name(v.OUT.name+'-verification');out.mkdir(exist_ok=False);(out/'objects').mkdir();members={}
    def add(name,body):
        a.need(name not in members,'unique final member');digest=a.hashlib.sha256(body).hexdigest();path=out/'objects'/digest
        if path.exists():a.need(path.read_bytes()==body,'same stored object')
        else:path.write_bytes(body)
        members[name]=dict(sha256=digest,bytes=len(body))
    for path in (a.BASE/'final').rglob('*'):
        if path.is_file():add('capture/'+path.relative_to(a.BASE/'final').as_posix(),path.read_bytes())
    for folder in (gates.HERE,gates.p.HERE):
        for path in folder.iterdir():
            if path.is_file():add('source/'+folder.name+'/'+path.name,path.read_bytes())
    for name in gates.DOCS:add('documents/'+name,(a.ROOT/name).read_bytes())
    index=dict(schema='hc074-chitchat-context-verification-v1',members=members,primary_index_sha256=a.hashlib.sha256(raw).hexdigest(),
        archived_code_executed=False,root_adopted=False,release_admitted=False,prerequisite_passed=False)
    (out/'index.json').write_text(json.dumps(index,indent=2,sort_keys=True)+'\n',encoding='utf8',newline='\n')
    (out/'.gitattributes').write_text('objects/* -text whitespace=cr-at-eol,-blank-at-eol,-blank-at-eof\nindex.json -text\n',encoding='utf8',newline='\n')
    for row in members.values():
        body=(out/'objects'/row['sha256']).read_bytes();a.need(len(body)==row['bytes'] and a.hashlib.sha256(body).hexdigest()==row['sha256'],'whole final stored bytes')
    print(json.dumps(dict(index_sha256=a.hashlib.sha256((out/'index.json').read_bytes()).hexdigest(),members=len(members),objects=len(list((out/'objects').iterdir())))))


if __name__=='__main__':main()
