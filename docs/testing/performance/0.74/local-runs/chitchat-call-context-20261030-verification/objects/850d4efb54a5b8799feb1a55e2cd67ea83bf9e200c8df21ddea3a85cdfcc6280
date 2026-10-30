"""Only stored bytes and current reviewer; no archived producer execution."""
import json
import finalize as f
r=f.r
OUT=r.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-call-context-20261030'


def validate(index,get):
    r.need(index['schema']=='hc074-chitchat-call-context-v1' and index['parent_index_sha256']==r.PARENT_SHA and index['archived_producer_executed'] is False,'exact data-only scope/parent')
    r.need(all(index[n] is False for n in ('root_adopted','release_admitted','mtu_repaired','full_resource_contract_passed')),'no unqualified promotion')
    def read(name):
        row=index['members'][name];raw=get(row['sha256'])
        r.need(len(raw)==row['bytes'] and r.sha(raw)==row['sha256'],'whole retained object: '+name);return raw
    for name in index['members']:read(name)
    result=f.full(read)
    r.need(index['prerequisite_passed']==result['prerequisite_passed'] and result==json.loads(read('final-analysis/result.json')),'complete independent decision')
    return result


if __name__=='__main__':
    value=validate(json.loads((OUT/'index.json').read_bytes()),lambda h:(OUT/'objects'/h).read_bytes())
    print(json.dumps({k:v for k,v in value.items() if k!='observations'}))
