"""Independent offline body/dominance review, without executing old producers."""
import ast,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];BASE=ROOT/'target/chitchat-frame-20261030'
PARENT=ROOT/'docs/testing/performance/0.74/local-runs/chitchat-fused-20261030'
PARENT_SHA='e554bd8ee5bd7b4f0cc2963393ccbea922e93663f294962446d1caa651034403'
FIELDS={'logical':('allocs','reallocs','requested','peak'),'physical':('allocs','realloc_calls','requested','peak'),'c':('allocs','realloc_calls','requested','peak')}
NATIVE={'malloc_calls','calloc_calls','realloc_calls','free_calls','allocs','requested','failed','external_frees','live','peak'}
sha=lambda raw:hashlib.sha256(raw).hexdigest()
def need(ok,why):
    if not ok:raise ValueError(why)
def diagnostic(raw,marker):
    rows=[s.split(marker,1)[1] for s in raw.decode().splitlines() if marker in s]
    need(len(rows)==1,'one complete raw diagnostic');return json.loads(rows[0])
def reference(name):
    raw=(PARENT/'index.json').read_bytes();need(sha(raw)==PARENT_SHA,'immutable F negative reference')
    row=json.loads(raw)['members'][name];raw=(PARENT/'objects'/row['sha256']).read_bytes();need(sha(raw)==row['sha256'] and len(raw)==row['bytes'],'whole reference object');return raw

def windows():
    original=reference('reference/compat-B0/stdout.log');count=0
    peers=[]
    for suffix in ('','-188'):
        for mode in ('build','catalog','check','clippy','compat','old0','old1',*['case'+str(i) for i in range(10)],*['frame'+str(i) for i in range(4)]):
            folder=BASE/(mode+'-G'+suffix);result=json.loads((folder/'result.json').read_bytes());owner=json.loads((folder/'execution.json').read_bytes())
            need(result['complete'] and owner['exit']==0 and owner['retired'] and not owner['errors'],'complete first Windows phase')
            need(owner['capacity_poll_seconds']==owner['cleanup_seconds']==5,'original Windows ownership bounds')
            count+=result.get('passed',0)
        peers.append((BASE/('compat-G'+suffix)/'stdout.log').read_bytes())
    for marker,fields,n in [('HC074_SUPPORTED_DELTA:',('accepted','actual','mtu','remaining','value_len','wire'),239),
      ('HC074_STREAM_BOUNDS:',('actual','id','payload','remaining','threshold','wire'),37)]:
        values=[diagnostic(s,marker) for s in [original]+peers]
        need(values[1]==values[2],'whole final Windows compiler bodies agree')
        need(all(len(v['cases'])==n and v['seed']==7606312 for v in values),'complete original corpus/seed')
        projected=[[[r[k] for k in fields] for r in v['cases']] for v in values]
        need(projected[0]==projected[1]==projected[2],'original payload/admission/wire/cursor')
    malformed=[diagnostic(s,'HC074_SECURITY_MALFORMED:') for s in [original]+peers]
    need(malformed[0]==malformed[1]==malformed[2] and len(malformed[0]['rows'])==7,'all malformed/cursor cases')
    return dict(complete=True,selected_positive_executions=count,scope='Windows1.94/1.88 functional, excludes first retained observation',full_resource_gate_pass=False,root_adopted=False)

def meter(record,kind):
    m=record if kind=='operation' else record['meter']
    logical=m['rust'];native=m['native']
    need(set(logical)=={'allocs','reallocs','requested','live','peak'} and all(type(v) is int and v>=0 for v in logical.values()) and logical['live']==0,'balanced whole logical Rust')
    need(type(native['errors']) is int and native['errors']==0,'native observer integrity')
    for owner in ('rust','c'):
        stat=native[owner];need(set(stat)==NATIVE and all(type(v) is int and v>=0 for v in stat.values()),'all native fields')
        need(stat['live']==stat['failed']==stat['external_frees']==0,'balanced whole physical/native ownership')
    need(type(m['c_free_checkpoints']) is int and m['c_free_checkpoints']>=3,'qualified no-owned-C checkpoints executed')
    return dict(logical=logical,physical=native['rust'],c=native['c'])

