"""Retain immutable first contexts and qualified counter evidence as bytes."""
from pathlib import Path
import json
import subprocess

import common as c
import analyze as a

OUT=c.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-native-cost-20261029'


def main():
    values=[]
    for index in range(1,5):
        folder=c.BASE/f'cohort-{index}'
        c.need(json.loads((folder/'result.json').read_bytes())['complete'],'complete original cohort')
        actual=c.diagnostic((folder/'native/observation/stdout.log').read_text(),'HC074_NATIVE_ALLOC:')
        c.need(actual==json.loads((folder/'native/observations.json').read_bytes()),'raw observations equal saved values');values.append(actual)
    result=a.reconcile(values);c.need(result==json.loads((c.BASE/'analysis/result.json').read_bytes()),'independent current parser reconciliation')
    for variant in ('B','P'):
        for label in ('build','control','check','clippy'):
            c.need(json.loads((c.BASE/f'{label}-{variant}/result.json').read_bytes())['complete'],'focused native gate')
    for mode in ('controls','format','performance-contract'):
        c.need(json.loads((c.BASE/f'final/{mode}/result.json').read_bytes())['complete'],'current-source final check')
    OUT.mkdir(exist_ok=False);(OUT/'objects').mkdir();members={}
    def add(name,path):
        body=path.read_bytes();digest=c.sha(body);dest=OUT/'objects'/digest
        if dest.exists():c.need(dest.read_bytes()==body,'object collision')
        else:dest.write_bytes(body)
        c.need(name not in members,'duplicate logical member');members[name]=dict(sha256=digest,bytes=len(body))
    contexts=('chitchat-native-cost-20261029','chitchat-native-cost-target-20261029',
              'chitchat-native-cost-scope-20261029','chitchat-native-cost-opaque-20261029')
    for context in contexts:
        folder=c.ROOT/'target'/context
        for path in sorted(folder.rglob('*')):
            if path.is_file():add('capture/'+context+'/'+path.relative_to(folder).as_posix(),path)
    for name in c.source():add('source/'+name,c.ROOT/name)
    add('source/target/chitchat-native-inventory-20261029.py',c.ROOT/'target/chitchat-native-inventory-20261029.py')
    add('source/target/inspect-native-tag-20261029.py',c.ROOT/'target/inspect-native-tag-20261029.py')
    index=dict(schema='hc074-chitchat-native-cost-v1',members=members,
        source_head_observation=subprocess.check_output(['git','rev-parse','HEAD'],cwd=c.ROOT).decode().strip(),
        predecessor_index_sha256='9381608b814a7fcdc16835ece436534fc1d48f816039023f9c0adbc16a234fd1',
        original_cases=239,process_order=['B','P','P','B'],native_C_regressions=9,
        integration_decision=result['integration_decision'],CPU_qualified=False,RSS_qualified=False,
        retained_memory_qualified=False,root_adopted=False,release_admitted=False,
        compiled_profile_binaries_excluded=True,actual_counter_link_objects_retained=True,original_zip_inputs_retained=True,
        byte_verifier_executes_archived_code=False)
    (OUT/'index.json').write_text(json.dumps(index,indent=2,sort_keys=True)+'\n',encoding='utf8',newline='\n')
    (OUT/'.gitattributes').write_text('objects/* -text whitespace=cr-at-eol,-blank-at-eol,-blank-at-eof\nindex.json -text\n',encoding='utf8',newline='\n')
    for row in members.values():
        body=(OUT/'objects'/row['sha256']).read_bytes();c.need(c.sha(body)==row['sha256'] and len(body)==row['bytes'],'stored byte verification')
    print(json.dumps(dict(members=len(members),objects=len(list((OUT/'objects').iterdir())),index_sha256=c.sha((OUT/'index.json').read_bytes()))))


if __name__=='__main__':main()
