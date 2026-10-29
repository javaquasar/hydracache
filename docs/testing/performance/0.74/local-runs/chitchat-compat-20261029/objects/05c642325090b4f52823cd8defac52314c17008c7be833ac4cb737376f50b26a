"""Reconcile raw B/Q/Q/B bodies and every predeclared allocation field."""
from pathlib import Path
import json
import common as c

old=c.load(c.ROOT/'tools/chitchat-native-cost-074/analyze.py','native_cost_accounting')
old.c=c
FIELDS={'logical':('allocs','reallocs','requested','peak'),
        'physical':('allocs','realloc_calls','requested','peak'),
        'c':('allocs','realloc_calls','requested','peak')}


def reconcile(values):
    result=old.reconcile(values)
    changes=[];regressions=[]
    for before,after in zip(values[0]['rows'],values[1]['rows'],strict=True):
        b=before['records'][0];a=after['records'][0]
        for scope in FIELDS:
            select=lambda r:r['rust'] if scope=='logical' else r['native']['rust' if scope=='physical' else 'c']
            prior=select(b);actual=select(a)
            delta={k:actual[k]-prior[k] for k in FIELDS[scope]}
            if any(delta.values()):
                value=dict(mtu=after['mtu'],value_len=after['value_len'],accepted=after['accepted'],scope=scope,delta=delta)
                changes.append(value)
                if any(n>0 for n in delta.values()):regressions.append(value)
    result.update(process_order=['B','Q','Q','B'],allocation_dominance_pass=not regressions,
        allocation_changes=changes,allocation_regressions=regressions,
        integration_decision='allocation-subgate-only' if not regressions else 'declined-allocation-cost',
        stateful_retention_qualified=False,MSRV_qualified=False,Windows_qualified=False)
    return result


def evidence():
    values=[];frames=[];flags=[]
    for index,variant in enumerate(('B','Q','Q','B'),1):
        folder=c.BASE/f'cohort-{index}'
        c.need(json.loads((folder/'result.json').read_bytes())['complete'],'first complete cohort')
        image=json.loads((c.BASE/f'build-{variant}/native/artifact.json').read_bytes())
        call=json.loads((folder/'native/observation/intent.json').read_bytes())
        owner=json.loads((folder/'native/observation/execution.json').read_bytes())
        c.need(call['argv'][0]==image['path'] and owner['exit']==0 and owner['retired'] and not owner['errors'],'actual retired image owner')
        raw=(folder/'native/observation/stdout.log').read_text(encoding='utf8')
        value=c.diagnostic(raw,'HC074_NATIVE_ALLOC:')
        c.need(value==json.loads((folder/'native/observations.json').read_bytes()),'raw full body equality')
        old_count=len(json.loads((c.BASE/f'build-{variant}/native/catalog.json').read_bytes()))
        c.need(f'1 passed; 0 failed; 0 ignored; 0 measured; {old_count-1} filtered out;' in raw,'actual complete test accounting')
        control=json.loads((c.BASE/f'control-{variant}/native/controls.json').read_bytes())
        c.need(control['complete'] and control['image']==image,'controls on actual image')
        values.append(value);frames.append(call['frame']);flags.append(json.loads((c.BASE/f'build-{variant}/native/flags.json').read_bytes()))
    frozen=json.loads((c.BASE/'format/result.json').read_bytes())
    c.need(len(set(frames))==1 and frames[0]==frozen['frame_sha256'] and c.sha((c.BASE/'frame.json').read_bytes())==frames[0],'one externally held actual source frame')
    c.need(c.sha((c.BASE/'source.zip').read_bytes())==frozen['zip_sha256'] and flags.count(flags[0])==4,'exact capsule/object/link flags')
    return values


def main():
    c.need(c.PROFILE=='Q','fixed Q accounting')
    out=c.BASE/'analysis';out.mkdir(exist_ok=False)
    (out/'producer.py').write_bytes(Path(__file__).read_bytes())
    result=reconcile(evidence());c.write(out,'result.json',result)
    print(json.dumps({k:v for k,v in result.items() if not isinstance(v,list) or k=='process_order'}))


if __name__=='__main__':main()