def payload(kind):
    if kind=='Mixed':return noise(128)+b'r'*32
    if kind=='Tiny':return bytes([17,18,19])
    if kind=='Node':return bytes([4,0])+b'node'
    if isinstance(kind,dict) and set(kind)=={'Noise'}:return noise(kind['Noise'])
    if isinstance(kind,dict) and set(kind)=={'Ascii'}:return noise(kind['Ascii'],True)
    raise ValueError('fixed payload recipe')
def noise(n,ascii=False):
    seed=7606312;out=[];mask=(1<<64)-1
    for _ in range(n):
        seed^=(seed<<13)&mask;seed^=seed>>7;seed^=(seed<<17)&mask;out.append(32+seed%95 if ascii else seed&255)
    return bytes(out)

def review(values):
    order=('B','P','G','G','P','B');need(len(values)==6,'one predeclared complete B/P/G/G/P/B cohort')
    tree=ast.parse((ROOT/'tools/chitchat-stateful-074/analyze.py').read_bytes())
    cases=next(ast.literal_eval(n.value) for n in tree.body if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='CASES' for t in n.targets))
    table_raw=(ROOT/'tools/chitchat-caller-074/cases-r2.json').read_bytes();need(sha(table_raw)=='4a3308a82d014fb58df808a9f3984922537a80c02219610cdf5ee6fa1ec4a1f9','exact caller table')
    callers=json.loads(table_raw)['cases'];original=diagnostic(reference('reference/compat-B0/stdout.log'),'HC074_SUPPORTED_DELTA:')['cases']
    for cell,v in zip(values,order,strict=True):
        for kind,n in [('operation',239),('stateful',9),('caller',30)]:
            value=cell[kind];need(value['seed']==7606312 and len(value['rows'])==n,'complete corpus/seed')
            if kind!='operation':need(value['variant']==v and value['CPU_qualified'] is False and value['RSS_qualified'] is False,'actual runtime variant and no timing promotion')
            else:need(value['CPU_qualified'] is False and value['allocator_retention_qualified'] is False,'native allocation scope')
            for i,row in enumerate(value['rows']):
                need(len(row['records'])==2 and row['records'][0]==row['records'][1],'both inner observations')
                record=row['records'][0];meter(record,kind)
                if kind=='operation':
                    need(all(row[k]==original[i][k] for k in ('mtu','value_len','accepted','wire')),'all original239 complete work/wire')
                    need(record['c_free_checkpoints']==4,'all constructor/node/kv/return C checks')
                else:
                    if kind=='stateful':
                        name,t,steps=cases[i];need(row['name']==name and row['threshold']==t and row['steps']==[list(s) for s in steps],'all original ordered generic workload/budgets')
                        need(row['inputs']==[payload(s[0]).hex() for s in steps],'original complete payload bytes')
                    else:need(row['input']==callers[i],'all original actual caller requests/MTU')
                    need(all(type(v) is bool for v in record['decisions']),'complete boolean admissions')
                    need(isinstance(record['wire'],str) and bytes.fromhex(record['wire']).hex()==record['wire'],'complete canonical wire')
                    need(len(record['states'])==len(record['decisions'])+1 and record['meter']['c_free_checkpoints']==len(record['decisions'])+2,'every constructor/append/return retained checkpoint')
                    if kind=='caller':need(all(record['decisions'][:-1]) and (not record['decisions'][-1] or len(record['decisions'])==len(row['input']['requests'])),'actual caller stops at first false without omitted suffix')
                    need(type(record['finish_capacity']) is int and record['finish_capacity']>=len(bytes.fromhex(record['wire'])),'returned Vec capacity')
                    for state in record['states']:
                        need(len(state['len'])==len(state['capacity'])==3 and all(type(x) is type(cap) is int and 0<=x<=cap for x,cap in zip(state['len'],state['capacity'],strict=True)),'all per-buffer inventories')
                        cache=state['cache'];need(cache is None or v=='G' and type(cache) is int and 0<cache<=state['len'][2],'valid exact G tail cache')
    need(values[0]==values[5] and values[1]==values[4] and values[2]==values[3],'same-image cross-process repeat')
    source={v:values[i] for v,i in [('B',0),('P',1),('G',2)]};regressions=[];excluded=[];comparisons={}
    for peer in ('B','P'):
        for kind in ('operation','stateful','caller'):
            comparable=0
            for i,(before,after) in enumerate(zip(source[peer][kind]['rows'],source['G'][kind]['rows'],strict=True)):
                b,g=before['records'][0],after['records'][0]
                semantic=(before['accepted'],before['wire'])==(after['accepted'],after['wire']) if kind=='operation' else (b['decisions'],b['wire'])==(g['decisions'],g['wire'])
                if not semantic:
                    need(peer=='B' and kind!='operation','only old unsafe B may differ')
                    if kind=='stateful':need(before['name'] in ('effectful-refusal-continue','effectful-refusal-no-cache'),'only fixed two unsafe generic B cases')
                    else:need(len(bytes.fromhex(b['wire']))>before['input']['mtu'],'demonstrated unsafe B caller')
                    excluded.append(dict(peer=peer,kind=kind,row=i));continue
                comparable+=1;old=meter(b,kind);new=meter(g,kind)
                for scope,fields in FIELDS.items():
                    for field in fields:
                        if new[scope][field]>old[scope][field]:regressions.append(dict(peer=peer,kind=kind,row=i,scope=scope,field=field,before=old[scope][field],after=new[scope][field]))
                if kind!='operation':
                    for checkpoint,(a,z) in enumerate(zip(b['states'],g['states'],strict=True)):
                        for buffer,(x,y) in enumerate(zip(a['capacity'],z['capacity'],strict=True)):
                            if y>x:regressions.append(dict(peer=peer,kind=kind,row=i,checkpoint=checkpoint,buffer=buffer,before=x,after=y))
                    if g['finish_capacity']>b['finish_capacity']:regressions.append(dict(peer=peer,kind=kind,row=i,checkpoint='finish',before=b['finish_capacity'],after=g['finish_capacity']))
                comparisons[peer+'/'+kind]=comparable
    return dict(complete=True,allocation_retention_gate_pass=not regressions,regressions=regressions,comparisons=comparisons,excluded_unsafe_B=excluded,
      order=list(order),cases=[239,9,30],observations_per_case_per_image=4,CPU_qualified=False,RSS_qualified=False,allocator_page_retention_qualified=False,stack_qualified=False,root_adopted=False,release_admitted=False)

