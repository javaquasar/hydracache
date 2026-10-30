"""Offline first checkpoint: no metric compensation and no re-execution."""
import json
import recipe as p
c=p.c

def reconcile(values):
    b,g=[values[v]['observation'] for v in ('B','G')]
    c.need(all(v['complete'] for v in values.values()),'complete first cases')
    for key in ('seed','name','threshold','budget','payload','scope','CPU_qualified','RSS_qualified'):
        c.need(b[key]==g[key],'same complete original work/scope')
    c.need((b['seed'],b['threshold'],b['budget'],b['name'])==(7606312,64,167,'immediate-exact-tail'),'frozen original input')
    c.need(len(b['records'])==len(g['records'])==2 and b['records'][0]==b['records'][1] and g['records'][0]==g['records'][1],'both predeclared observations')
    before,after=b['records'][0],g['records'][0]
    c.need((before['wire'],before['finish_len'])==(after['wire'],after['finish_len']),'same complete admitted wire')
    regressions=[]
    for i,(a,z) in enumerate(zip(before['append_capacity'],after['append_capacity'],strict=True)):
        if z>a:regressions.append(dict(checkpoint='append',buffer=i,before=a,after=z))
    if after['finish_capacity']>before['finish_capacity']:
        regressions.append(dict(checkpoint='finish',before=before['finish_capacity'],after=after['finish_capacity']))
    return dict(complete=True,checkpoint_pass=not regressions,regressions=regressions,
      baseline_finish_capacity=before['finish_capacity'],candidate_finish_capacity=after['finish_capacity'],
      baseline_append_capacity=before['append_capacity'],candidate_append_capacity=after['append_capacity'],
      finish_len=before['finish_len'],observations_per_image=2,root_adopted=False,
      full_resource_gate_pass=False,CPU_qualified=False,RSS_qualified=False,release_admitted=False)

if __name__=='__main__':
    values={v:json.loads((p.BASE/('retention-'+v)/'result.json').read_bytes()) for v in ('B','G')}
    result=reconcile(values);c.write(p.BASE,'checkpoint.json',result)
    if not result['checkpoint_pass']:c.write(p.BASE,'STOP.json',dict(reason='first checkpoint regression',root_adopted=False))
    print(json.dumps(result));raise SystemExit(0 if result['checkpoint_pass'] else 1)
