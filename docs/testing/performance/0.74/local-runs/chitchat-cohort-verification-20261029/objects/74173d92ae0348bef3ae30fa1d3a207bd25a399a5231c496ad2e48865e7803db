"""Independent byte-only terminal replay; archived producers never execute."""
import json
import re
import analyze as a

EXPECTED='5e4b0f0282d3cd7e329a25f2abe236201d023d6e3a682a79915bcbd9abdde1fa'
PACKET=a.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-cohort-20261029'


def main():
    raw=(PACKET/'index.json').read_bytes();a.need(a.sha(raw)==EXPECTED,'exact terminal primary index')
    index=json.loads(raw);members=index['members'];seen=set()
    a.need(index['schema']=='hc074-chitchat-cohort-terminal-v1' and index['original_plan_sha256']==a.PLAN_SHA,'terminal schema/external plan')
    for name,row in members.items():
        a.need(not name.startswith('/') and '..' not in name.split('/') and re.fullmatch('[0-9a-f]{64}',row['sha256']) and type(row['bytes']) is int and row['bytes']>=0,'contained object reference')
        path=PACKET/'objects'/row['sha256'];a.need(path.is_file() and not path.is_symlink(),'regular stored object')
        raw=path.read_bytes();a.need(a.sha(raw)==row['sha256'] and len(raw)==row['bytes'],'exact sealed bytes');seen.add(row['sha256'])
        a.need(not name.endswith(('/payload.tar.gz','/collect.stdout.log')) and '/input/images/' not in name and '/input/vendor/' not in name,'no duplicated binary/registry/archive caches')
    a.need(seen=={p.name for p in (PACKET/'objects').iterdir()},'complete unique object inventory')
    def get(name):
        row=members[name];return (PACKET/'objects'/row['sha256']).read_bytes()
    result=a.reconcile(lambda name:get('capture/'+name))
    a.need(result==index['terminal_result']==json.loads(get('capture/'+a.R1+'/analysis/result.json')),'independent full raw terminal reconciliation')
    a.need(index['predecessor_index_sha256']==a.PARENT_SHA and index['archived_code_executed'] is False,'original source graph retained without archived execution')
    for context in (a.R0,a.R1):
        prepared=json.loads(get('capture/'+context+'/prepared.json'))
        for name,digest in prepared['files'].items():
            if name=='maintenance_base.py':
                a.need(a.sha(get('capture/'+context+'/payload/'+name))==digest=='fcb8a53eba5ebc7e1364cbfa0d2bcb9e0ad1098f75b4e53dae56b7503c02ed38','unchanged actual maintenance base')
            else:a.need(a.sha(get('original-producer/'+context+'/tools/chitchat-cohort-074/'+name))==digest,'exact original committed producer')
    for name in ('cohort-complete.json','restored.json','host-postflight.json'):
        a.need('capture/'+a.R1+'/collection/'+a.NUMERIC+'/'+name not in members,'absent successful automatic/numerical receipt remains absent')
    print(json.dumps(dict(complete=True,terminal_accounting_complete=True,members=len(members),objects=len(seen),index_sha256=EXPECTED,
        numerical_complete=False,automatic_postflight_pass=False,manual_supervisor_restored=True,CPU_qualified=False,RSS_qualified=False,archived_code_executed=False)))


if __name__=='__main__':main()
