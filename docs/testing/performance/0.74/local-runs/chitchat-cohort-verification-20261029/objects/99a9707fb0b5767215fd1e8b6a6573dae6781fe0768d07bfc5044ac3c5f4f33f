"""Seal final documentation/check receipts apart from the failed host cohort."""
import json
from pathlib import Path
import subprocess
import analyze as a
import verify as v

OUT=a.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-cohort-verification-20261029'
DOCS=('docs/articles/007-optimization-starts-with-measurement.md','docs/testing/performance/0.74/README.md',
    'docs/testing/performance/0.74/release-closure-execution-plan-20261026.md',
    'docs/testing/performance/0.74/chitchat-cohort-result-20261029.md',
    'docs/testing/performance/0.74/chitchat-cost-exclusive-host-plan-20261029.md',
    'docs/testing/performance/0.74/chitchat-host-cost-result-20261029.md')


def main():
    v.main();OUT.mkdir(exist_ok=False);(OUT/'objects').mkdir();members={}
    def add(name,path):
        raw=path.read_bytes();digest=a.sha(raw);dest=OUT/'objects'/digest
        if dest.exists():a.need(dest.read_bytes()==raw,'object collision')
        else:dest.write_bytes(raw)
        a.need(name not in members,'unique logical verification member');members[name]=dict(sha256=digest,bytes=len(raw))
    for mode in ('doc-check-sealed','byte-verify','performance-contract','controls-sealed','syntax-sealed'):
        folder=a.ROOT/'target'/a.R1/'final'/mode
        a.need(json.loads((folder/'result.json').read_bytes())['complete'],'complete final owned check')
        a.owner(json.loads((folder/'execution.json').read_bytes()))
        for path in sorted(folder.rglob('*')):
            if path.is_file():add('capture/'+mode+'/'+path.relative_to(folder).as_posix(),path)
    for name in DOCS:add('documents/'+name,a.ROOT/name)
    for path in sorted(Path(__file__).parent.iterdir()):
        if path.is_file():add('source/tools/chitchat-cohort-074/'+path.name,path)
    manifest=a.ROOT/'docs/testing/performance/0.74/qualification-manifest.toml'
    a.need(a.sha(manifest.read_bytes())=='11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc','unchanged qualification identity')
    add('documents/docs/testing/performance/0.74/qualification-manifest.toml',manifest)
    index=dict(schema='hc074-chitchat-cohort-verification-v1',members=members,predecessor_index_sha256=v.EXPECTED,
        source_head_observation=subprocess.check_output(['git','rev-parse','HEAD'],cwd=a.ROOT,text=True).strip(),
        terminal_accounting_complete=True,numerical_complete=False,automatic_postflight_pass=False,
        manual_supervisor_restored=True,CPU_qualified=False,RSS_qualified=False,archived_code_executed=False,
        root_adopted=False,release_admitted=False)
    (OUT/'index.json').write_text(json.dumps(index,indent=2,sort_keys=True)+'\n',encoding='utf8',newline='\n')
    (OUT/'.gitattributes').write_text('objects/* -text whitespace=cr-at-eol,-blank-at-eol,-blank-at-eof\nindex.json -text\n',encoding='utf8',newline='\n')
    for row in members.values():
        raw=(OUT/'objects'/row['sha256']).read_bytes();a.need(a.sha(raw)==row['sha256'] and len(raw)==row['bytes'],'sealed verification bytes')
    print(json.dumps(dict(members=len(members),objects=len(list((OUT/'objects').iterdir())),index_sha256=a.sha((OUT/'index.json').read_bytes()))))


if __name__=='__main__':main()
