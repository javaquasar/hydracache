"""Replay retained data independently; never imports the archived producer."""
import json
import analyze as a

OUT=a.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-context-20261030'


def validate(index,get):
    a.need(index['schema']=='hc074-chitchat-context-prerequisite-v1' and index['archived_code_executed'] is False,'exact data-only schema')
    a.need(index['parent_index_sha256']=='e6dd71ebb6818b13587b4750360ade76db5484e5083021ca6505699c039373b3','exact immutable dependency-only parent')
    a.need(all(index[k] is False for k in ('root_adopted','release_admitted','mtu_repaired')),'no release or MTU promotion')
    members=index['members']
    def read(name):
        row=members[name];raw=get(row['sha256'])
        a.need(len(raw)==row['bytes'] and a.hashlib.sha256(raw).hexdigest()==row['sha256'],'whole retained object: '+name)
        return raw
    for name in members:read(name)
    result=a.analyze(read)
    a.need(index['prerequisite_passed']==result['prerequisite_passed'] and result==json.loads(read('analysis-r5-final/result.json')),'complete independent prerequisite decision')
    return result


if __name__=='__main__':
    value=validate(json.loads((OUT/'index.json').read_bytes()),lambda h:(OUT/'objects'/h).read_bytes())
    print(json.dumps({k:v for k,v in value.items() if k!='observations'}))
