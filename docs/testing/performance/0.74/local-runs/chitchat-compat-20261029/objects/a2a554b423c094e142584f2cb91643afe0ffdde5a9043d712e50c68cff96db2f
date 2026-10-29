"""Actual fixed-MTU body/accounting; never promotes the generic component gate."""
import json
import common as c

FIELDS={'logical':('allocs','reallocs','requested','peak'),
    'physical':('allocs','realloc_calls','requested','peak'),'c':('allocs','realloc_calls','requested','peak')}
NATIVE={'malloc_calls','calloc_calls','realloc_calls','free_calls','allocs','requested','failed','external_frees','live','peak'}


def expected():
    raw=(c.TOOLS/'cases-r2.json').read_bytes();c.need(c.sha(raw)==c.TABLE_SHA,'frozen premeasurement table');return json.loads(raw)['cases']


def check(value,variant):
    c.need(value['variant']==variant and type(value['seed']) is int and value['seed']==0x0741028,'fixed variant/seed')
    c.need(value['scope']=='actual fixed-MTU DeltaSerializer through final Delta serialization/drop' and value['CPU_qualified'] is False and value['RSS_qualified'] is False,'unpromoted actual scope')
    cases=expected();c.need(len(value['rows'])==30,'complete actual caller table')
    for row,case in zip(value['rows'],cases,strict=True):
        c.need(row['input']==case,'exact full ordered requests/MTU')
        c.need(len(row['records'])==2 and row['records'][0]==row['records'][1],'inner repetition')
        r=row['records'][0];decisions=r['decisions']
        c.need(0<len(decisions)<=len(case['requests']) and all(type(v) is bool for v in decisions),'complete actual admission types')
        c.need(all(decisions[:-1]) and (not decisions[-1] or len(decisions)==len(case['requests'])),'stop at first refusal; no omitted success suffix')
        c.need(len(r['states'])==len(decisions)+1,'complete checkpoint inventory')
        c.need(isinstance(r['wire'],str) and bytes.fromhex(r['wire']).hex()==r['wire'],'canonical full wire bytes')
        c.need(variant=='B' or len(bytes.fromhex(r['wire']))<=case['mtu'],'safe caller MTU')
        c.need(type(r['compressions']) is int and r['compressions']>0 and len(r['paths'])==4 and all(type(v) is int and v>=0 for v in r['paths']),'actual compression/path counters')
        rust=r['meter']['rust'];c.need(set(rust)=={'allocs','reallocs','requested','live','peak'} and all(type(v) is int and v>=0 for v in rust.values()) and rust['live']==0,'complete balanced logical Rust')
        native=r['meter']['native'];c.need(type(native['errors']) is int and native['errors']==0,'native meter integrity')
        for scope in ('c','rust'):
            stats=native[scope];c.need(set(stats)==NATIVE and all(type(v) is int and v>=0 for v in stats.values()),'complete integral native meter')
            c.need(stats['live']==stats['external_frees']==stats['failed']==0,'whole-region ownership balance')
        for s in r['states']:
            for key in ('len','capacity'):c.need(len(s[key])==3 and all(type(n) is int and n>=0 for n in s[key]),'complete buffer lengths/capacities')
            c.need(all(n<=cap for n,cap in zip(s['len'],s['capacity'],strict=True)),'length bounded by capacity')
            n=s['cache'];c.need(n is None or variant=='Q' and type(n) is int and 0<n<=s['len'][2],'valid cache metadata')
        if case['name'].endswith('/large-then-small'):
            c.need(len(decisions)>=2 and all(decisions[:2]),'required large admission coverage')
            if case['mtu']>32768 and variant!='B':c.need(r['paths'][1]>0,'required ambiguous branch coverage')


