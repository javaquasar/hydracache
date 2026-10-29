"""Account the incomplete first cohort and exact recovery; no partial estimator.

Only current reviewed repository validation code runs. Archived producers and
input capsules are bytes, never imported. No SSH, service or measurement action.
"""
import ast
import hashlib
import json
from pathlib import Path, PurePosixPath
import types

ROOT=Path(__file__).resolve().parents[2]
R0='chitchat-cohort-20261029';R1=R0+'-r1'
PACKAGE='hc074-chitchat-cohort-package-20261029.oO3u1l'
NUMERIC='hc074-chitchat-cost-20261029.er47i8gp'
WORKER='hc074-chitchat-cost-20261029.qv1yctbx'
PLAN_SHA='c38d5aa3712ad53b8f42aa706d8b4086258eb0b085d5cc46d7473491218e47f0'
PARENT_SHA='134607dd8e6f8dced2bb1f0f1eb1b594f1ccc70c802a8c7d601bd64b2cf7e670'
INPUT_SHA='90057ac9679be66d141a0f3b573fa6d911d8287365051fc10e379479ffb26c20'
VALIDATOR_SHA='147ef73abb97c90acd5e6815315aec955f5b7194ca97cfaffe9e243217baab58'
STABLE='hydracache-performance-supervisor-074.service'


def need(ok,why):
    if not ok:raise ValueError(why)


def sha(raw):return hashlib.sha256(raw).hexdigest()
def local(name):return (ROOT/'target'/name).read_bytes()


def parent(name):
    folder=ROOT/'docs/testing/performance/0.74/local-runs/chitchat-host-cost-20261029'
    raw=(folder/'index.json').read_bytes();need(sha(raw)==PARENT_SHA,'exact native control predecessor')
    row=json.loads(raw)['members'][name];raw=(folder/'objects'/row['sha256']).read_bytes()
    need(sha(raw)==row['sha256'] and len(raw)==row['bytes'],'original predecessor bytes');return raw


def validation(expected,protocol):
    raw=(ROOT/'tools/chitchat-cost-074/analyze.py').read_bytes();need(sha(raw)==VALIDATOR_SHA,'unchanged current trusted validation predicates')
    class SemanticPath:
        def __init__(self,path=''):self.path=path
        def __truediv__(self,key):return SemanticPath(self.path+'/'+key)
        def read_bytes(self):
            need(self.path=='/input/expected-caller.json','exact original semantic input');return expected
    ns=dict(c=types.SimpleNamespace(need=need,protocol=lambda:protocol,BASE=SemanticPath()),json=json)
    tree=ast.parse(raw)
    for node in tree.body:
        if isinstance(node,ast.Assign) and len(node.targets)==1 and isinstance(node.targets[0],ast.Name) and node.targets[0].id in ('FIELDS','HEAP'):
            ns[node.targets[0].id]=ast.literal_eval(node.value)
    node=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='validate')
    exec(compile(ast.Module(body=[node],type_ignores=[]),'current_reviewed_cost_validate','exec'),ns)
    return ns['validate']


def owner(v):
    need(v['exit']==v['observed_child_exit']==0 and v['retired'] and not v['errors'] and v['classification']=='success','original complete CI ownership')
    need(v['bound_seconds']==1200 and v['capacity_poll_seconds']==v['cleanup_seconds']==5 and v['production_writer_authority'] is False,'original ownership limits, no product authority')


