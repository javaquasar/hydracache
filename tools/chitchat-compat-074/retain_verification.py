"""Retain final document/byte receipts apart from the immutable peer packet."""
import json
import subprocess
import common as c
import verify

OUT=c.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-compat-verification-20261029'
DOCS=('docs/articles/007-optimization-starts-with-measurement.md','docs/testing/performance/0.74/README.md',
    'docs/testing/performance/0.74/release-closure-execution-plan-20261026.md','docs/testing/performance/0.74/chitchat-compat-result-20261029.md')


def main():
    OUT.mkdir(exist_ok=False);(OUT/'objects').mkdir();members={}
    def add(name,path):
        body=path.read_bytes();digest=c.sha(body);dest=OUT/'objects'/digest
        if dest.exists():c.need(dest.read_bytes()==body,'object collision')
        else:dest.write_bytes(body)
        c.need(name not in members,'unique logical member');members[name]=dict(sha256=digest,bytes=len(body))
    for mode in ('doc-check','byte-verify'):
        folder=c.BASE/'final'/mode;c.need(json.loads((folder/'result.json').read_bytes())['complete'],'first complete final check')
        from analyze import owner
        owner(json.loads((folder/'execution.json').read_bytes()))
        for path in sorted(folder.rglob('*')):
            if path.is_file():add('capture/'+mode+'/'+path.relative_to(folder).as_posix(),path)
    for name in DOCS:add('documents/'+name,c.ROOT/name)
    for name in c.source():add('source/'+name,c.ROOT/name)
    index=dict(schema='hc074-chitchat-compat-verification-v1',members=members,predecessor_index_sha256=verify.EXPECTED,
        source_head_observation=subprocess.check_output(['git','rev-parse','HEAD'],cwd=c.ROOT).decode().strip(),
        archived_code_executed=False,root_adopted=False,release_admitted=False)
    (OUT/'index.json').write_text(json.dumps(index,indent=2,sort_keys=True)+'\n',encoding='utf8',newline='\n')
    (OUT/'.gitattributes').write_text('objects/* -text whitespace=cr-at-eol,-blank-at-eol,-blank-at-eof\nindex.json -text\n',encoding='utf8',newline='\n')
    for row in members.values():
        body=(OUT/'objects'/row['sha256']).read_bytes();c.need(c.sha(body)==row['sha256'] and len(body)==row['bytes'],'sealed verification bytes')
    print(json.dumps(dict(members=len(members),objects=len(list((OUT/'objects').iterdir())),index_sha256=c.sha((OUT/'index.json').read_bytes()))))


if __name__=='__main__':main()
