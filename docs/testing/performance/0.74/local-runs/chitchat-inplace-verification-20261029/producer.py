"""Independent byte/raw-log accounting; no archived code is executed."""
from pathlib import Path
import hashlib
import json

ROOT=Path(__file__).resolve().parent.parent
PACKET=ROOT/'docs/testing/performance/0.74/local-runs/chitchat-inplace-20261029'
sha=lambda raw:hashlib.sha256(raw).hexdigest()
raw=(PACKET/'index.json').read_bytes()
assert sha(raw)=='9381608b814a7fcdc16835ece436534fc1d48f816039023f9c0adbc16a234fd1'
index=json.loads(raw);members=index['members']
for row in members.values():
    body=(PACKET/'objects'/row['sha256']).read_bytes()
    assert len(body)==row['bytes'] and sha(body)==row['sha256']
assert len(members)==3476 and len({v['sha256'] for v in members.values()})==1130


def get(name):
    return (PACKET/'objects'/members[name]['sha256']).read_bytes()


def payload(raw,marker):
    rows=[v[len(marker):] for v in raw.splitlines() if v.startswith(marker)]
    assert len(rows)==1
    return json.loads(rows[0])


result={}
for candidate,expected in (('S',71),('D',9),('P',0)):
    values=[]
    for position in range(1,5):
        stem=f'capture/cohort-{candidate}-194-{position}'
        done=json.loads(get(stem+'/result.json'));assert done['complete']
        owned=json.loads(get(stem+'/allocation/execution.json'))
        assert owned['exit']==0 and owned['retired'] and not owned['errors']
        value=payload(get(stem+'/allocation/stdout.log'),b'HC074_INPLACE_ALLOC:')
        assert value==json.loads(get(stem+'/observations.json'))
        assert value['seed']==0x0741028 and value['native_C_allocations_included'] is False
        assert len(value['rows'])==239
        assert len({(r['mtu'],r['value_len']) for r in value['rows']})==239
        assert sum(r['accepted'] for r in value['rows'])==136
        assert all(len(r['records'])==2 and r['records'][0]==r['records'][1] for r in value['rows'])
        assert all(r['records'][0]['complete_counts']['live']==0 for r in value['rows'])
        values.append(value)
    assert values[0]==values[3] and values[1]==values[2]
    changed=[]
    for before,after in zip(values[0]['rows'],values[1]['rows'],strict=True):
        assert all(before[k]==after[k] for k in ('mtu','value_len','accepted','wire'))
        b=before['records'][0]['complete_counts'];a=after['records'][0]['complete_counts']
        if any(a[k]>b[k] for k in ('allocs','reallocs','requested','peak')):changed.append(after)
        if candidate=='P':assert a==b
    assert len(changed)==expected
    result[candidate]=dict(cases=239,regressions=len(changed),accepted_regressions=sum(v['accepted'] for v in changed))
print(json.dumps(dict(complete=True,members_verified=len(members),objects_verified=1130,cohorts=result,
                     byte_raw_log_verification=True,archived_code_executed=False,root_adopted=False)))
