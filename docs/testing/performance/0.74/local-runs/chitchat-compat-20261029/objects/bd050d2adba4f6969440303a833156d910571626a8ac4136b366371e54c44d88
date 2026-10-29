"""Seal first runner refusals, baseline, candidate and raw observations; no replay."""
from pathlib import Path
import json
import os
import subprocess
os.environ['HC074_COMMIT_PROFILE']='Q'
import common as c
import analyze as a

OUT=c.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-commit-20261029'


def main():
    result=a.reconcile(a.evidence())
    c.need(result==json.loads((c.BASE/'analysis/result.json').read_bytes()),'complete independent raw reconciliation')
    for label in ('focused-Q','build-B','build-Q','control-B','control-Q','check-B','check-Q','clippy-B','clippy-Q'):
        c.need(json.loads((c.BASE/label/'result.json').read_bytes())['complete'],'completed first phase '+label)
    for mode in ('controls','format','performance-contract'):
        c.need(json.loads((c.BASE/'final'/mode/'result.json').read_bytes())['complete'],'current source final gate '+mode)
    OUT.mkdir(exist_ok=False);(OUT/'objects').mkdir();members={}
    def add(name,path):
        body=path.read_bytes();digest=c.sha(body);dest=OUT/'objects'/digest
        if dest.exists():c.need(dest.read_bytes()==body,'object collision')
        else:dest.write_bytes(body)
        c.need(name not in members,'unique logical member');members[name]=dict(sha256=digest,bytes=len(body))
    contexts=('chitchat-commit-P-20261029','chitchat-commit-P-20261029-r1','chitchat-commit-P-20261029-r2','chitchat-commit-Q-20261029-r2')
    for context in contexts:
        for path in sorted((c.ROOT/'target'/context).rglob('*')):
            c.need(not path.is_symlink() and not getattr(path,'is_junction',lambda:False)(),'owned regular capture')
            if path.is_file():add('capture/'+context+'/'+path.relative_to(c.ROOT/'target'/context).as_posix(),path)
    for name in c.source():add('source/'+name,c.ROOT/name)
    index=dict(schema='hc074-chitchat-commit-v1',members=members,
        source_head_observation=subprocess.check_output(['git','rev-parse','HEAD'],cwd=c.ROOT).decode().strip(),
        predecessor_index_sha256='3550fa5293b3f61f3b6d06662e3090b17658a42c1ca87c97b32f0c20e6d6c489',
        process_order=['B','Q','Q','B'],cases=239,allocation_dominance_pass=result['allocation_dominance_pass'],
        integration_decision=result['integration_decision'],CPU_qualified=False,RSS_qualified=False,
        retained_memory_qualified=False,MSRV_qualified=False,Windows_qualified=False,root_adopted=False,release_admitted=False,
        compiled_test_images_excluded=True,actual_counter_link_objects_retained=True,original_zip_inputs_retained=True,
        first_runner_refusals_retained=True,byte_verifier_executes_archived_code=False)
    (OUT/'index.json').write_text(json.dumps(index,indent=2,sort_keys=True)+'\n',encoding='utf8',newline='\n')
    (OUT/'.gitattributes').write_text('objects/* -text whitespace=cr-at-eol,-blank-at-eol,-blank-at-eof\nindex.json -text\n',encoding='utf8',newline='\n')
    for row in members.values():
        body=(OUT/'objects'/row['sha256']).read_bytes();c.need(c.sha(body)==row['sha256'] and len(body)==row['bytes'],'sealed bytes')
    print(json.dumps(dict(members=len(members),objects=len(list((OUT/'objects').iterdir())),index_sha256=c.sha((OUT/'index.json').read_bytes()))))


if __name__=='__main__':main()
