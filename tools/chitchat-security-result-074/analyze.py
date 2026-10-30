"""Post-capture data analysis. Never executes an archived producer or test image."""
import json
from pathlib import Path
import sys
import tomllib
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'chitchat-security-only-074'))
import scope as c
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'chitchat-security-only-074'))
import candidate
import runner
import finalize

MARKERS=('HC074_STREAM_BOUNDS:','HC074_SUPPORTED_DELTA:','HC074_SECURITY_MALFORMED:')

def diagnostic(raw,marker):
    text=raw.decode();c.need(text.count(marker)==1,'exact diagnostic body')
    return json.loads(text.split(marker,1)[1].splitlines()[0])

def analyze(read,exists):
    root=read('root/Cargo.lock');c.need(c.sha(root)==c.PINS['Cargo.lock'],'same immutable Root')
    old,variants=candidate.variants(read('input/archives/chitchat-0.11.1.crate'))
    for name,files in variants.items():
        for p,body in files.items():c.need(read('canonical/'+name+'/'+p)==body,'whole exact source inverse')
    graphs=finalize.review_graphs({n:read('fixtures/'+n+'/Cargo.lock') for n in ('B','Z','U')})
    deny=read('root/deny.toml');c.need(c.sha(deny)==c.PINS['deny.toml'],'unchanged complete Root policy')
    config=tomllib.loads(read('policy/deny-location-only.toml').decode());config['advisories'].pop('db-path')
    c.need(config==tomllib.loads(deny.decode()),'only advisory storage location differs')
    db=json.loads(read('policy/snapshot.json'))
    for name,digest in db['files'].items():c.need(c.sha(read('policy/advisory-dbs/advisory-db-3157b0e258782691/'+name))==digest,'whole frozen advisory database')
    policy={}
    for name,expected in [('B',['RUSTSEC-2026-0253','RUSTSEC-2026-0391','RUSTSEC-2026-0398','RUSTSEC-2026-0403']),('Z',['RUSTSEC-2026-0253']),('U',[])]:
        result=json.loads(read('policy/'+name+'/result.json'))
        c.need(result['complete'] and result['errors']==expected and result['policy_passed']==(not expected),'exact complete source-policy verdict')
        errors=[]
        for line in read('policy/'+name+'/stderr.log').decode().splitlines():
            fields=json.loads(line).get('fields',{})
            if fields.get('severity')=='error':errors.append(fields['advisory']['id'])
        c.need(sorted(errors)==expected,'raw full-policy verdict, not only summary flags')
        policy[name]=expected
    peers={};snapshots={};known_negatives=0
    for label in sorted(runner.PEERS):
        base='peers-r4/'+label+'/'
        if not exists(base+'result.json'):
            peers[label]=dict(complete=False,state='not-dispatched');continue
        value=json.loads(read(base+'result.json'));peers[label]=value
        if not value['complete']:continue  # No partial inner result admission.
        owner=json.loads(read(base+'execution.json'));intent=json.loads(read(base+'intent.json'))
        c.need(owner['classification']=='success' and owner['exit']==0 and owner['retired'] and not owner['errors'],'complete actual outer owner')
        c.need(owner['bound_seconds']==1200 and owner['cleanup_seconds']==owner['capacity_poll_seconds']==5,'original complete outer envelope')
        for name,digest in c.PINS.items():c.need(intent['source'][name]==digest,'unchanged exact Root/helper pin')
        c.need(set(intent['input_trees'])=={'fixtures','canonical','input'},'whole original input-tree scope')
        for category,rows in intent['input_trees'].items():
            for name,digest in rows.items():c.need(c.sha(read(category+'/'+name))==digest,'complete source/archive/lock preimage')
        session=json.loads(read(base+'session-result.json'));c.need(session['complete'] and session['wide_gate_passed'] is False,'complete first session only')
        catalog=json.loads(read(base+'catalog.json'));names=catalog['all'];selected=catalog['selected']
        listed=read(base+'catalog/stdout.log').decode().splitlines()
        actual=sorted(line[:-6] for line in listed if line.endswith(': test'))
        c.need(len(names)==len(set(names))==106 and names==actual and listed[-1]=='106 tests, 0 benchmarks','complete actual unshortened native catalog')
        expected=[n for n in names if n.startswith(runner.PURE) or n in runner.OBS or n==runner.CONTEXT and not label.endswith('-B')]
        c.need(selected==expected,'all original mandatory selections, including property tests')
        c.need(catalog['negative_contracts']==(list(runner.NEGATIVE) if label.startswith('windows-194-') else []),'all unchanged negative contracts are actually dispatched')
        c.need(len(selected)==session['passed'] and set(names)==set(selected)|set(catalog['unexecuted'])|set(catalog['negative_contracts']),'complete native catalog partition')
        diagnostics={}
        for i,test in enumerate(selected):
            folder=base+f'test-{i:03}/';execution=json.loads(read(folder+'execution.json'));intent=json.loads(read(folder+'intent.json'))
            c.need(execution['classification']=='success' and execution['retired'] and not execution['errors'] and execution['exit']==0,'complete original case owner')
            c.need(execution['bound_seconds']==5 and intent['argv'][1:]==['--exact',test,'--test-threads=1','--nocapture'],'unchanged actual selection and five-second cap')
            raw=read(folder+'stdout.log');runner.account(raw,names,test,False)
            for marker in MARKERS:
                if marker.encode() in raw:diagnostics[marker]=diagnostic(raw,marker)
        c.need(set(diagnostics)==set(MARKERS),'all complete diagnostic tables')
        c.need(len(diagnostics[MARKERS[0]]['cases'])==37 and len(diagnostics[MARKERS[1]]['cases'])==239 and len(diagnostics[MARKERS[2]]['rows'])==7,'whole original corpora')
        for i,test in enumerate(catalog['negative_contracts']):
            folder=base+'known-bound-'+str(i)+'/';execution=json.loads(read(folder+'execution.json'))
            c.need(execution['exit']==101 and execution['classification']=='test-failure' and execution['retired'] and not execution['errors'],'actual unchanged negative contract')
            runner.account(read(folder+'stdout.log'),names,test,True);known_negatives+=1
        snapshots[label]=diagnostics
    complete=all(v['complete'] for v in peers.values())
    same=None
    if complete:
        first=next(iter(snapshots.values()))
        c.need(all(v==first for v in snapshots.values()),'same complete diagnostics on every platform/compiler/input')
        same=True
    return dict(schema='hc074-chitchat-dependency-only-result-v1',context='r4',source_members=len(old),graph=graphs,policy=policy,peers=peers,
      complete_functional_matrix=complete,completed_peers=sum(v['complete'] for v in peers.values()),planned_peers=12,
      diagnostic_tables_equal=same,actual_known_bound_failures=known_negatives,wide_gate_passed=False,
      source_algorithms_unchanged=True,q_adopted=False,root_adopted=False,registry_qualified=False,
      performance_claim=False,CPU_qualified=False,resources_qualified=False,release_admitted=False)

def live():
    def read(name):
        return (c.ROOT/name.removeprefix('root/')).read_bytes() if name.startswith('root/') else (c.BASE/name).read_bytes()
    return analyze(read,lambda n:(c.BASE/n).exists())

if __name__=='__main__':print(json.dumps(live(),indent=2,sort_keys=True))
