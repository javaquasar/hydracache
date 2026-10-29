"""Seal the first maintenance/fault/recovery history without build caches."""
import json
import os
from pathlib import Path
import subprocess
import analyze as a

HERE=Path(__file__).resolve().parent
OUT=a.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-cohort-20261029'


def main():
    result=a.reconcile();a.need(result==json.loads(a.local(a.R1+'/analysis/result.json')),'independent first-result accounting')
    for mode in ('controls-final','syntax-final'):
        a.need(json.loads(a.local(a.R1+'/final/'+mode+'/result.json'))['complete'],'final offline source check')
        a.owner(json.loads(a.local(a.R1+'/final/'+mode+'/execution.json')))
    OUT.mkdir(exist_ok=False);(OUT/'objects').mkdir();members={}
    def add(name,body):
        digest=a.sha(body);path=OUT/'objects'/digest
        if path.exists():a.need(path.read_bytes()==body,'object collision')
        else:path.write_bytes(body)
        a.need(name not in members,'unique logical member');members[name]=dict(sha256=digest,bytes=len(body))
    for context in (a.R0,a.R1):
        base=a.ROOT/'target'/context
        for folder,dirs,names in os.walk(base,followlinks=False):
            folder=Path(folder)
            for name in dirs:a.need(not (folder/name).is_symlink() and not getattr(folder/name,'is_junction',lambda:False)(),'regular capture directory')
            if folder==base/'final':dirs[:]=[n for n in dirs if n not in ('doc-check','byte-verify','performance-contract')]
            for name in sorted(names):
                if name in ('payload.tar.gz','collect.stdout.log'):continue
                path=folder/name;a.need(path.is_file() and not path.is_symlink(),'regular saved capture')
                add('capture/'+context+'/'+path.relative_to(base).as_posix(),path.read_bytes())
        head=json.loads((base/'stage-once.json').read_bytes())['head']
        names=subprocess.check_output(['git','ls-tree','--name-only',head,'tools/chitchat-cohort-074/'],cwd=a.ROOT,text=True).splitlines()
        for name in names:add('original-producer/'+context+'/'+name,subprocess.check_output(['git','show',head+':'+name],cwd=a.ROOT))
    # The worker-smoke producer was committed after staging the immutable r1.
    head='becf0d616'
    for name in ('worker_control.py','client.py'):
        add('original-worker-control/'+name,subprocess.check_output(['git','show',head+':tools/chitchat-cohort-074/'+name],cwd=a.ROOT))
    for path in sorted(HERE.iterdir()):
        if path.is_file():add('source/tools/chitchat-cohort-074/'+path.name,path.read_bytes())
    index=dict(schema='hc074-chitchat-cohort-terminal-v1',members=members,predecessor_index_sha256=a.PARENT_SHA,
        source_head_observation=subprocess.check_output(['git','rev-parse','HEAD'],cwd=a.ROOT,text=True).strip(),
        original_plan_sha256=a.PLAN_SHA,terminal_result=result,compiled_images_excluded=True,
        vendor_copies_excluded=True,original_source_registry_graph_retained_in_predecessor=True,archived_code_executed=False)
    (OUT/'index.json').write_text(json.dumps(index,indent=2,sort_keys=True)+'\n',encoding='utf8',newline='\n')
    (OUT/'.gitattributes').write_text('objects/* -text whitespace=cr-at-eol,-blank-at-eol,-blank-at-eof\nindex.json -text\n',encoding='utf8',newline='\n')
    for row in members.values():
        raw=(OUT/'objects'/row['sha256']).read_bytes();a.need(a.sha(raw)==row['sha256'] and len(raw)==row['bytes'],'stored evidence body')
    print(json.dumps(dict(members=len(members),objects=len(list((OUT/'objects').iterdir())),index_sha256=a.sha((OUT/'index.json').read_bytes()))))


if __name__=='__main__':main()
