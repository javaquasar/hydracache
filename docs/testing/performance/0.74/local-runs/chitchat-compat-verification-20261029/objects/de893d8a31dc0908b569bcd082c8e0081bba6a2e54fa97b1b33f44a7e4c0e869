"""Seal the first component result without promoting it to a caller verdict."""
from pathlib import Path
import json
import subprocess
import common as c
import analyze as a

OUT=c.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-stateful-20261029'


def main():
    result=a.reconcile(a.evidence());c.need(result==json.loads((c.BASE/'analysis/result.json').read_bytes()),'raw independent result')
    for variant in ('B','P','Q'):
        for label in ('build','control','check','clippy'):
            c.need(json.loads((c.BASE/f'{label}-{variant}/result.json').read_bytes())['complete'],'completed first phase')
    for mode in ('controls','format','performance-contract'):
        c.need(json.loads((c.BASE/'final'/mode/'result.json').read_bytes())['complete'],'final current-source check')
    OUT.mkdir(exist_ok=False);(OUT/'objects').mkdir();members={}
    def add(name,path):
        body=path.read_bytes();digest=c.sha(body);dest=OUT/'objects'/digest
        if dest.exists():c.need(dest.read_bytes()==body,'object collision')
        else:dest.write_bytes(body)
        c.need(name not in members,'unique logical name');members[name]=dict(sha256=digest,bytes=len(body))
    for path in sorted(c.BASE.rglob('*')):
        c.need(not path.is_symlink() and not getattr(path,'is_junction',lambda:False)(),'owned regular capture')
        if path.is_file():add('capture/'+path.relative_to(c.BASE).as_posix(),path)
    for name in c.source():add('source/'+name,c.ROOT/name)
    index=dict(schema='hc074-chitchat-stateful-v1',members=members,
        predecessor_index_sha256='853f8691d7e9e1f0d2c1143908fddaefdb36e0fd24a507cbf7ac4424a3c61a71',
        source_head_observation=subprocess.check_output(['git','rev-parse','HEAD'],cwd=c.ROOT).decode().strip(),
        component_resource_gate_pass=result['component_resource_gate_pass'],integration_decision=result['integration_decision'],
        fixed_MTU_caller_qualified=False,production_regression_proved=False,root_adopted=False,release_admitted=False,
        archived_code_executed=False,compiled_images_excluded=True,actual_link_object_retained=True,original_zip_inputs_retained=True)
    (OUT/'index.json').write_text(json.dumps(index,indent=2,sort_keys=True)+'\n',encoding='utf8',newline='\n')
    (OUT/'.gitattributes').write_text('objects/* -text whitespace=cr-at-eol,-blank-at-eol,-blank-at-eof\nindex.json -text\n',encoding='utf8',newline='\n')
    for row in members.values():
        body=(OUT/'objects'/row['sha256']).read_bytes();c.need(c.sha(body)==row['sha256'] and len(body)==row['bytes'],'stored bytes')
    print(json.dumps(dict(members=len(members),objects=len(list((OUT/'objects').iterdir())),index_sha256=c.sha((OUT/'index.json').read_bytes()))))


if __name__=='__main__':main()
