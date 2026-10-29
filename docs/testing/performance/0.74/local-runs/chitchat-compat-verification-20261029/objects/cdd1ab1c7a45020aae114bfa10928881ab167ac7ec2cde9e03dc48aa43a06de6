"""Strict offline component accounting; distinguishes caller applicability."""
import json
import common as c

SCHEMA={'malloc_calls','calloc_calls','realloc_calls','free_calls','allocs','requested','failed','external_frees','live','peak'}
FIELDS={'logical':('allocs','reallocs','requested','peak'),
        'physical':('allocs','realloc_calls','requested','peak'),'c':('allocs','realloc_calls','requested','peak')}
CASES=[
    ('immediate-exact-tail',64,[('Mixed',167)]),
    ('continue-roomy',64,[('Mixed',167),('Tiny',65507)]),
    ('cheap-refusal-continue',64,[('Mixed',167),({'Noise':4096},1000),('Tiny',65507)]),
    ('effectful-refusal-continue',64,[('Mixed',167),({'Noise':4096},4269),('Tiny',65507)]),
    ('continue-tight-twice',64,[('Mixed',167),('Tiny',173),('Tiny',176)]),
    ('repeated-cheap-refusal',64,[('Mixed',167),({'Noise':4096},1000),({'Noise':4096},1000),({'Noise':4096},1000),('Tiny',65507)]),
    ('accepted-multiblock',1000,[('Node',1000),({'Ascii':4096},4109),('Tiny',65507)]),
    ('small-raw-tail',64,[({'Noise':35},1000),('Tiny',65507)]),
    ('effectful-refusal-no-cache',16,[({'Noise':35},1000),({'Noise':129},177),('Tiny',65507)]),
]


def payload(kind):
    def noise(n,ascii=False):
        seed=0x0741028;out=[];mask=(1<<64)-1
        for _ in range(n):
            seed^=(seed<<13)&mask;seed^=seed>>7;seed^=(seed<<17)&mask
            out.append(32+seed%95 if ascii else seed&255)
        return bytes(out)
    if kind=='Mixed':return noise(128)+b'r'*32
    if kind=='Tiny':return bytes([17,18,19])
    if kind=='Node':return bytes([4,0])+b'node'
    if isinstance(kind,dict) and set(kind)=={'Noise'}:return noise(kind['Noise'])
    if isinstance(kind,dict) and set(kind)=={'Ascii'}:return noise(kind['Ascii'],True)
    raise ValueError('unknown payload recipe')


def check(value,variant):
    c.need(value['variant']==variant and type(value['seed']) is int and value['seed']==0x0741028,'fixed variant/seed')
    c.need(value['scope']=='linked-default-C-malloc-family; Rust System explicitly tagged' and value['CPU_qualified'] is False and value['RSS_qualified'] is False,'fixed unpromoted scope')
    c.need(len(value['rows'])==len(CASES),'complete fixed corpus')
    for row,(name,threshold,steps) in zip(value['rows'],CASES,strict=True):
        c.need(row['name']==name and row['threshold']==threshold and row['steps']==[list(s) for s in steps],'exact ordered workload/budgets')
        c.need(row['inputs']==[payload(s[0]).hex() for s in steps],'exact payload bytes')
        c.need(len(row['records'])==2 and row['records'][0]==row['records'][1],'exact inner repetition')
        r=row['records'][0];c.need(len(r['decisions'])==len(steps) and all(type(n) is bool for n in r['decisions']),'complete admission types')
        c.need(isinstance(r['wire'],str) and bytes.fromhex(r['wire']).hex()==r['wire'],'complete canonical wire')
        c.need(type(r['compressions']) is int and r['compressions']>=0 and r['writer_stack_bytes']==(96 if variant=='Q' else 80),'complete call/layout fields')
        rust=r['meter']['rust'];c.need(set(rust)=={'allocs','reallocs','requested','live','peak'} and all(type(v) is int and v>=0 for v in rust.values()) and rust['live']==0,'complete balanced logical counters')
        native=r['meter']['native'];c.need(type(native['errors']) is int and native['errors']==0,'native observer integrity')
        for scope in ('rust','c'):
            stats=native[scope]
            c.need(set(stats)==SCHEMA and all(type(v) is int and v>=0 for v in stats.values()),'complete integral native counters')
            c.need(stats['live']==stats['failed']==stats['external_frees']==0,'successful whole-region ownership')
        c.need(len(r['states'])==len(steps)+1,'complete checkpoint inventory')
        for state in r['states']:
            for key in ('len','capacity'):c.need(len(state[key])==3 and all(type(n) is int and n>=0 for n in state[key]),'complete buffer inventory')
            c.need(all(n<=cap for n,cap in zip(state['len'],state['capacity'],strict=True)),'length bounded by capacity')
            n=state['cache'];c.need(n is None or variant=='Q' and type(n) is int and 0<n<=state['len'][2] and state['len'][1]<=threshold,'valid optional cache')