def load_native():
    need(json.loads((BASE/'native-observe-r4/result.json').read_bytes())['complete'] and json.loads((BASE/'native-observe-r4/session-result.json').read_bytes())['complete'],'complete first R4 native observation envelope')
    values=[]
    for i,v in enumerate(('B','P','G','G','P','B'),1):
        cell={}
        for kind,marker in [('operation','HC074_NATIVE_ALLOC:'),('stateful','HC074_STATEFUL:'),('caller','HC074_CALLER:')]:
            folder=BASE/'native-observe-r4'/f'observe-{i}-{v}-{kind}';owner=json.loads((folder/'execution.json').read_bytes());intent=json.loads((folder/'intent.json').read_bytes())
            image=json.loads((BASE/'native-state-r2'/('image-'+v+'194.json')).read_bytes())
            test={'operation':'delta::cost_review::native_operation_observations','stateful':'serialize::stateful_review::stateful_observations','caller':'delta::cost_review::actual_caller::fixed_caller_observations'}[kind]
            need(owner['exit']==0 and owner['retired'] and not owner['errors'] and intent['argv']==[image['path'],'--exact',test,'--test-threads=1','--nocapture'] and owner['bound_seconds']==intent['bound']==5,'first actual held native observation')
            need(intent['image_sha256']==image['sha256'] and owner['capacity_poll_seconds']==owner['cleanup_seconds']==5,'held image and original poll/retirement')
            raw=(folder/'stdout.log').read_bytes();names=json.loads((BASE/'native-state-r2'/('catalog-'+v+'194.json')).read_bytes())
            need(f'1 passed; 0 failed; 0 ignored; 0 measured; {len(names)-1} filtered out;'.encode() in raw,'complete selected case accounting')
            cell[kind]=diagnostic(raw,marker)
        values.append(cell)
    return values

