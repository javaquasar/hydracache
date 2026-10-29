"""Independent complete native cohort accounting; no timing or retained claim."""
import json

import common as c

SCOPE='linked-default-C-malloc-family; Rust System explicitly tagged'
NATIVE_FIELDS={'malloc_calls','calloc_calls','realloc_calls','free_calls','allocs','requested','failed','external_frees','live','peak'}
DOMINANCE=('allocs','realloc_calls','requested','peak')


def reconcile(values):
    c.need(len(values)==4,'exact B/P/P/B cohort')
    for value in values:
        c.need(value['seed']==0x0741028 and value['scope']==SCOPE,'fixed seed and observer scope')
        c.need(value['CPU_qualified'] is False and value['allocator_retention_qualified'] is False,'no scope promotion')
        c.semantics(value['rows'])
        for row in value['rows']:
            c.need(type(row['mtu']) is int and type(row['value_len']) is int and type(row['accepted']) is bool and isinstance(row['wire'],str),'exact workload field types')
            c.need(len(row['records'])==2 and row['records'][0]==row['records'][1],'exact within-process repetition')
            record=row['records'][0]
            c.need(type(record['native']['errors']) is int and record['native']['errors']==0,'no native observer error')
            for owner in ('c','rust'):
                stats=record['native'][owner]
                c.need(set(stats)==NATIVE_FIELDS and all(type(v) is int and v>=0 for v in stats.values()),'complete nonnegative integral native counters')
                c.need(stats['live']==stats['external_frees']==stats['failed']==0,'balanced successful in-region ownership')
            logical=record['rust']
            c.need(set(logical)=={'allocs','reallocs','requested','live','peak'} and all(type(v) is int and v>=0 for v in logical.values()) and logical['live']==0,'balanced complete logical Rust requests')
            c.need(len(record['actual_calls'])==4 and all(type(v) is int and v>=0 for v in record['actual_calls']),'complete actual-path counters')
    c.need(values[0]==values[3] and values[1]==values[2],'same-image cross-process repetition')
    changes=[];logical_changes=[];physical_rust_changes=[]
    for before,after in zip(values[0]['rows'],values[1]['rows'],strict=True):
        c.need(all(before[k]==after[k] for k in ('mtu','value_len','accepted','wire')),'unchanged workload/admissions/exact wire')
        b=before['records'][0];a=after['records'][0]
        delta={k:a['native']['c'][k]-b['native']['c'][k] for k in sorted(NATIVE_FIELDS)}
        if any(delta.values()):changes.append(dict(mtu=after['mtu'],value_len=after['value_len'],accepted=after['accepted'],delta=delta,before=b['native']['c'],after=a['native']['c']))
        if a['rust']!=b['rust']:logical_changes.append(dict(mtu=after['mtu'],value_len=after['value_len']))
        if a['native']['rust']!=b['native']['rust']:physical_rust_changes.append(dict(mtu=after['mtu'],value_len=after['value_len']))
    regressions=[v for v in changes if any(v['delta'][k]>0 for k in DOMINANCE)]
    return dict(complete=True,cases=239,processes=4,per_image_observations_per_case=4,
        native_C_dominance_pass=not regressions,native_C_regressions=len(regressions),
        accepted_regressions=sum(v['accepted'] for v in regressions),rejected_regressions=sum(not v['accepted'] for v in regressions),
        native_C_changes=changes,logical_Rust_changes=logical_changes,physical_tagged_Rust_changes=physical_rust_changes,
        integration_decision='declined-native-C-cost' if regressions else 'native-C-subgate-only',
        CPU_qualified=False,RSS_qualified=False,retained_memory_qualified=False,root_adopted=False,release_admitted=False)


def main():
    out=c.BASE/'analysis';out.mkdir(exist_ok=False);(out/'producer.py').write_bytes(__import__('pathlib').Path(__file__).read_bytes())
    values=[];flags=[];frames=[]
    for index,variant in enumerate(('B','P','P','B'),1):
        folder=c.BASE/f'cohort-{index}';c.need(json.loads((folder/'result.json').read_bytes())['complete'],'complete first cohort process')
        image=json.loads((c.BASE/f'build-{variant}/native/artifact.json').read_bytes())
        invocation=json.loads((folder/'native/observation/intent.json').read_bytes());frames.append(invocation['frame'])
        c.need(invocation['argv'][0]==image['path'],'bound actual image')
        owned=json.loads((folder/'native/observation/execution.json').read_bytes())
        c.need(owned['exit']==0 and owned['retired'] and not owned['errors'],'complete native owner')
        raw=(folder/'native/observation/stdout.log').read_text(encoding='utf8')
        value=c.diagnostic(raw,'HC074_NATIVE_ALLOC:')
        c.need(value==json.loads((folder/'native/observations.json').read_bytes()),'raw full observation reconciliation')
        values.append(value);flags.append(json.loads((c.BASE/f'build-{variant}/native/flags.json').read_bytes()))
    c.need(len(set(frames))==1 and flags.count(flags[0])==4,'same source frame/object/linkage flags')
    result=reconcile(values);c.write(out,'result.json',result)
    print(json.dumps({k:v for k,v in result.items() if k not in ('native_C_changes','logical_Rust_changes','physical_tagged_Rust_changes')}))


if __name__=='__main__':main()
