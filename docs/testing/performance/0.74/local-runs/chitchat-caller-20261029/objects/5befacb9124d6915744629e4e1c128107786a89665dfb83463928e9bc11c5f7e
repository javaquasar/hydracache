"""Seal the actual-caller result and all premeasurement first refusals."""
import json
import subprocess
import common as c
import analyze as a

OUT=c.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-caller-20261029'


def main():
    result=a.reconcile(a.evidence());c.need(result==json.loads((c.BASE/'analysis/result.json').read_bytes()),'independent raw result')
    for variant in ('B','P','Q'):
        for label in ('build','control','check','clippy'):
            c.need(json.loads((c.BASE/f'{label}-{variant}/result.json').read_bytes())['complete'],'first complete native phase')
    for mode in ('controls','format','performance-contract'):
        c.need(json.loads((c.BASE/'final'/mode/'result.json').read_bytes())['complete'],'current final check')
    OUT.mkdir(exist_ok=False);(OUT/'objects').mkdir();members={}
    def add(name,path):
        body=path.read_bytes();digest=c.sha(body);dest=OUT/'objects'/digest
        if dest.exists():c.need(dest.read_bytes()==body,'object collision')
        else:dest.write_bytes(body)
        c.need(name not in members,'unique logical name');members[name]=dict(sha256=digest,bytes=len(body))
    for context in ('chitchat-caller-20261029','chitchat-caller-20261029-r1','chitchat-caller-20261029-r2'):
        for path in sorted((c.ROOT/'target'/context).rglob('*')):
            c.need(not path.is_symlink() and not getattr(path,'is_junction',lambda:False)(),'owned regular capture')
            if path.is_file():add('capture/'+context+'/'+path.relative_to(c.ROOT/'target'/context).as_posix(),path)
    for name in c.source():add('source/'+name,c.ROOT/name)
    index=dict(schema='hc074-chitchat-caller-v1',members=members,
        predecessor_index_sha256='0b195601989e9109725812ef5f8da76b3b2c9b4b42aed57e8e7992b6a25f35fa',
        source_head_observation=subprocess.check_output(['git','rev-parse','HEAD'],cwd=c.ROOT).decode().strip(),
        table_sha256=c.TABLE_SHA,fixed_caller_resource_gate_pass=result['fixed_caller_resource_gate_pass'],
        generic_component_gate_pass=False,CPU_qualified=False,RSS_qualified=False,MSRV_qualified=False,Windows_qualified=False,
        root_adopted=False,release_admitted=False,compiled_images_excluded=True,original_zip_inputs_retained=True,
        actual_link_objects_retained=True,archived_code_executed=False)
    (OUT/'index.json').write_text(json.dumps(index,indent=2,sort_keys=True)+'\n',encoding='utf8',newline='\n')
    (OUT/'.gitattributes').write_text('objects/* -text whitespace=cr-at-eol,-blank-at-eol,-blank-at-eof\nindex.json -text\n',encoding='utf8',newline='\n')
    for row in members.values():
        body=(OUT/'objects'/row['sha256']).read_bytes();c.need(c.sha(body)==row['sha256'] and len(body)==row['bytes'],'stored byte verification')
    print(json.dumps(dict(members=len(members),objects=len(list((OUT/'objects').iterdir())),index_sha256=c.sha((OUT/'index.json').read_bytes()))))


if __name__=='__main__':main()
