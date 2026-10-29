"""Read sealed JSON/ZIP/bytes; never execute captured source or binaries."""
import io
import json
import re
import zipfile
import common as c
import analyze as a

EXPECTED='874ba6355b98229cbeab11c5b0742e439cbced847e6954478234e10e92d7ef56'
PACKET=c.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-caller-20261029'


def main():
    raw=(PACKET/'index.json').read_bytes();c.need(c.sha(raw)==EXPECTED,'exact sealed index')
    index=json.loads(raw);members=index['members'];seen=set()
    c.need(index['schema']=='hc074-chitchat-caller-v1' and len(members)==1796,'complete schema/member accounting')
    for name,row in members.items():
        c.need(not name.startswith('/') and '..' not in name.split('/') and re.fullmatch('[0-9a-f]{64}',row['sha256']) and type(row['bytes']) is int and row['bytes']>=0,'contained object reference')
        path=PACKET/'objects'/row['sha256'];c.need(path.is_file() and not path.is_symlink(),'regular object')
        body=path.read_bytes();c.need(c.sha(body)==row['sha256'] and len(body)==row['bytes'],'sealed body '+name);seen.add(row['sha256'])
    c.need(len(seen)==463 and {p.name for p in (PACKET/'objects').iterdir()}==seen,'complete unique objects')
    def get(name):return (PACKET/'objects'/members[name]['sha256']).read_bytes()
    for context in ('chitchat-caller-20261029','chitchat-caller-20261029-r1','chitchat-caller-20261029-r2'):
        prefix='capture/'+context+'/';frame_raw=get(prefix+'frame.json');frame=json.loads(frame_raw)
        frozen=json.loads(get(prefix+'format/result.json'));capsule=get(prefix+'source.zip')
        c.need(c.sha(frame_raw)==frozen['frame_sha256'] and c.sha(capsule)==frozen['zip_sha256'],'original capsule/frame')
        with zipfile.ZipFile(io.BytesIO(capsule)) as z:
            c.need(len(z.namelist())==len(set(z.namelist())) and set(z.namelist())==set(frame)|{'target/'+context+'/frame.json'},'unique full ZIP inventory')
            for name,digest in frame.items():c.need(c.sha(z.read(name))==digest,'archived source input')
    c.need(c.sha(get('source/tools/chitchat-caller-074/cases-r2.json'))==c.TABLE_SHA,'exact premeasurement accepted table')
    prefix='capture/chitchat-caller-20261029-r2/';values=[]
    for i,variant in enumerate(('B','P','Q','Q','P','B'),1):
        part=prefix+f'cohort-{i}/native/';value=c.diagnostic(get(part+'observation/stdout.log').decode(),'HC074_CALLER:')
        c.need(value==json.loads(get(part+'observations.json')),'raw full observation')
        image=json.loads(get(prefix+f'build-{variant}/native/artifact.json'));call=json.loads(get(part+'observation/intent.json'));owner=json.loads(get(part+'observation/execution.json'))
        c.need(call['argv'][0]==image['path'] and owner['exit']==0 and owner['retired'] and not owner['errors'],'bound retired image')
        values.append(value)
    result=a.reconcile(values);c.need(result==json.loads(get(prefix+'analysis/result.json')) and result['fixed_caller_resource_gate_pass'] and not result['generic_component_gate_pass'],'separate exact caller/component verdicts')
    first=get('capture/chitchat-caller-20261029-r1/control-B/native/6/stdout.log').decode()
    c.need('left: 50' in first and 'right: 60' in first,'first canonical-input refusal retained')
    print(json.dumps(dict(complete=True,members=len(members),objects=len(seen),index_sha256=EXPECTED,archived_code_executed=False,root_adopted=False,release_admitted=False)))


if __name__=='__main__':main()
