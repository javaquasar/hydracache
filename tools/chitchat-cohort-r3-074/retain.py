"""Seal both incomplete carriers, refused preparations and the complete matrix."""
import json
import os
from pathlib import Path
import subprocess
import analyze as a

CONTEXTS=('chitchat-cohort-r2-20261029','chitchat-cohort-r3-20261030','chitchat-cohort-r3-20261030-r1','chitchat-cohort-r3-20261030-r2')
OUT=a.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-cohort-r3-20261030'

def main():
    result=a.reconcile();a.need(result==json.loads((a.BASE/'analysis/result.json').read_bytes()),'independent complete raw accounting')
    old=json.loads((a.ROOT/'target'/CONTEXTS[0]/'analysis/result.json').read_bytes())
    a.need(old['complete'] is False and old['manual_restoration_verified'] and not old['estimator_applied'],'r2 first incomplete result remains negative')
    for mode in ('controls-final','syntax-final'):
        a.need(json.loads((a.BASE/'final'/mode/'result.json').read_bytes())['complete'],'final owned check')
        a.owner(json.loads((a.BASE/'final'/mode/'execution.json').read_bytes()))
    OUT.mkdir();(OUT/'objects').mkdir();members={}
    def add(name,raw):
        digest=a.sha(raw);path=OUT/'objects'/digest
        if path.exists():a.need(path.read_bytes()==raw,'object collision')
        else:path.write_bytes(raw)
        a.need(name not in members,'unique member');members[name]=dict(sha256=digest,bytes=len(raw))
    for context in CONTEXTS:
        base=a.ROOT/'target'/context
        for folder,dirs,names in os.walk(base,followlinks=False):
            folder=Path(folder)
            for n in dirs:a.need(not (folder/n).is_symlink() and not getattr(folder/n,'is_junction',lambda:False)(),'regular capture directory')
            dirs[:]=[n for n in dirs if n!='tmp']
            if folder==base/'final':dirs[:]=[n for n in dirs if n not in ('byte-verify','doc-check','performance-contract')]
            for n in sorted(names):
                if n in ('payload.tar.gz','collect.stdout.log'):continue
                path=folder/n;a.need(path.is_file() and not path.is_symlink(),'regular saved capture')
                add('capture/'+context+'/'+path.relative_to(base).as_posix(),path.read_bytes())
        head=json.loads((base/'stage-once.json').read_bytes())['head'];tool='chitchat-cohort-r2-074' if context==CONTEXTS[0] else 'chitchat-cohort-r3-074'
        names=subprocess.check_output(['git','ls-tree','--name-only',head,'tools/'+tool+'/'],cwd=a.ROOT,text=True).splitlines()
        for n in names:add('original-producer/'+context+'/'+n,subprocess.check_output(['git','show',head+':'+n],cwd=a.ROOT))
    for tool in ('chitchat-cohort-r2-074','chitchat-cohort-r3-074'):
        for path in sorted((a.ROOT/'tools'/tool).iterdir()):
            if path.is_file():add('source/tools/'+tool+'/'+path.name,path.read_bytes())
    index=dict(schema='hc074-chitchat-complete-matrix-v1',members=members,predecessor_index_sha256=a.PREDECESSOR,
        original_plan_sha256=a.PLAN_SHA,source_head_observation=subprocess.check_output(['git','rev-parse','HEAD'],cwd=a.ROOT,text=True).strip(),
        result=result,incomplete_r2_result=old,contexts=list(CONTEXTS),compiled_images_excluded=True,vendor_copies_excluded=True,
        archived_code_executed=False,root_adopted=False,release_admitted=False)
    (OUT/'index.json').write_text(json.dumps(index,indent=2,sort_keys=True)+'\n',encoding='utf8',newline='\n')
    (OUT/'.gitattributes').write_text('objects/* -text whitespace=cr-at-eol,-blank-at-eol,-blank-at-eof\nindex.json -text\n',encoding='utf8',newline='\n')
    for row in members.values():
        raw=(OUT/'objects'/row['sha256']).read_bytes();a.need(a.sha(raw)==row['sha256'] and len(raw)==row['bytes'],'stored object bytes')
    print(json.dumps(dict(members=len(members),objects=len(list((OUT/'objects').iterdir())),index_sha256=a.sha((OUT/'index.json').read_bytes()))))

if __name__=='__main__':main()