def reconcile(values):
    c.need(len(values)==6,'one fixed B/P/Q/Q/P/B cohort')
    for value,variant in zip(values,('B','P','Q','Q','P','B'),strict=True):check(value,variant)
    c.need(values[0]==values[5] and values[1]==values[4] and values[2]==values[3],'exact cross-process repetition')
    source={'B':values[0],'P':values[1],'Q':values[2]};regressions=[];capacity=[];changes=[];excluded=[];counts={}
    for peer in ('B','P'):
        comparable=0
        for before,after in zip(source[peer]['rows'],source['Q']['rows'],strict=True):
            b=before['records'][0];a=after['records'][0]
            if (b['decisions'],b['wire'])!=(a['decisions'],a['wire']):
                c.need(peer=='B' and len(bytes.fromhex(b['wire']))>before['input']['mtu'],'only demonstrated unsafe B work may be excluded')
                excluded.append(before['input']['name']);continue
            comparable+=1
            for scope,fields in FIELDS.items():
                select=lambda r:r['meter']['rust'] if scope=='logical' else r['meter']['native']['rust' if scope=='physical' else 'c']
                delta={k:select(a)[k]-select(b)[k] for k in fields}
                if any(delta.values()):
                    row=dict(peer=peer,name=after['input']['name'],scope=scope,delta=delta);changes.append(row)
                    if any(v>0 for v in delta.values()):regressions.append(row)
            for i,(prior,actual) in enumerate(zip(b['states'],a['states'],strict=True)):
                if sum(actual['capacity'])>sum(prior['capacity']):capacity.append(dict(peer=peer,name=after['input']['name'],checkpoint=i,before=sum(prior['capacity']),after=sum(actual['capacity'])))
        counts[peer]=comparable
    passed=not regressions and not capacity
    return dict(complete=True,fixed_caller_resource_gate_pass=passed,comparisons=counts,excluded_unsafe_B=excluded,
        allocation_changes=changes,allocation_regressions=regressions,buffer_capacity_regressions=capacity,
        table_sha256=c.TABLE_SHA,cases=30,processes=6,observations_per_case_per_image=4,
        integration_decision='fixed-caller-resource-subgate-only' if passed else 'blocked-fixed-caller-resource-gate',
        generic_component_gate_pass=False,CPU_qualified=False,RSS_qualified=False,allocator_retention_qualified=False,
        MSRV_qualified=False,Windows_qualified=False,root_adopted=False,release_admitted=False)


def evidence():
    values=[];frames=[];flags=[]
    for i,variant in enumerate(('B','P','Q','Q','P','B'),1):
        folder=c.BASE/f'cohort-{i}';c.need(json.loads((folder/'result.json').read_bytes())['complete'],'first complete cohort process')
        image=json.loads((c.BASE/f'build-{variant}/native/artifact.json').read_bytes());control=json.loads((c.BASE/f'control-{variant}/native/controls.json').read_bytes())
        c.need(control['complete'] and len(control['cases'])==8 and control['image']==image,'eight controls on actual image')
        call=json.loads((folder/'native/observation/intent.json').read_bytes());owner=json.loads((folder/'native/observation/execution.json').read_bytes())
        c.need(call['argv'][0]==image['path'] and owner['exit']==0 and owner['retired'] and not owner['errors'],'actual retired image')
        raw=(folder/'native/observation/stdout.log').read_text(encoding='utf8');value=c.diagnostic(raw,'HC074_CALLER:')
        c.need(value==json.loads((folder/'native/observations.json').read_bytes()),'raw full body equality')
        catalog=json.loads((c.BASE/f'build-{variant}/native/catalog.json').read_bytes())
        c.need(f'1 passed; 0 failed; 0 ignored; 0 measured; {len(catalog)-1} filtered out;' in raw,'complete selected execution accounting')
        values.append(value);frames.append(call['frame']);flags.append(json.loads((c.BASE/f'build-{variant}/native/flags.json').read_bytes()))
    frozen=json.loads((c.BASE/'format/result.json').read_bytes())
    c.need(len(set(frames))==1 and frames[0]==c.sha((c.BASE/'frame.json').read_bytes())==frozen['frame_sha256'],'one externally held actual source frame')
    c.need(c.sha((c.BASE/'source.zip').read_bytes())==frozen['zip_sha256'] and flags.count(flags[0])==6,'exact capsule/object/flags')
    return values


if __name__=='__main__':
    out=c.BASE/'analysis';out.mkdir(exist_ok=False);(out/'producer.py').write_bytes(__import__('pathlib').Path(__file__).read_bytes())
    result=reconcile(evidence());c.write(out,'result.json',result)
    print(json.dumps({k:v for k,v in result.items() if not isinstance(v,list)}))