def reconcile(get=local):
    read=lambda n:json.loads(get(n))
    remote=lambda root,name:R1+'/collection/'+root+'/'+name
    rread=lambda root,name:read(remote(root,name))
    def absent(root,name):
        try:get(remote(root,name))
        except (FileNotFoundError,KeyError):return
        raise ValueError('unexpected later effect or promoted automatic result: '+name)
    before=read(R1+'/preflight.stdout.log')['host']
    need(read(R0+'/remote-faults-result.json')['complete'] is False and b'os.sched_getaffinity(0)' in get(R0+'/remote-faults.stderr.log'),'first SSH-affinity-domain refusal retained')
    need(read(R1+'/final/controls/result.json')['complete'] is False and b"does not have the attribute 'sched_getaffinity'" in get(R1+'/final/controls/stderr.log'),'first Windows mock refusal retained')
    need(read(R1+'/review-first-refusal.json')['numeric_launched'] is False,'first offline inventory-count refusal retained')
    for mode in ('controls-prepared','format','base-controls','controls-worker','syntax-worker'):
        need(read(R1+'/final/'+mode+'/result.json')['complete'],'original prelaunch owned check')
        owner(read(R1+'/final/'+mode+'/execution.json'))
    faults=rread(PACKAGE,'faults-once/result.json')
    need(faults==read(R1+'/remote-faults.stdout.log') and faults['complete'] and faults['stable_supervisor_unchanged'],'whole three-case native fault result')
    need([c['case'] for c in faults['cases']]==['healthy','deadline','owner-kill'] and all(c['passed'] for c in faults['cases']),'all fixed fault cases')
    for case in faults['cases']:
        name=PurePosixPath(case['root']).name;p=rread(name,'plan.json');stopped=rread(name,'all-writers-stopped.json')
        need(p['synthetic'] and p['supervisor']==name+'-standin.service' and rread(name,'restored.json')['verified'],'only own stand-in restored')
        need(stopped['original_groups_empty'] and stopped['original_fence_acquired'],'native empty-inode/fence proof')
        need(rread(name,'host-postflight.json')==before,'stable service and host unchanged by synthetic faults')
        if case['case']=='deadline':need(rread(name,'0.deadline.json')['expired'],'actual absolute watchdog expiry')
        need(rread(name,'0.retired.json')['events']['populated']=='0' if case['case']!='owner-kill' else stopped['original_groups_empty'],'original writer retirement')
    protocol=json.loads(parent('capture/chitchat-host-cost-20261029/prepared/input/protocol.json'))
    expected=parent('capture/chitchat-host-cost-20261029/prepared/input/semantics/input/expected-caller.json')
    validate=validation(expected,protocol)
    worker=rread(PACKAGE,'worker-controls-once/result.json');need(worker==read(R1+'/worker-controls.stdout.log') and worker['complete'] and worker['root']=='/tmp/'+WORKER,'whole worker B/P/Q controls')
    need(worker['stable_supervisor_unchanged'] and rread(WORKER,'host-postflight.json')==before,'stable unchanged throughout worker smoke')
    need(rread(WORKER,'all-writers-stopped.json')['original_fence_acquired'] and rread(WORKER,'cohort-complete.json')['attempts']==3,'complete whole-worker fence control')
    bodies=[]
    for i,v in enumerate(('B','P','Q')):
        owner(rread(WORKER,str(i)+'.execution.json'))
        lines=[x.removeprefix('HC074_CLEAN_COST:') for x in get(remote(WORKER,str(i)+'.stdout.log')).decode().splitlines() if x.startswith('HC074_CLEAN_COST:')]
        need(len(lines)==1,'one raw body');body=json.loads(lines[0]);validate(body,v,'control')
        child=rread(WORKER,str(i)+'.worker-child.json');need(body['pid']==child['pid'] and str(body['start_ticks'])==child['start_ticks'] and body['boot_id']==before['boot'],'exact actual worker image identity')
        bodies.append(body)
    raw=get(remote(NUMERIC,'plan.json'));need(sha(raw)==PLAN_SHA and raw==get(R1+'/numeric-plan.json'),'exact immutable numerical plan')
    p=json.loads(raw);root=PurePosixPath('/tmp')/NUMERIC
    need(p['operator_approved'] and not p['synthetic'] and p['supervisor']==STABLE and p['expected_host']==before,'separate finite approved service scope')
    need(p['fence_identity']==[2306,18101991] and p['cpu_profile']==dict(online='0-7',system_slice_effective='0-7'),'original inode/CPU placement')
    need([p[k] for k in ('outer_bound_seconds','session_bound_seconds','capacity_poll_seconds','cleanup_seconds')]==[1200,1100,5,5],'unchanged original bounds')
    need([(a['mode'],a['variant']) for a in p['attempts']]==[(m,v) for m in ('cpu','memory') for v in ('B','P','Q','Q','P','B')*5],'all60 exact planned positions')
    manifest_raw=parent('capture/chitchat-host-cost-20261029-r1/patch/input/inputs.json');need(sha(manifest_raw)==INPUT_SHA,'original 5742-input manifest')
    manifest=json.loads(manifest_raw);files={str(root/'input'/n):digest for n,digest in manifest['files'].items()}
    files[str(root/'input/inputs.json')]=INPUT_SHA
    for n,digest in read(R1+'/prepared.json')['files'].items():
        need(sha(get(remote(NUMERIC,n)))==digest,'original executed controller bytes');files[str(root/n)]=digest
    need(files==p['inputs'] and len(files)==5748,'complete original source/registry/image/protocol/controller graph')
    need(p['protocol_sha256']==sha(parent('capture/chitchat-host-cost-20261029/prepared/input/protocol.json')),'unchanged actual protocol bytes')
    need(read(R1+'/prelaunch-review.json')['reviewed'] and read(R1+'/numerical-launch-once.json')['retry'] is False,'review and exclusive launch latch')
    for name in ('cohort-complete.json','all-writers-stopped.json','restored.json','host-postflight.json','4.execution.json','4.worker-child.json'):absent(NUMERIC,name)
    for i in range(5,60):
        for suffix in ('.intent.json','.child.json','.worker-child.json','.body.json'):absent(NUMERIC,str(i)+suffix)
    completed=[]
    for i,v in enumerate(('B','P','Q','Q')):
        lines=[x.removeprefix('HC074_CLEAN_COST:') for x in get(remote(NUMERIC,str(i)+'.stdout.log')).decode().splitlines() if x.startswith('HC074_CLEAN_COST:')]
        need(len(lines)==1,'one first raw image body');body=json.loads(lines[0]);validate(body,v,'cpu')
        need(body==rread(NUMERIC,str(i)+'.body.json'),'validated body equals raw stream');owner(rread(NUMERIC,str(i)+'.execution.json'))
        child=rread(NUMERIC,str(i)+'.worker-child.json');need(body['pid']==child['pid'] and str(body['start_ticks'])==child['start_ticks'] and body['boot_id']==before['boot'],'actual image identity')
        completed.append(body)
    need(len({(b['pid'],b['start_ticks']) for b in bodies+completed})==7,'distinct control/measured processes')
    failed=get(remote(NUMERIC,'4.stderr.log')).decode();need('original kernel/clock/mount/marker/source host frame' in failed and not get(remote(NUMERIC,'4.stdout.log')),'first fifth worker refuses before measured image')
    intent=rread(NUMERIC,'manual-recovery-intent.json');proof=rread(NUMERIC,'manual-recovery-fence.json');restored=rread(NUMERIC,'manual-restored.json')
    need(get(R1+'/manual-recovery-producer.py')==(ROOT/'tools/chitchat-cohort-074/recover.py').read_bytes(),'exact current reviewed one-shot recovery source')
    need(intent['original_plan_sha256']==PLAN_SHA and not intent['numerical_replay'] and len(intent['prior_retirement_receipts'])==5,'terminal recovery with all first worker receipts')
    for i,entry in enumerate(intent['prior_retirement_receipts']):
        need(entry['intent']==rread(NUMERIC,str(i)+'.intent.json') and entry['retired']==rread(NUMERIC,str(i)+'.retired.json'),'raw original durable empty-inode proof')
        need(entry['retired']['events']['populated']=='0' and entry['retired']['all_original_writers_stopped'] and entry['retired']['exit']==(0 if i<4 else 1),'all original writers empty before cgroup collection')
        need(entry['intent']['deadline_ns']==rread(NUMERIC,'session-deadline.json')['deadline_ns'],'one absolute original deadline')
    need(proof==dict(cohort_complete=False,deadline_units_retired=10,durable_original_group_empty_proofs=5,fence_identity=[2306,18101991],measurement_frame_pass=False,missing_cgroup_used_as_empty_proof=False,original_fence_acquired=True,original_processes_gone=9),'exact explicit restoration custody proof')
    from recover import host_review
    host_review(before,intent['host']);host_review(before,restored['host'])
    need(restored['verified'] and restored['supervisor']['ActiveState']=='active' and restored['supervisor']['SubState']=='running' and restored['supervisor']['MainPID']=='409364' and restored['supervisor']['NRestarts']=='0','actual exact supervisor restoration')
    for name in ('automatic_postflight_pass','measurement_frame_pass','CPU_qualified','RSS_qualified','numerical_replay','installation'):need(restored[name] is False,'no recovery-to-measurement promotion')
    journal=get(R1+'/host-refusal.stdout.log').decode();need('first child refusal 4: 1; no retry' in journal and 'Control process exited' in journal,'both main and automatic postflight refusals retained')
    return dict(complete=False,terminal_accounting_complete=True,planned_processes=60,worker_intents=5,retired_worker_groups=5,
        completed_cpu_images=4,completed_memory_images=0,controls=3,synthetic_faults=3,first_refusal='unrelated /run/user/1000 tmpfs identity changed',
        automatic_postflight_pass=False,manual_supervisor_restored=True,restored_pid=409364,root_adopted=False,release_admitted=False,
        CPU_qualified=False,RSS_qualified=False,cpu_nonregression_gate_pass=False,memory_growth_screen_pass=False,
        continuous_retention_qualified=False,generic_component_gate_pass=False,estimator_applied=False,retry=False,archived_code_executed=False)


if __name__=='__main__':
    folder=ROOT/'target'/R1/'analysis';folder.mkdir(exist_ok=False);result=reconcile()
    with (folder/'result.json').open('x') as out:json.dump(result,out,indent=2,sort_keys=True)
    print(json.dumps(result))
