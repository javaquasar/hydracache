"""Seal native controls and both carrier histories; exclude binary/vendor copies."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import analyze as a

ROOT=a.ROOT;HERE=Path(__file__).resolve().parent
OUT=ROOT/'docs/testing/performance/0.74/local-runs/chitchat-host-cost-20261029'


def sha(body):return hashlib.sha256(body).hexdigest()


def main():
    result=a.reconcile();a.need(result==json.loads((ROOT/'target'/a.R1/'analysis/result.json').read_bytes()),'independent raw result')
    for mode in ('controls-accounting','syntax-final'):
        a.need(json.loads((ROOT/'target'/a.R0/'final'/mode/'result.json').read_bytes())['complete'],'final source/accounting check')
    OUT.mkdir(exist_ok=False);(OUT/'objects').mkdir();members={}
    def add(name,body):
        digest=sha(body);dest=OUT/'objects'/digest
        if dest.exists():a.need(dest.read_bytes()==body,'object collision')
        else:dest.write_bytes(body)
        a.need(name not in members,'unique logical member');members[name]=dict(sha256=digest,bytes=len(body))
    for context in (a.R0,a.R1):
        base=ROOT/'target'/context
        for folder,dirs,names in os.walk(base,followlinks=False):
            folder=Path(folder)
            for name in dirs:a.need(not (folder/name).is_symlink() and not getattr(folder/name,'is_junction',lambda:False)(),'regular capture directory')
            if folder==base/'prepared/input':dirs[:]=[n for n in dirs if n not in ('images','vendor')]
            for name in sorted(names):
                if name in ('payload.tar.gz','patch.tar.gz'):continue
                path=folder/name;a.need(path.is_file() and not path.is_symlink(),'regular capture')
                add('capture/'+context+'/'+path.relative_to(base).as_posix(),path.read_bytes())
        frozen=json.loads((base/'frozen.json').read_bytes());head=json.loads((base/'dispatch-once.json').read_bytes())['head']
        for name,digest in frozen['producer'].items():
            raw=subprocess.check_output(['git','show',head+':tools/chitchat-host-cost-074/'+name],cwd=ROOT)
            a.need(sha(raw)==digest,'original committed producer bytes '+context+'/'+name)
            add('original-producer/'+context+'/'+name,raw)
    for path in sorted(HERE.iterdir()):
        if path.is_file():add('source/tools/chitchat-host-cost-074/'+path.name,path.read_bytes())
    index=dict(schema='hc074-chitchat-host-control-v1',members=members,
        predecessor_index_sha256='9589ebd9014f7b2be15aa8cfedec635e06d042a4b81fe92da3894fdea1008462',
        source_head_observation=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT).decode().strip(),
        host_control_gate_pass=True,controls=3,first_carrier_refusal_retained=True,first_WSL_refusal_unchanged=True,
        CPU_qualified=False,RSS_qualified=False,cohort_dispatched=False,exclusive_host=False,
        root_adopted=False,release_admitted=False,compiled_images_excluded=True,actual_registry_copies_excluded=True,
        original_zip_and_registry_archives_retained=True,archived_code_executed=False)
    (OUT/'index.json').write_text(json.dumps(index,indent=2,sort_keys=True)+'\n',encoding='utf8',newline='\n')
    (OUT/'.gitattributes').write_text('objects/* -text whitespace=cr-at-eol,-blank-at-eol,-blank-at-eof\nindex.json -text\n',encoding='utf8',newline='\n')
    for row in members.values():
        raw=(OUT/'objects'/row['sha256']).read_bytes();a.need(sha(raw)==row['sha256'] and len(raw)==row['bytes'],'stored bytes')
    print(json.dumps(dict(members=len(members),objects=len(list((OUT/'objects').iterdir())),index_sha256=sha((OUT/'index.json').read_bytes()))))


if __name__=='__main__':main()