def reconcile(values):
    order=('B','P','Q','Q','P','B');c.need(len(values)==6,'one fixed B/P/Q/Q/P/B cohort')
    for value,variant in zip(values,order,strict=True):check(value,variant)
    c.need(values[0]==values[5] and values[1]==values[4] and values[2]==values[3],'same-image cross-process repetition')
    source={k:values[i] for k,i in (('B',0),('P',1),('Q',2))};changes=[];regressions=[];retention=[];excluded=[];comparisons={}
    for peer in ('B','P'):
        comparable=0
        for before,after in zip(source[peer]['rows'],source['Q']['rows'],strict=True):
            b=before['records'][0];a=after['records'][0]
            same=(b['decisions'],b['wire'])==(a['decisions'],a['wire'])
            if not same:
                c.need(peer=='B' and after['name'] in ('effectful-refusal-continue','effectful-refusal-no-cache'),'safe P/Q semantics must agree')
                excluded.append(dict(peer=peer,name=after['name'],reason='unsafe B admission; incomparable work'));continue
            comparable+=1
            for scope,fields in FIELDS.items():
                select=lambda r:r['meter']['rust'] if scope=='logical' else r['meter']['native']['rust' if scope=='physical' else 'c']
                delta={k:select(a)[k]-select(b)[k] for k in fields}
                if any(delta.values()):
                    row=dict(peer=peer,name=after['name'],scope=scope,delta=delta);changes.append(row)
                    if any(v>0 for v in delta.values()):regressions.append(row)
            for checkpoint,(prior,actual) in enumerate(zip(b['states'],a['states'],strict=True)):
                total_before=sum(prior['capacity']);total_after=sum(actual['capacity'])
                if total_after>total_before:retention.append(dict(peer=peer,name=after['name'],checkpoint=checkpoint,before=total_before,after=total_after))
        comparisons[peer]=comparable
    c.need(len(excluded)==2,'exact known B semantic exclusions')
    passed=not regressions and not retention
    return dict(complete=True,component_resource_gate_pass=passed,comparisons=comparisons,
        allocation_changes=changes,allocation_regressions=regressions,buffer_capacity_regressions=retention,
        excluded_semantics=excluded,processes=6,scenarios=9,observations_per_scenario_per_image=4,
        integration_decision='component-subgate-only' if passed else 'blocked-general-stream-resource-gate',
        fixed_MTU_caller_qualified=False,production_regression_proved=False,CPU_qualified=False,RSS_qualified=False,
        allocator_retention_qualified=False,MSRV_qualified=False,Windows_qualified=False,root_adopted=False,release_admitted=False)


def evidence():
    values=[];flags=[];frames=[]
    for i,variant in enumerate(('B','P','Q','Q','P','B'),1):
        folder=c.BASE/f'cohort-{i}';c.need(json.loads((folder/'result.json').read_bytes())['complete'],'first complete cohort process')
        call=json.loads((folder/'native/observation/intent.json').read_bytes());owner=json.loads((folder/'native/observation/execution.json').read_bytes())
        image=json.loads((c.BASE/f'build-{variant}/native/artifact.json').read_bytes())
        c.need(call['argv'][0]==image['path'] and owner['exit']==0 and owner['retired'] and not owner['errors'],'actual retired image')
        controls=json.loads((c.BASE/f'control-{variant}/native/controls.json').read_bytes())
        c.need(controls['complete'] and controls['image']==image and len(controls['cases'])==6,'six controls on actual image')
        raw=(folder/'native/observation/stdout.log').read_text(encoding='utf8');value=c.diagnostic(raw,'HC074_STATEFUL:')
        c.need(value==json.loads((folder/'native/observations.json').read_bytes()),'raw full-body reconciliation')
        catalog=json.loads((c.BASE/f'build-{variant}/native/catalog.json').read_bytes())
        c.need(f'1 passed; 0 failed; 0 ignored; 0 measured; {len(catalog)-1} filtered out;' in raw,'complete selected test accounting')
        values.append(value);frames.append(call['frame']);flags.append(json.loads((c.BASE/f'build-{variant}/native/flags.json').read_bytes()))
    frozen=json.loads((c.BASE/'format/result.json').read_bytes())
    c.need(len(set(frames))==1 and frames[0]==c.sha((c.BASE/'frame.json').read_bytes())==frozen['frame_sha256'],'one externally held source frame')
    c.need(c.sha((c.BASE/'source.zip').read_bytes())==frozen['zip_sha256'] and flags.count(flags[0])==6,'exact capsule/link object/flags')
    return values


if __name__=='__main__':
    out=c.BASE/'analysis';out.mkdir(exist_ok=False);(out/'producer.py').write_bytes(__import__('pathlib').Path(__file__).read_bytes())
    result=reconcile(evidence());c.write(out,'result.json',result)
    print(json.dumps({k:v for k,v in result.items() if not isinstance(v,list)}))
