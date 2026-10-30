"""Independent C2 data review: exact source, workload, wire and owned memory."""
import ast
import hashlib
import json
from pathlib import Path
import re
import traceback

ROOT=Path(__file__).resolve().parents[2]
BASE=ROOT/'target/chitchat-call-context-20261030'
PARENT=ROOT/'docs/testing/performance/0.74/local-runs/chitchat-context-20261030'
PARENT_SHA='9668415d296e8ed37cbbc371b28bbf5396d3ddd52aaa3d4a9b2899c6420de47a'
FIELDS={'malloc_calls','calloc_calls','realloc_calls','free_calls','allocs','requested','failed','external_frees','live','peak'}
DOMINANCE=('allocs','realloc_calls','requested','peak','live')
POINTS=('construct','append1','append2','finish','retirement')
sha=lambda raw:hashlib.sha256(raw).hexdigest()


def need(value,why):
    if not value:raise ValueError(why)


def reconcile(before,after):
    for variant,value in [('B0',before),('C2',after)]:
        need(set(value)=={'threshold','repetitions','wire','variant','context_present','CPU_qualified','RSS_qualified','allocator_page_retention_qualified',*POINTS},'complete exact resource schema')
        need(value['variant']==variant and type(value['context_present']) is list and len(value['context_present'])==3 and all(type(v) is bool and not v for v in value['context_present']),'actual input identity and no context at public checkpoints')
        need(type(value['threshold']) is type(value['repetitions']) is int and value['threshold']==64 and value['repetitions']==30,'same frozen original workload')
        need(all(value[n] is False for n in ('CPU_qualified','RSS_qualified','allocator_page_retention_qualified')),'no unmeasured scope promotion')
        need(type(value['wire']) is list and value['wire'] and all(type(v) is int and 0<=v<=255 for v in value['wire']),'complete exact wire')
        previous={v:{k:0 for k in DOMINANCE[:-1]} for v in ('c','rust')}
        for point in POINTS:
            entry=value[point]
            if point in POINTS[:3]:
                need(type(entry) is list and len(entry)==2,'complete snapshot/inventory pair');snapshot,inventory=entry
                need(set(inventory)=={'len','capacity'} and len(inventory['len'])==len(inventory['capacity'])==3,'all original Vecs')
                need(all(type(n) is type(c) is int and 0<=n<=c for n,c in zip(inventory['len'],inventory['capacity'],strict=True)),'valid complete lengths/capacities')
            else:snapshot=entry
            need(set(snapshot)=={'c','rust','errors'} and type(snapshot['errors']) is int and snapshot['errors']==0,'no physical counter error')
            for owner in ('c','rust'):
                stats=snapshot[owner]
                need(set(stats)==FIELDS and all(type(v) is int and v>=0 for v in stats.values()),'complete integral physical fields')
                need(stats['failed']==stats['external_frees']==0 and stats['live']<=stats['peak'],'owned successful requests')
                for field in previous[owner]:
                    need(stats[field]>=previous[owner][field],'monotonic cumulative fields');previous[owner][field]=stats[field]
                if point=='retirement':need(stats['live']==0,'balanced final drop')
    need(before['wire']==after['wire'],'exact semantic/wire equality')
    violations=[]
    for point in POINTS:
        b,a=before[point],after[point]
        if point in POINTS[:3]:
            need(b[1]['len']==a[1]['len'],'same original logical writer state')
            for i in range(3):
                if a[1]['capacity'][i]>b[1]['capacity'][i]:violations.append(dict(point=point,owner='Vec'+str(i),field='capacity',before=b[1]['capacity'][i],after=a[1]['capacity'][i]))
            b,a=b[0],a[0]
        for owner in ('c','rust'):
            for field in DOMINANCE:
                if a[owner][field]>b[owner][field]:violations.append(dict(point=point,owner=owner,field=field,before=b[owner][field],after=a[owner][field]))
    return dict(complete=True,prerequisite_passed=not violations,violations=violations,
        decision='call-lifetime-subgate-only' if not violations else 'declined-call-lifetime-resource-contract',
        root_adopted=False,release_admitted=False,mtu_repaired=False,full_resource_contract_passed=False,
        CPU_qualified=False,RSS_qualified=False,allocator_page_retention_qualified=False)


def diagnostic(raw):
    marker='HC074_CONTEXT_LIFETIME:';text=raw.decode();need(text.count(marker)==1,'one complete actual observation')
    return json.loads(text.split(marker,1)[1].splitlines()[0])


