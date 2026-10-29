"""Reconcile all first receipts with the unchanged frozen five-block estimator."""
import ast
import hashlib
import importlib.util
import json
import math
from pathlib import Path,PurePosixPath
import statistics
import types

ROOT=Path(__file__).resolve().parents[2];BASE=ROOT/'target/chitchat-cohort-r2-20261029'
PLAN_SHA='d73710aa0fbb958db510c60f103bf25a0a9457f749746277fd6703cd3a53052e'
NUMERIC='hc074-chitchat-cost-20261029.7wpdw9n4'
PACKAGE='hc074-chitchat-r2-package-20261029.bifv9n'
PREDECESSOR='5e4b0f0282d3cd7e329a25f2abe236201d023d6e3a682a79915bcbd9abdde1fa'
VALIDATOR_SHA='147ef73abb97c90acd5e6815315aec955f5b7194ca97cfaffe9e243217baab58'

def need(ok,why):
    if not ok:raise ValueError(why)
def sha(raw):return hashlib.sha256(raw).hexdigest()
def local(name):return (BASE/name).read_bytes()

def previous():
    folder=ROOT/'docs/testing/performance/0.74/local-runs/chitchat-cohort-20261029';raw=(folder/'index.json').read_bytes()
    need(sha(raw)==PREDECESSOR and not json.loads(raw)['terminal_result']['complete'],'unchanged first incomplete cohort')
    return json.loads(raw)

def parent(name):
    spec=importlib.util.spec_from_file_location('hc074_trusted_predecessor_accounting',ROOT/'tools/chitchat-cohort-074/analyze.py')
    old=importlib.util.module_from_spec(spec);spec.loader.exec_module(old);return old.parent(name)

def method():
    raw=(ROOT/'tools/chitchat-cost-074/analyze.py').read_bytes();need(sha(raw)==VALIDATOR_SHA,'unchanged current trusted validator and estimator')
    expected=parent('capture/chitchat-host-cost-20261029/prepared/input/semantics/input/expected-caller.json')
    protocol=parent('capture/chitchat-host-cost-20261029/prepared/input/protocol.json')
    class SemanticPath:
        def __init__(self,path=''):self.path=path
        def __truediv__(self,key):return SemanticPath(self.path+'/'+key)
        def read_bytes(self):need(self.path=='/input/expected-caller.json','exact semantic path');return expected
    namespace=dict(c=types.SimpleNamespace(BASE=SemanticPath(),need=need,protocol=lambda:json.loads(protocol)),json=json,math=math,statistics=statistics)
    nodes=[]
    for n in ast.parse(raw).body:
        if isinstance(n,ast.Assign) and len(n.targets)==1 and isinstance(n.targets[0],ast.Name) and n.targets[0].id in ('FIELDS','HEAP'):namespace[n.targets[0].id]=ast.literal_eval(n.value)
        if isinstance(n,ast.FunctionDef) and n.name in ('validate','interval','reconcile'):nodes.append(n)
    need(len(nodes)==3,'complete original numerical method closure')
    exec(compile(ast.Module(body=nodes,type_ignores=[]),'current_reviewed_frozen_cost_method','exec'),namespace)
    return namespace,json.loads(protocol)

def owner(v):
    need(v['exit']==v['observed_child_exit']==0 and v['retired'] and not v['errors'] and v['classification']=='success','original complete CI retirement')
    need(v['bound_seconds']==1200 and v['capacity_poll_seconds']==v['cleanup_seconds']==5 and v['production_writer_authority'] is False,'unchanged CI bounds and authority')

