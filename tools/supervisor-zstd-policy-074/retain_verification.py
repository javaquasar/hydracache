"""Seal final offline checks separately; never rewrite the primary result."""
import json
from pathlib import Path
import sys
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'supervisor-zstd-security-074'))
import common as c
import gates
import verify

def main():
    primary=verify.OUT;index=json.loads((primary/'index.json').read_bytes())
    verify.validate(index,lambda h:(primary/'objects'/h).read_bytes())
    for mode in gates.COMMANDS:
        c.need(json.loads((c.BASE/'final'/mode/'result.json').read_bytes())['complete'],'every final fixed check: '+mode)
    out=primary.with_name(primary.name+'-verification');out.mkdir(exist_ok=False);(out/'objects').mkdir();members={}
    def add(name,raw):
        digest=c.sha(raw);p=out/'objects'/digest
        if p.exists():c.need(p.read_bytes()==raw,'identical content object')
        else:p.write_bytes(raw)
        c.need(name not in members,'unique final member');members[name]=dict(sha256=digest,bytes=len(raw))
    for p in (c.BASE/'final').rglob('*'):
        c.need(not p.is_symlink(),'regular final evidence')
        if p.is_file():add('capture/'+p.relative_to(c.BASE/'final').as_posix(),p.read_bytes())
    for n in gates.DOCS:add('documents/'+n,(c.ROOT/n).read_bytes())
    for p in Path(__file__).parent.iterdir():
        if p.is_file():add('source/'+p.name,p.read_bytes())
    c.write(out,'index.json',dict(schema='hc074-direct-supervisor-zstd-verification-v1',members=members,
      primary_index_sha256=c.sha((primary/'index.json').read_bytes()),archived_code_executed=False,
      root_security_passed=False,workspace_gate_passed=False,release_admitted=False))
    (out/'.gitattributes').write_text('objects/* -text whitespace=cr-at-eol,-blank-at-eol,-blank-at-eof\nindex.json -text\n',encoding='utf8',newline='\n')
    for r in members.values():
        raw=(out/'objects'/r['sha256']).read_bytes();c.need(c.sha(raw)==r['sha256'] and len(raw)==r['bytes'],'exact final retained bytes')
    print(json.dumps(dict(index_sha256=c.sha((out/'index.json').read_bytes()),members=len(members),objects=len(list((out/'objects').iterdir())))))

if __name__=='__main__':main()
