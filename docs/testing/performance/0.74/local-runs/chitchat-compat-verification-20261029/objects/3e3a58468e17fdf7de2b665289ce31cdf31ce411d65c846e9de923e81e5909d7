"""Byte-only verification: archived producers are never imported or executed."""
import io
import json
import re
import zipfile
import common as c
import analyze as a

EXPECTED='0b195601989e9109725812ef5f8da76b3b2c9b4b42aed57e8e7992b6a25f35fa'
PACKET=c.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-stateful-20261029'


def main():
    raw=(PACKET/'index.json').read_bytes();c.need(c.sha(raw)==EXPECTED,'exact primary index')
    index=json.loads(raw);members=index['members'];seen=set()
    c.need(index['schema']=='hc074-chitchat-stateful-v1' and len(members)==841,'complete schema/members')
    for name,row in members.items():
        c.need(not name.startswith('/') and '..' not in name.split('/') and re.fullmatch('[0-9a-f]{64}',row['sha256']) and type(row['bytes']) is int and row['bytes']>=0,'contained regular object reference')
        path=PACKET/'objects'/row['sha256'];c.need(path.is_file() and not path.is_symlink(),'regular object')
        body=path.read_bytes();c.need(c.sha(body)==row['sha256'] and len(body)==row['bytes'],'stored bytes '+name);seen.add(row['sha256'])
    c.need(len(seen)==335 and {p.name for p in (PACKET/'objects').iterdir()}==seen,'complete object accounting')
    def get(name):return (PACKET/'objects'/members[name]['sha256']).read_bytes()
    frame_raw=get('capture/frame.json');frame=json.loads(frame_raw);frozen=json.loads(get('capture/format/result.json'))
    capsule=get('capture/source.zip');c.need(c.sha(frame_raw)==frozen['frame_sha256'] and c.sha(capsule)==frozen['zip_sha256'],'held capsule/frame')
    with zipfile.ZipFile(io.BytesIO(capsule)) as z:
        c.need(len(z.namelist())==len(set(z.namelist())) and set(z.namelist())==set(frame)|{c.RELBASE+'/frame.json'},'complete unique ZIP inventory')
        for name,digest in frame.items():c.need(c.sha(z.read(name))==digest,'archived source bytes')
    values=[]
    for i,variant in enumerate(('B','P','Q','Q','P','B'),1):
        prefix=f'capture/cohort-{i}/native/'
        value=c.diagnostic(get(prefix+'observation/stdout.log').decode(),'HC074_STATEFUL:')
        c.need(value==json.loads(get(prefix+'observations.json')),'raw full observation')
        image=json.loads(get(f'capture/build-{variant}/native/artifact.json'))
        call=json.loads(get(prefix+'observation/intent.json'));owner=json.loads(get(prefix+'observation/execution.json'))
        c.need(call['argv'][0]==image['path'] and call['frame']==frozen['frame_sha256'] and owner['exit']==0 and owner['retired'] and not owner['errors'],'bound retired actual source image')
        values.append(value)
    result=a.reconcile(values);c.need(result==json.loads(get('capture/analysis/result.json')) and not result['component_resource_gate_pass'],'retained exact negative result')
    print(json.dumps(dict(complete=True,index_sha256=EXPECTED,members=len(members),objects=len(seen),archived_code_executed=False,root_adopted=False,release_admitted=False)))


if __name__=='__main__':main()
