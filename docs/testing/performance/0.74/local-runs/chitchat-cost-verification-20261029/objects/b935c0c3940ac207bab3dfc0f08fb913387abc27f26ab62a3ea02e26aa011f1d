"""Byte-only sealed source/first-refusal review, never an executable replay."""
import io
import json
import re
import zipfile
import common as c

EXPECTED='9589ebd9014f7b2be15aa8cfedec635e06d042a4b81fe92da3894fdea1008462'
PACKET=c.ROOT/'docs/testing/performance/0.74/local-runs/chitchat-cost-20261029'


def main():
    raw=(PACKET/'index.json').read_bytes();c.need(c.sha(raw)==EXPECTED,'sealed first-refusal index')
    index=json.loads(raw);members=index['members'];seen=set()
    c.need(index['schema']=='hc074-chitchat-clean-cost-v1' and len(members)==967,'complete member accounting')
    for name,row in members.items():
        c.need(not name.startswith('/') and '..' not in name.split('/') and re.fullmatch('[0-9a-f]{64}',row['sha256']) and type(row['bytes']) is int and row['bytes']>=0,'contained object reference')
        path=PACKET/'objects'/row['sha256'];c.need(path.is_file() and not path.is_symlink(),'regular stored body')
        body=path.read_bytes();c.need(len(body)==row['bytes'] and c.sha(body)==row['sha256'],'exact sealed member');seen.add(row['sha256'])
    c.need(len(seen)==373 and seen=={p.name for p in (PACKET/'objects').iterdir()},'complete unique objects')
    def get(name):return (PACKET/'objects'/members[name]['sha256']).read_bytes()
    def read(name):return json.loads(get(name))
    for context in ('chitchat-cost-20261029','chitchat-cost-20261029-r1'):
        prefix='capture/'+context+'/';frame_raw=get(prefix+'frame.json');frame=json.loads(frame_raw);capsule=get(prefix+'source.zip');frozen=read(prefix+'format/result.json')
        c.need(c.sha(frame_raw)==frozen['frame_sha256'] and c.sha(capsule)==frozen['zip_sha256'],'exact original capsule')
        with zipfile.ZipFile(io.BytesIO(capsule)) as z:
            c.need(len(z.namelist())==len(set(z.namelist())) and set(z.namelist())==set(frame)|{'target/'+context+'/frame.json'},'complete unique ZIP')
            for name,digest in frame.items():c.need(c.sha(z.read(name))==digest,'original actual source bytes')
        for name,digest in c.PINS.items():c.need(frame[name]==digest,'unchanged Root/helper pin')
    p0='capture/chitchat-cost-20261029/';p1='capture/chitchat-cost-20261029-r1/'
    c.need(not read(p0+'build-B/result.json')['complete'] and b'E0599' in get(p0+'build-B/native/cargo/stdout.log'),'first constructor refusal retained')
    for variant in ('B','P','Q'):
        for action in ('build','check','clippy'):
            prefix=p1+action+'-'+variant+'/'
            c.need(read(prefix+'result.json')['complete'] and read(prefix+'native/session-result.json')['complete'],'complete compiler phase')
            for name in ('execution.json','native-envelope.json','native/cargo/execution.json'):
                r=read(prefix+name);c.need(r['exit']==0 and r['retired'] and not r['errors'] and r['bound_seconds']==1200 and r['capacity_poll_seconds']==r['cleanup_seconds']==5,'retired original compiler ownership')
        symbols=get(p1+'build-'+variant+'/native/imports/stdout.log')
        c.need(not any(v in symbols for v in (b'__wrap_malloc',b'cost_review',b'q_review',b'stateful_review')),'clean normal linked image')
        messages=[json.loads(s) for s in get(p1+'build-'+variant+'/native/cargo/stdout.log').decode().splitlines()]
        built=[v for v in messages if v.get('reason')=='compiler-artifact' and v.get('executable')]
        c.need(len(built)==1 and built[0]['profile']['test'] is False,'normal non-test artifact')
    prefix=p1+'control-B/';body=c.diagnostic(get(prefix+'native/observation/stdout.log').decode(),'HC074_CLEAN_COST:')
    c.need(body['rows']==read(p1+'input/expected-caller.json') and len(body['rows'])==30 and body['variant']=='B' and body['mode']=='control' and body['test_cfg'] is body['allocation_wrapper'] is False,'exact clean control semantics')
    c.need(not read(prefix+'result.json')['complete'] and read(prefix+'native/session-result.json')['first_refusal']['reason']=='no suspend/raw-clock discrepancy','unchanged first gate refusal')
    for name,code in (('execution.json',1),('native-envelope.json',1),('native/observation/execution.json',0)):
        r=read(prefix+name);c.need(r['exit']==code and r['retired'] and not r['errors'],'first refusal fully retired')
    image=read(p1+'build-B/native/artifact.json');call=read(prefix+'native/observation/intent.json')
    c.need(call['argv']==[image['path'],'control'] and call['image_sha256']==image['sha256'],'bound actual built control image')
    diagnosis=read(p1+'diagnosis/result.json');elapsed=body['elapsed']
    discrepancy=abs(elapsed['raw']-elapsed['mono'])/elapsed['raw'];protocol=read('source/tools/chitchat-cost-074/protocol.json')
    c.need(discrepancy==diagnosis['relative_discrepancy'] and discrepancy>protocol['monotonic_raw_drift_fraction']==0.01,'raw first clock refusal')
    for name in members:
        c.need(not re.search(r'/(?:launch-)?(?:cpu|memory)-\d+/',name) and not any('/'+x+'/' in name for x in ('control-P','control-Q')),'no measurement/continuation after refusal')
    c.need(index['cohort_processes_dispatched']==0 and index['clock_control_pass'] is False,'no invented cohort')
    for key in ('CPU_qualified','RSS_qualified','allocator_retention_qualified','root_adopted','release_admitted','generic_component_gate_pass','archived_code_executed'):
        c.need(index[key] is False,'no qualification promotion')
    for mode in ('controls-final','format','performance-contract'):c.need(read(p1+'final/'+mode+'/result.json')['complete'],'complete offline final check')
    print(json.dumps(dict(complete=True,members=len(members),objects=len(seen),index_sha256=EXPECTED,
        clock_control_pass=False,cohort_processes_dispatched=0,archived_code_executed=False,root_adopted=False)))


if __name__=='__main__':main()