def native_semantics(read):
    """Reconcile the executed selections and full diagnostic bodies as data."""
    total=0;observed=[]
    for compiler,keys in [('194',('B194','P194','G194')),('188',('G188',))]:
        phase='native-semantic'+compiler+'-r3/'
        need(json.loads(read(phase+'result.json'))['complete'] and json.loads(read(phase+'session-result.json'))['complete'],'complete first semantic envelopes')
        for key in keys:
            selection=json.loads(read(phase+'selection-'+key+'.json'))
            selected=selection['selected'];need(len(selected)==(95 if key.startswith('G') else 8) and len(set(selected))==len(selected),'whole selected semantic catalog')
            names=json.loads(read('native-state-r2/catalog-'+key+'.json'))
            need(set(selected).isdisjoint(selection['unexecuted']) and set(selected)|set(selection['unexecuted'])==set(names),'complete executed/unexecuted partition')
            for i,test in enumerate(selected):
                folder=phase+('control-'+key+f'-{i:02}' if i<8 else 'case-'+key+f'-{i-8:03}')+'/'
                intent=json.loads(read(folder+'intent.json'));own=json.loads(read(folder+'execution.json'))
                need(intent['argv'][1:]==['--exact',test,'--test-threads=1','--nocapture'] and own['bound_seconds']==5 and own['exit']==0 and own['retired'] and not own['errors'],'exact original retired semantic execution')
                raw=read(folder+'stdout.log');need(f'1 passed; 0 failed; 0 ignored; 0 measured; {len(names)-1} filtered out;'.encode() in raw,'complete semantic result accounting')
                if test in ('serialize::security_stream_review::observe_streams','delta::security_delta_review::supported_delta_operations'):
                    marker='HC074_STREAM_BOUNDS:' if test.startswith('serialize') else 'HC074_SUPPORTED_DELTA:'
                    value=diagnostic(raw,marker)
                    expected=diagnostic(read('compat-G'+('-188' if compiler=='188' else '')+'/stdout.log'),marker)
                    need(value==expected,'complete GNU/Windows actual diagnostic body')
                    observed.append(key+'/'+marker)
                total+=1
    need(total==206 and len(observed)==4,'all native selected executions/diagnostics')
    return dict(complete=True,selected_positive_executions=total,full_library_passed=False,complete_native_windows_bodies=True)

def native_builds(read):
    paths=[]
    root='/home/artur/.cache/hydracache-074-frame/20261030-r2'
    for key in ('B194','P194','G194','G188'):
        prefix='native-build-r2/build-'+key+'/'
        owner=json.loads(read(prefix+'execution.json'));intent=json.loads(read(prefix+'intent.json'))
        need(owner['exit']==0 and owner['retired'] and not owner['errors'] and owner['bound_seconds']==1200,'complete original native build owner')
        argv=intent['argv'];need(argv[1]=='test' and all(s in argv for s in ('--offline','--locked','--release','--all-features','--lib','--no-run','--message-format=json')),'original build contract')
        need(argv[argv.index('--manifest-path')+1]==root+'/fixtures/'+key[0]+'/Cargo.toml' and argv[argv.index('--target-dir')+1]==root+'/builds/'+key,'independent actual source/target')
        rows=[json.loads(s) for s in read(prefix+'stdout.log').decode().splitlines() if s.startswith('{')]
        actual=[r for r in rows if r.get('reason')=='compiler-artifact' and r.get('executable') and r['target']['kind']==['lib'] and r['profile']['test']]
        need(len(actual)==1 and actual[0]['fresh'] is False and [r['success'] for r in rows if r.get('reason')=='build-finished']==[True],'fresh exact native library')
        image=json.loads(read('native-state-r2/image-'+key+'.json'))
        need(image['path']==actual[0]['executable'] and actual[0]['target']['src_path']==root+'/fixtures/'+key[0]+'/src/lib.rs' and image['path'].startswith(root+'/builds/'+key+'/'),'whole actual native image identity')
        paths.append(image['path'])
    need(len(set(paths))==4,'four independent images');return dict(complete=True,fresh_images=4)

if __name__=='__main__':
    import sys
    action=sys.argv[1];need(action in ('windows','semantic','native'),'fixed offline review')
    result=windows() if action=='windows' else native_semantics(lambda name:(BASE/name).read_bytes()) if action=='semantic' else review(load_native())
    folder=BASE/'analysis';folder.mkdir(exist_ok=True)
    with (folder/(action+'.json')).open('x',encoding='utf8',newline='\n') as f:json.dump(result,f,indent=2,sort_keys=True)
    if action=='native' and not result['allocation_retention_gate_pass']:
        with (BASE/'STOP.json').open('x',encoding='utf8') as f:json.dump(dict(reason='first allocation/retention regression',root_adopted=False),f)
    print(json.dumps({k:v for k,v in result.items() if k!='regressions'}))
