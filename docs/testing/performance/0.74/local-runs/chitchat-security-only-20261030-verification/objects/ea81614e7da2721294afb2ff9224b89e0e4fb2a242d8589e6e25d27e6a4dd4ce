"""Preserve post-capture replay/source/control/docs proofs independently."""
import json
from pathlib import Path
import gates
import verify
c=verify.c

def main():
    base=verify.OUT;index=json.loads((base/'index.json').read_bytes())
    verify.validate(index,lambda h:(base/'objects'/h).read_bytes())
    for mode in gates.COMMANDS:c.need(json.loads((c.CAPTURE/'final'/mode/'result.json').read_bytes())['complete'],'complete offline check: '+mode)
    out=base.with_name(base.name+'-verification');out.mkdir(exist_ok=False);(out/'objects').mkdir();members={}
    def add(name,raw):
        c.need(name not in members,'unique final evidence');digest=c.sha(raw);p=out/'objects'/digest
        if p.exists():c.need(p.read_bytes()==raw,'same content object')
        else:p.write_bytes(raw)
        members[name]=dict(sha256=digest,bytes=len(raw))
    for p in (c.CAPTURE/'final').rglob('*'):
        c.need(not p.is_symlink(),'regular final capture')
        if p.is_file():add('capture/'+p.relative_to(c.CAPTURE/'final').as_posix(),p.read_bytes())
    for name in gates.DOCS:add('documents/'+name,(c.ROOT/name).read_bytes())
    for tool in ('chitchat-security-only-074','chitchat-security-result-074'):
        for p in (c.ROOT/'tools'/tool).iterdir():
            if p.is_file():add('source/'+tool+'/'+p.name,p.read_bytes())
    c.write(out,'index.json',dict(schema='hc074-chitchat-dependency-only-verification-v1',members=members,
      primary_index_sha256=c.sha((base/'index.json').read_bytes()),archived_code_executed=False,root_adopted=False,wide_gate_passed=False,release_admitted=False))
    (out/'.gitattributes').write_text('objects/* -text whitespace=cr-at-eol,-blank-at-eol,-blank-at-eof\nindex.json -text\n',encoding='utf8',newline='\n')
    for row in members.values():
        raw=(out/'objects'/row['sha256']).read_bytes();c.need(len(raw)==row['bytes'] and c.sha(raw)==row['sha256'],'fresh final stored bytes')
    print(json.dumps(dict(index_sha256=c.sha((out/'index.json').read_bytes()),members=len(members),objects=len(list((out/'objects').iterdir())))))

if __name__=='__main__':main()
