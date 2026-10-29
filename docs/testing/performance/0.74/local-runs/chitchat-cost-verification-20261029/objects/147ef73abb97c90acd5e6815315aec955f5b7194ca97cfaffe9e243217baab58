"""Strict semantic/environment validation; prespecified local cost estimates."""
import json
import math
import statistics
import common as c

FIELDS=('rss','pss','private_clean','private_dirty')
HEAP=('arena','ordblks','smblks','hblks','hblkhd','usmblks','fsmblks','uordblks','fordblks','keepcost')


def validate(row,variant,mode):
    p=c.protocol();c.need(row['variant']==variant and row['mode']==mode and row['seed']==p['seed'],'fixed identity')
    c.need(row['rows']==json.loads((c.BASE/'input/expected-caller.json').read_bytes()),'complete original full caller input/admission/wire')
    c.need(row['test_cfg'] is False and row['allocation_wrapper'] is False,'actual clean normal image')
    for key in ('cpu_repetitions','memory_epochs','warm_sweeps'):c.need(row[key]==p[key],'fixed work '+key)
    c.need(row['memory_sweeps']==p['memory_sweeps_per_epoch'],'fixed memory work')
    c.need(type(row['pid']) is int and row['pid']>0 and type(row['start_ticks']) is int and row['start_ticks']>0 and len(row['boot_id'])==36,'same lifetime identity')
    c.need(len(row['resolutions_ns'])==4 and all(type(v) is int and 0<v<=1_000_000 for v in row['resolutions_ns']),'qualified clocks')
    samples=[row['elapsed']]+row['timings']
    c.need(len(row['timings'])==(30 if mode=='cpu' else 0),'complete timing cell count')
    for stamp in samples:
        c.need(set(stamp)=={'raw','mono','boot','cpu'} and all(type(v) is int and v>=0 for v in stamp.values()),'complete clock samples')
        c.need(stamp['raw']>100*row['resolutions_ns'][0] and stamp['cpu']>100*row['resolutions_ns'][3],'positive resolved CPU/wall')
        c.need(stamp['cpu']<=stamp['raw']*1.1,'one-thread CPU/wall sanity')
    # Whole-window drift only: adjacent tiny-cell clock reads have finite skew.
    stamp=row['elapsed']
    for key in ('mono','boot'):c.need(abs(stamp[key]-stamp['raw'])<=p['monotonic_raw_drift_fraction']*stamp['raw'],'no suspend/raw-clock discrepancy')
    c.need(len(row['memory'])==7,'full memory buffer')
    if mode in ('memory','control'):
        for snap in row['memory'][:7 if mode=='memory' else 2]:
            c.need(snap['threads']==1 and all(type(snap[k]) is int and snap[k]>=0 for k in FIELDS),'single-thread whole-process memory')
            c.need(set(snap['heap'])==set(HEAP) and all(type(v) is int and v>=0 for v in snap['heap'].values()),'complete glibc telemetry')
            c.need(snap['rss']>=snap['pss']>0 and snap['heap']['arena']>=snap['heap']['uordblks'],'nonempty sane memory')


def interval(values):
    c.need(len(values)==5 and all(math.isfinite(v) and v>0 for v in values),'five complete positive block ratios')
    logs=list(map(math.log,values));mean=statistics.mean(logs);half=2.7764451051977987*statistics.stdev(logs)/math.sqrt(5)
    return dict(ratio=math.exp(mean),lower95=math.exp(mean-half),upper95=math.exp(mean+half),block_ratios=values)


def reconcile(cpu,memory):
    c.need(len(cpu)==len(memory)==30,'one full five-block cohort per mode')
    order=('B','P','Q','Q','P','B');identities=[]
    for mode,values in (('cpu',cpu),('memory',memory)):
        for i,row in enumerate(values):
            validate(row,order[i%6],mode);identities.append((row['boot_id'],row['pid'],row['start_ticks']))
    c.need(len(set(identities))==60 and len({v[0] for v in identities})==1,'60 independent processes within one boot')
    timing=[]
    for scope in ('total',*range(30)):
        for metric in ('cpu','raw'):
            ratios=[]
            for i in range(0,30,6):
                def amount(pos):return sum(v[metric] for v in cpu[pos]['timings']) if scope=='total' else cpu[pos]['timings'][scope][metric]
                ratios.append((amount(i+2)+amount(i+3))/(amount(i)+amount(i+5)))
            timing.append(dict(scope=scope,metric=metric,**interval(ratios)))
    failures=[]
    for i in range(0,30,6):
        for b,q in ((memory[i],memory[i+2]),(memory[i+5],memory[i+3])):
            for metric in (*FIELDS,'arena','hblkhd','uordblks'):
                def value(s):return s[metric] if metric in FIELDS else s['heap'][metric]
                bd=[value(s)-value(b['memory'][0]) for s in b['memory'][1:]]
                qd=[value(s)-value(q['memory'][0]) for s in q['memory'][1:]]
                if any(a>d for a,d in zip(qd,bd,strict=True)):
                    failures.append(dict(block=i//6+1,baseline_pid=b['pid'],candidate_pid=q['pid'],metric=metric,B_delta=bd,Q_delta=qd))
    return dict(complete=True,cpu_nonregression_gate_pass=all(r['upper95']<=1.03 for r in timing),
        timing=timing,memory_growth_screen_pass=not failures,memory_growth_refusals=failures,
        generic_component_gate_pass=False,continuous_retention_qualified=False,root_adopted=False,release_admitted=False)


def evidence(mode):
    values=[]
    for i in range(1,31):
        folder=c.BASE/(mode+'-'+str(i));c.need(json.loads((folder/'result.json').read_bytes())['complete'],'complete outer outcome')
        for name in ('execution.json','native-envelope.json','native/observation/execution.json'):
            owner=json.loads((folder/name).read_bytes());c.need(owner['exit']==0 and owner['retired'] and not owner['errors'] and owner['bound_seconds']==1200 and owner['capacity_poll_seconds']==owner['cleanup_seconds']==5,'complete original ownership')
        row=c.diagnostic((folder/'native/observation/stdout.log').read_text(),'HC074_CLEAN_COST:')
        c.need(row==json.loads((folder/'native/observation.json').read_bytes()),'raw full body');values.append(row)
    return values


if __name__=='__main__':
    folder=c.BASE/'analysis';folder.mkdir(exist_ok=False);result=reconcile(evidence('cpu'),evidence('memory'));c.write(folder,'result.json',result)
    print(json.dumps({k:v for k,v in result.items() if k not in ('timing','memory_growth_refusals')}))
