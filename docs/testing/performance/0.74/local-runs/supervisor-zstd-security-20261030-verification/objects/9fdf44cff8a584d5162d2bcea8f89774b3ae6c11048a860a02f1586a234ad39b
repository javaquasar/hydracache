"""Verify stored data without executing any archived producer or Rust image."""
import json
from pathlib import Path
import re
import sys
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'supervisor-zstd-security-074'))
import common as c
import graph
from retain import OUT

def validate(index,read):
    c.need(index['schema']=='hc074-direct-supervisor-zstd-v1' and index['archived_code_executed'] is False,'data-only evidence schema')
    data={}
    for name,row in index['members'].items():
        raw=read(row['sha256']);c.need(len(raw)==row['bytes'] and c.sha(raw)==row['sha256'],'exact stored object: '+name);data[name]=raw
    result=json.loads(data['analysis.json']);c.need(index['result']==result,'one reconciled analysis')
    for name in ('workspace_gate_passed','release_admitted','q_adopted','q_cpu_gate_passed','standalone_wire_is_exact_root_graph','windows_full_formatter_passed'):
        c.need(result[name] is False,'no promoted claim: '+name)
    old=data['capture/baseline/Cargo.lock'];new=data['candidate/Cargo.lock'];review=graph.review(old,new)
    c.need(all(result[k]==v for k,v in review.items()),'exact single-edge historical graph')
    old_manifest=data['capture/baseline/supervisor.Cargo.toml']
    c.need(data['candidate/'+c.PKG+'/Cargo.toml']==old_manifest.replace(b'zstd = "0.13.3"',b'zstd = "=0.14.3"'),'only direct private consumer changes')
    for platform in ('','gnu/'):
        for version in ('194','188'):
            for i,name in enumerate(c.TESTS):
                base='capture/'+platform+f'candidate-archive-{i}-{version}/'
                execution=json.loads(data[base+'execution.json']);intent=json.loads(data[base+'intent.json'])
                c.need(execution['classification']=='success' and execution['exit']==0 and execution['retired'] and not execution['errors'],'original successful archive owner')
                c.need(execution['bound_seconds']==5 and execution['cleanup_seconds']==execution['capacity_poll_seconds']==5,'original case deadlines')
                c.need(intent['argv'][1:]==['--exact',name,'--test-threads=1','--nocapture'] and intent['source']['Cargo.lock']==c.sha(new),'same exact case and Root graph')
                raw=data[base+'stdout.log'].decode()
                c.need(re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',raw)==[('1','0','0','0','3')],'all four exact cases, no hidden ignore')
    for scope,ids in [('supervisor',[]),('root',['RUSTSEC-2026-0391','RUSTSEC-2026-0398','RUSTSEC-2026-0403'])]:
        errors=[]
        for line in data['capture/policy/'+scope+'/stderr.log'].decode().splitlines():
            fields=json.loads(line).get('fields',{})
            if fields.get('severity')=='error':errors.append(fields['advisory']['id'])
        c.need(sorted(errors)==ids,'exact actual policy errors: '+scope)
        verdict=json.loads(data['capture/policy/'+scope+'/result.json'])
        c.need(verdict['complete'] and verdict['policy_passed']==(not ids),'exact policy disposition')
    return result

def main():
    index=json.loads((OUT/'index.json').read_bytes());result=validate(index,lambda h:(OUT/'objects'/h).read_bytes())
    print(json.dumps(dict(objects_verified=len(index['members']),candidate_lock_sha256=result['candidate_sha256'],root_remaining_advisories=result['root_remaining_advisories'],release_admitted=False)))

if __name__=='__main__':main()