def review(read):
    load=lambda n:json.loads(read(n))
    for name,digest in [('Cargo.lock','8ade63f6a4aae93f27f5161798bc87804542a48093b4203f35e25e29b680a4f5'),
      ('deny.toml','16ee708c678497358f30d772cbb843832b4e02e8f3ae4355b648620a94c44930'),
      ('docs/testing/performance/0.74/qualification-manifest.toml','11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc')]:
        need(sha(read('root/'+name))==digest,'unchanged Root/policy/qualification')
    recipe=ast.parse(read('source/current/recipe.py'));constants={}
    for node in recipe.body:
        if isinstance(node,ast.Assign) and len(node.targets)==1 and isinstance(node.targets[0],ast.Name) and node.targets[0].id in ('OLD_APPEND','CALL_APPEND'):
            constants[node.targets[0].id]=ast.literal_eval(node.value)
    need(set(constants)=={'OLD_APPEND','CALL_APPEND'},'literal current source recipe')
    old=read('fixtures/T1/src/serialize.rs');new=read('fixtures/C2/src/serialize.rs');meta=load('preparation/candidate.json')
    need(old.count(constants['OLD_APPEND'])==1 and old.replace(constants['OLD_APPEND'],constants['CALL_APPEND'])==new,'only append guard changes the functional source')
    need(sha(old)==meta['before_sha256'] and sha(new)==meta['after_sha256'],'whole actual candidate identities')
    baseline=load('preparation/baseline.json')['fixtures']
    prep=load('native-input-r1/preparation.json')
    need(set(baseline)=={'B0','T1'} and set(prep['fixtures'])=={'B0','C2'},'exact control/candidate domains')
    need(set(prep['fixtures']['C2'])==set(baseline['T1']),'same whole candidate source set')
    for name,digest in baseline['T1'].items():
        need(sha(read('fixtures/T1/'+name))==digest,'immutable failing baseline')
        if name not in ('src/serialize.rs','src/call_context.rs'):
            need(prep['fixtures']['C2'][name]==digest,'no unrelated source/dependency hypothesis')
    old_test=read('fixtures/T1/src/call_context.rs');new_test=read('fixtures/C2/src/call_context.rs')
    need(sha(old_test)==meta['old_tests_sha256'] and sha(new_test)==meta['new_tests_sha256'],'explicit test-only strengthening identities')
    def without_size_test(raw):
        start=b'fn oversized_item_is_refused_before_serialization()';end=b'#[test]\nfn zero_length_generic_item_keeps_original_behavior()'
        need(raw.count(start)==raw.count(end)==1,'one oversized sentinel scope')
        a=raw.index(start);b=raw.index(end,a);return raw[:a]+raw[b:]
    need(without_size_test(old_test)==without_size_test(new_test),'all other new diagnostic functions remain exact')
    for mode in ('control-T1','panic-T1'):
        result=load(mode+'/result.json');own=load(mode+'/execution.json')
        need(result['complete'] and result['expected_failure'] and result['failed']==1 and own['exit']==101 and own['retired'] and not own['errors'],'actual failing test controls before C2')
    for n in ('native-r1/result.json','native-r1/session-result.json'):
        value=load(n);need(value['complete'] and value['root_adopted'] is False and value['resources_qualified'] is False,'complete original carrier without promotion')
    for name in ('','source-control/','gcc-version/','ld-version/','compile-wrapper/','compile-control/','control/','compile-peek_control/','peek_control/','compiler/'):
        own=load('native-r1/'+name+'execution.json')
        need(own['exit']==0 and own['retired'] and not own['errors'] and own['cleanup_seconds']==own['capacity_poll_seconds']==5,'complete original carrier/control ownership')
    frame=load('native-frame-r1.json')
    for name,digest in frame.items():need(sha(read('native-input-r1/'+name))==digest,'whole native input frame')
    for v in ('B0','C2'):
        for name,digest in prep['fixtures'][v].items():need(sha(read('fixtures/'+v+'/'+name))==digest,'immutable original component source')
    values=[];images=[]
    for v in ('B0','C2'):
        for mode in ('build','catalog','control','compat'):
            need(load(mode+'-'+v+'/result.json')['complete'],'complete Windows controls')
        image=load('native-r1/image-'+v+'.json');images.append(image)
        rows=[json.loads(line) for line in read('native-r1/build-'+v+'/stdout.log').decode().splitlines() if line.startswith('{')]
        artifacts=[r for r in rows if r.get('reason')=='compiler-artifact' and r.get('executable') and r['target']['kind']==['lib'] and r['profile']['test']]
        need(len(artifacts)==1 and artifacts[0]['fresh'] is False and artifacts[0]['executable']==image['path'],'fresh independent actual image')
        argv=load('native-r1/build-'+v+'/intent.json')['argv'];target=argv[argv.index('--target-dir')+1]
        need(target.endswith('/builds/'+v) and image['path'].startswith(target+'/x86_64-unknown-linux-gnu/release/deps/'),'isolated target and explicit GNU linkage')
        catalog=read('native-r1/catalog-'+v+'/stdout.log').decode();names=[line[:-6] for line in catalog.splitlines() if line.endswith(': test')]
        expected=load('catalog-'+v+'/result.json')['catalog']+['serialize::native_lifetime::allocator_controls','serialize::native_lifetime::context_lifetime_observation']
        need(sorted(names)==sorted(expected) and len(names)==len(set(names))==118 and catalog.splitlines()[-1]=='118 tests, 0 benchmarks','complete original catalog plus two native cases')
        for name in ('build','catalog','check','clippy','zstd-imports','linked-zstd','controls','observe'):
            base='native-r1/'+name+'-'+v+'/'
            own=load(base+'execution.json');intent=load(base+'intent.json')
            need(own['exit']==0 and own['retired'] and not own['errors'] and own['cleanup_seconds']==own['capacity_poll_seconds']==5,'complete original native ownership')
            if name in ('controls','observe'):
                test='serialize::native_lifetime::'+('allocator_controls' if name=='controls' else 'context_lifetime_observation')
                need(intent['argv']==[image['path'],'--exact',test,'--test-threads=1','--nocapture'] and intent['image_sha256']==image['sha256'] and intent['bound']==5,'exact original observed image/case/bound')
                raw=read(base+'stdout.log').decode()
                need(re.findall(r'^test (.+?) \.\.\. ',raw,re.M)==[test] and re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',raw,re.M)==[('1','0','0','0','117')],'complete actual test accounting')
        linked=read('native-r1/linked-zstd-'+v+'/stdout.log').decode();need(re.search(r'\bcall\s+[0-9a-f]+\s+<__wrap_calloc>',linked),'actual linked context creation is observed')
        values.append(diagnostic(read('native-r1/observe-'+v+'/stdout.log')))
    need(images[0]['path']!=images[1]['path'] and images[0]['sha256']!=images[1]['sha256'],'no cached-baseline substitution')
    parent_index=read('parents/c1-index.json');need(sha(parent_index)==PARENT_SHA,'immutable original C1 packet identity')
    wrapper_row=json.loads(parent_index)['members']['source/native/wrap.c'];wrapper=read('parents/c1-wrapper.c')
    need(sha(wrapper)==wrapper_row['sha256'] and len(wrapper)==wrapper_row['bytes'],'qualified original C counter source')
    need(read('native-input-r1/wrap.c')==wrapper+b'\n'+read('source/current/peek.c'),'unchanged C observer plus qualified read-only peek')
    parent_row=json.loads(parent_index)['members']['analysis-r5-final/result.json'];parent_body=read('parents/c1-analysis.json')
    need(sha(parent_body)==parent_row['sha256'] and len(parent_body)==parent_row['bytes'],'whole original stateless observation')
    need(values[0]==json.loads(parent_body)['observations']['B0'],'disabled common hook does not perturb this baseline allocation/wire observation')
    result=reconcile(*values);result['observations']=dict(zip(('B0','C2'),values,strict=True));return result


def live_read(name):
    if name.startswith('source/current/'):return (ROOT/'tools/chitchat-call-context-074'/name.removeprefix('source/current/')).read_bytes()
    if name.startswith('root/'):return (ROOT/name.removeprefix('root/')).read_bytes()
    if name=='parents/c1-index.json':return (PARENT/'index.json').read_bytes()
    parents={'parents/c1-analysis.json':'analysis-r5-final/result.json','parents/c1-catalog.json':'catalog-B0/result.json','parents/c1-wrapper.c':'source/native/wrap.c'}
    if name in parents:
        row=json.loads((PARENT/'index.json').read_bytes())['members'][parents[name]];return (PARENT/'objects'/row['sha256']).read_bytes()
    return (BASE/name).read_bytes()


def main():
    out=BASE/'analysis';out.mkdir(exist_ok=False)
    (out/'producer.py').write_bytes(Path(__file__).read_bytes())
    try:result=review(live_read)
    except BaseException as error:result=dict(complete=False,first_refusal=dict(type=type(error).__name__,reason=str(error),traceback=traceback.format_exc()),root_adopted=False)
    (out/'result.json').write_text(json.dumps(result,indent=2,sort_keys=True)+'\n',encoding='utf8',newline='\n')
    print(json.dumps({k:v for k,v in result.items() if k!='observations'}));raise SystemExit(0 if result['complete'] else 1)


if __name__=='__main__':main()
