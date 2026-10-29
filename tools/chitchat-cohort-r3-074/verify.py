"""Independent byte-only complete-matrix replay; archived code never executes."""
import json
import re
import analyze as a

EXPECTED='1366650e2145b9a671bc505a243aefb32e545ee35157cdaf0a53f7e71379ee08'
PACKET=a.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-cohort-r3-20261030'

def main():
    raw=(PACKET/'index.json').read_bytes();a.need(a.sha(raw)==EXPECTED,'exact primary sealed index')
    index=json.loads(raw);members=index['members'];seen=set()
    for name,row in members.items():
        a.need(not name.startswith('/') and '..' not in name.split('/') and re.fullmatch('[0-9a-f]{64}',row['sha256']) and type(row['bytes']) is int and row['bytes']>=0,'contained object reference')
        path=PACKET/'objects'/row['sha256'];a.need(path.is_file() and not path.is_symlink(),'regular stored object');raw=path.read_bytes()
        a.need(a.sha(raw)==row['sha256'] and len(raw)==row['bytes'],'exact sealed bytes');seen.add(row['sha256'])
        a.need(not name.endswith(('/payload.tar.gz','/collect.stdout.log')) and '/input/images/' not in name and '/input/vendor/' not in name,'no compiled/vendor caches')
    a.need(seen=={p.name for p in (PACKET/'objects').iterdir()},'complete unique object inventory')
    def get(name):return (PACKET/'objects'/members[name]['sha256']).read_bytes()
    result=a.reconcile(lambda n:get('capture/chitchat-cohort-r3-20261030-r2/'+n))
    a.need(result==index['result'] and result['complete'] and not result['cpu_nonregression_gate_pass'] and result['memory_growth_screen_pass'],'full unchanged method result including negative CPU gate')
    a.need(index['predecessor_index_sha256']==a.PREDECESSOR and not index['incomplete_r2_result']['complete'],'unchanged first incomplete histories')
    for context in index['contexts']:
        prepared=json.loads(get('capture/'+context+'/prepared.json'));tool='chitchat-cohort-r2-074' if context=='chitchat-cohort-r2-20261029' else 'chitchat-cohort-r3-074'
        for name,digest in prepared['files'].items():
            a.need(a.sha(get('capture/'+context+'/payload/'+name))==digest,'original prepared bytes')
            if name not in ('maintenance_base.py','anchor.json'):a.need(a.sha(get('original-producer/'+context+'/tools/'+tool+'/'+name))==digest,'original committed producer')
    print(json.dumps(dict(complete=True,members=len(members),objects=len(seen),index_sha256=EXPECTED,numerical_processes=60,
        automatic_restoration_verified=True,cpu_nonregression_gate_pass=False,memory_growth_screen_pass=True,archived_code_executed=False)))

if __name__=='__main__':main()
