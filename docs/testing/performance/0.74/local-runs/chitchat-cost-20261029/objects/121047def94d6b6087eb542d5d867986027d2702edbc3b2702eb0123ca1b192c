"""Record source-derived premeasurement coverage refusal; never invokes images."""
from pathlib import Path
import hashlib
import json

ROOT=Path(__file__).resolve().parents[2];TOOLS=Path(__file__).resolve().parent
OUT=ROOT/'target/chitchat-caller-20261029/table-review';OUT.mkdir(exist_ok=False)
(OUT/'producer.py').write_bytes(Path(__file__).read_bytes());rows=[]
for filename in ('cases.json','cases-r1.json'):
    raw=(TOOLS/filename).read_bytes();table=json.loads(raw);observations=[]
    for case in table['cases']:
        if case['mtu']<=32768 or not case['name'].endswith('/large-then-small'):continue
        node,item=case['requests'][:2]
        # Audited actual scalar/str/IPv4/DeltaOp lengths; no compression estimate.
        node_bytes=1+(2+len(node['id'].encode())+8+1+4+2)+8+8
        item_bytes=1+2+len(item['key'].encode())+2+len(item['value'].encode())+8+1
        old_floor=node_bytes+item_bytes+6+1
        observations.append(dict(name=case['name'],node_bytes=node_bytes,item_bytes=item_bytes,old_floor=old_floor,mtu=case['mtu'],admitted_by_floor=old_floor<=case['mtu']))
    rows.append(dict(file=filename,sha256=hashlib.sha256(raw).hexdigest(),observations=observations))
assert all(not r['admitted_by_floor'] and r['old_floor']==r['mtu']+4 for r in rows[0]['observations'])
assert all(r['admitted_by_floor'] and r['old_floor']==r['mtu']-2 for r in rows[1]['observations'])
sources={n:hashlib.sha256((ROOT/'target/chitchat-caller-20261029/fixtures/B'/n).read_bytes()).hexdigest() for n in ('src/serialize.rs','src/delta.rs')}
(OUT/'result.json').write_text(json.dumps(dict(complete=True,initial_table_refused=True,Q_data_observed=False,cohort_dispatched=False,source=sources,tables=rows),indent=2)+'\n',encoding='utf8')
print('premeasurement table review: initial coverage refused; separate r1 required')