def reconcile(get=local):
    read=lambda n:json.loads(get(n));remote=lambda root,name:'collection/'+root+'/'+name;rread=lambda root,name:read(remote(root,name))
    previous();ns,protocol=method()
    faults=rread(PACKAGE,'faults-once/result.json');need(faults==read('remote-faults.stdout.log') and faults['complete'] and faults['stable_supervisor_unchanged'],'six original native faults')
    need([x['case'] for x in faults['cases']]==['healthy','deadline','owner-kill','frame-denial','frame-denial-owner-kill','namespace-owner-kill'],'all fixed fault cases')
    for index,case in enumerate(faults['cases']):
        root=PurePosixPath(case['root']).name;p=rread(root,'plan.json');post=rread(root,'postflight-result.json');stopped=rread(root,'all-writers-stopped.json')
        need(case['passed'] and case['postflight']==post and post['restoration_verified'] and rread(root,'terminal-complete.json')['namespace_retired'],'autonomous recovery and keeper retirement')
        need(stopped['original_groups_empty'] and stopped['original_fence_acquired'] and rread(root,'restored.json')['verified'],'original cgroup-inode/fence proof')
        need(p['synthetic'] and p['supervisor']==root+'-standin.service','faults only own stand-in')
        if index>=3:need(not post['full_measurement_frames_pass'] and len(post['frames'])==2,'denied numeric frames never promoted by recovery')
        if case['case']=='deadline':need(rread(root,'0.deadline.json')['expired'],'real unchanged absolute deadline fired')
    controls=rread(PACKAGE,'worker-controls-once/result.json');need(controls==read('remote-worker-controls.stdout.log') and controls['complete'] and controls['stable_supervisor_unchanged'],'whole-worker controls')
    control_root=PurePosixPath(controls['root']).name;identities=[]
    for i,variant in enumerate(('B','P','Q')):
        raw=get(remote(control_root,str(i)+'.stdout.log')).decode();lines=[x.removeprefix('HC074_CLEAN_COST:') for x in raw.splitlines() if x.startswith('HC074_CLEAN_COST:')]
        need(len(lines)==1,'one raw healthy control');body=json.loads(lines[0]);ns['validate'](body,variant,'control');owner(rread(control_root,str(i)+'.execution.json'))
        child=rread(control_root,str(i)+'.worker-child.json');need(body['pid']==child['pid'] and str(body['start_ticks'])==child['start_ticks'],'actual control process identity')
        identities.append((body['boot_id'],body['pid'],body['start_ticks']))
    raw=get(remote(NUMERIC,'plan.json'));need(sha(raw)==PLAN_SHA and raw==get('numeric-plan.json'),'exact new immutable numerical plan')
    p=json.loads(raw);root=PurePosixPath('/tmp')/NUMERIC;prepared=read('prepared.json')
    need(p['operator_approved'] and not p['synthetic'] and p['supervisor']=='hydracache-performance-supervisor-074.service','exact newly approved maintenance')
    need(p['fence_identity']==[2306,18101991] and p['protocol_sha256']==sha(parent('capture/chitchat-host-cost-20261029/prepared/input/protocol.json')),'original fence inode and protocol')
    need([p[k] for k in ('outer_bound_seconds','session_bound_seconds','capacity_poll_seconds','cleanup_seconds')]==[1200,1100,5,5],'unchanged full bounds')
    manifest_raw=parent('capture/chitchat-host-cost-20261029-r1/patch/input/inputs.json');manifest=json.loads(manifest_raw)
    files={str(root/'input'/n):h for n,h in manifest['files'].items()};files[str(root/'input/inputs.json')]=sha(manifest_raw)
    for n,h in prepared['files'].items():
        if n=='anchor.json':continue
        need(sha(get(remote(NUMERIC,n)))==h,'original reviewed executed producer');files[str(root/n)]=h
    need(p['inputs']==files and len(files)==5750,'complete original source/registry/image/helper graph')
    need(p['expected_host']==p['namespace']['host'] and p['namespace']['outer_bound_seconds']==1200,'one complete original mount context')
    need(p['namespace']['identity']!=[0,0] and not any(' shared:' in x or ' master:' in x for x in p['expected_host']['mounts'].splitlines()),'private pinned namespace, no host propagation')
    need(read('prelaunch-review.json')['partial_cohort_spliced'] is False and read('matrix-once.json')['retry'] is False,'one new matrix, no old splicing')
    try:manual=rread(NUMERIC,'manual-restored.json')
    except (FileNotFoundError,KeyError):manual=None
    if manual is not None:
        proof=rread(NUMERIC,'manual-recovery-fence.json');evidence=rread(NUMERIC,'manual-recovery-intent.json')
        need(manual['verified'] and not manual['automatic_postflight_pass'] and manual['supervisor']['MainPID']=='411840','exact terminal manual restoration')
        need(proof['original_fence_acquired'] and proof['durable_original_group_empty_proofs']==5 and not proof['missing_cgroup_used_as_empty_proof'],'all original proofs before recovery')
        need(len(evidence['prior_retirement_receipts'])==5 and not evidence['numerical_replay'],'five original attempts, no replay')
        for i in range(5):
            retired=rread(NUMERIC,str(i)+'.retired.json');need(retired['exit']==(0 if i<4 else 1) and retired['events']['populated']=='0','original durable empty-group proof')
            if i<4:
                body=rread(NUMERIC,str(i)+'.body.json');ns['validate'](body,('B','P','Q','Q')[i],'cpu');owner(rread(NUMERIC,str(i)+'.execution.json'))
        need(not get(remote(NUMERIC,'4.stdout.log')) and b'original kernel/clock/mount/marker/source host frame' in get(remote(NUMERIC,'4.stderr.log')),'fifth image never spawned')
        diff=read('namespace-diff.stdout.log');before_lines=p['expected_host']['mounts'].splitlines();after_lines=diff['private_host']['mounts'].splitlines()
        removed=[x for x in before_lines if x not in after_lines];added=[x for x in after_lines if x not in before_lines]
        need(len(removed)==1 and removed[0].split()[4]=='/run/user/1000' and not added,'exact full mount record disappeared despite private propagation')
        for name in ('cohort-complete.json','terminal-complete.json','restored.json'):
            try:get(remote(NUMERIC,name))
            except (FileNotFoundError,KeyError):pass
            else:raise ValueError('incomplete automatic cohort promoted')
        return dict(complete=False,terminal_accounting_complete=True,synthetic_faults=6,worker_controls=3,completed_cpu_images=4,completed_memory_images=0,
            worker_intents=5,automatic_restoration_verified=False,manual_restoration_verified=True,restored_supervisor_pid=411840,
            first_refusal='entire /run/user/1000 mount record removed from private namespace',estimator_applied=False,
            cpu_nonregression_gate_pass=False,memory_growth_screen_pass=False,continuous_retention_qualified=False,
            generic_component_gate_pass=False,root_adopted=False,release_admitted=False,retry=False,first_incomplete_result_unchanged=True,archived_code_executed=False)
    need(rread(NUMERIC,'cohort-complete.json')==dict(attempts=60,complete=True,admission=False),'full sixty-image completion')
    deadline=rread(NUMERIC,'session-deadline.json');need(deadline['session_bound_seconds']==1100 and deadline['outer_bound_seconds']==1200 and not deadline['rebased'],'original shared non-rebased session')
    cpu=[];memory=[]
    for i,a in enumerate(p['attempts']):
        expected_mode='cpu' if i<30 else 'memory';variant=('B','P','Q','Q','P','B')[i%6]
        need(a['mode']==expected_mode and a['variant']==variant,'all fixed positions/order')
        intent=rread(NUMERIC,str(i)+'.intent.json');retired=rread(NUMERIC,str(i)+'.retired.json');child=rread(NUMERIC,str(i)+'.worker-child.json')
        need(intent['attempt']==a and intent['deadline_ns']==deadline['deadline_ns'] and intent['boot']==p['boot'],'one original source attempt/deadline/boot')
        need(retired['exit']==0 and retired['events']['populated']=='0' and retired['all_original_writers_stopped'],'original group empty before next position')
        need(intent['group']==str(PurePosixPath('/sys/fs/cgroup/system.slice')/p['unit']/('writers-'+str(i))) and len(intent['group_identity'])==2,'exact owned writer group identity')
        need(intent['quiet']['maximum_percent']==10 and intent['quiet']['busy_percent']<=10,'unchanged quiet guard, no deleted samples')
        owner(rread(NUMERIC,str(i)+'.execution.json'))
        lines=[x.removeprefix('HC074_CLEAN_COST:') for x in get(remote(NUMERIC,str(i)+'.stdout.log')).decode().splitlines() if x.startswith('HC074_CLEAN_COST:')]
        need(len(lines)==1,'one original complete raw body');body=json.loads(lines[0]);need(body==rread(NUMERIC,str(i)+'.body.json'),'raw/body exact join')
        need(body['pid']==child['pid'] and str(body['start_ticks'])==child['start_ticks'] and body['boot_id']==p['boot'],'same original measured process')
        ns['validate'](body,variant,expected_mode);identities.append((body['boot_id'],body['pid'],body['start_ticks']))
        (cpu if expected_mode=='cpu' else memory).append(body)
    need(len(identities)==len(set(identities))==63,'63 distinct control/numerical processes')
    post=rread(NUMERIC,'postflight-result.json');terminal=rread(NUMERIC,'terminal-complete.json');stopped=rread(NUMERIC,'all-writers-stopped.json');restored=rread(NUMERIC,'restored.json')
    need(post['restoration_verified'] and post['full_measurement_frames_pass'] and not post['external_recovery'] and len(post['frames'])==2,'both full original terminal frames passed without fallback')
    need(stopped['original_groups_empty'] and stopped['original_fence_acquired'] and stopped['cohort_complete'],'all sixty original groups/fence proof')
    need(terminal['restoration_verified'] and terminal['namespace_retired'] and restored['verified'] and not restored['installation'],'autonomous complete restoration and namespace retirement')
    after=rread(NUMERIC,'host-postflight.json');need({k:v for k,v in after.items() if k!='supervisor'}=={k:v for k,v in p['expected_host'].items() if k!='supervisor'},'complete private host context unchanged through postflight')
    state=restored['supervisor'];need(state['ActiveState']=='active' and state['SubState']=='running' and state['MainPID']!='0' and state['NRestarts']=='0','exact restored healthy supervisor')
    result=ns['reconcile'](cpu,memory)
    result.update(terminal_accounting_complete=True,synthetic_faults=6,worker_controls=3,numerical_processes=60,automatic_restoration_verified=True,
        full_host_frames_pass=True,restored_supervisor_pid=int(state['MainPID']),retry=False,first_incomplete_result_unchanged=True,
        plan_sha256=PLAN_SHA,archived_code_executed=False)
    return result

if __name__=='__main__':
    folder=BASE/'analysis';folder.mkdir();result=reconcile()
    with (folder/'result.json').open('x') as f:json.dump(result,f,indent=2,sort_keys=True)
    print(json.dumps({k:v for k,v in result.items() if k not in ('timing','memory_growth_refusals')}))
    if result['complete']:print(json.dumps(dict(total=[x for x in result['timing'] if x['scope']=='total'],timing_refusals=[x for x in result['timing'] if x['upper95']>1.03],memory_refusals=result['memory_growth_refusals'])))
