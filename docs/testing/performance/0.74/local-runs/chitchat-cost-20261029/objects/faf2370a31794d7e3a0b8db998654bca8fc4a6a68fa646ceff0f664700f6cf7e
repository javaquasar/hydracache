"""Reconcile the first control refusal offline; never reinterpret it as a pass."""
import json
import common as c
import analyze as a


def review():
    first=c.ROOT/'target/chitchat-cost-20261029'
    raw=(first/'build-B/native/cargo/stdout.log').read_text()
    errors=[json.loads(s)['message'] for s in raw.splitlines() if json.loads(s).get('reason')=='compiler-message']
    c.need(any(e.get('code',{}).get('code')=='E0599' and 'for_test' in e['message'] for e in errors),'first clean compile refusal')
    c.need(not json.loads((first/'build-B/result.json').read_bytes())['complete'],'compile result remains negative')
    for variant in ('B','P','Q'):
        for phase in ('build','check','clippy'):
            folder=c.BASE/(phase+'-'+variant)
            c.need(json.loads((folder/'result.json').read_bytes())['complete'],'complete scoped compiler checks')
            for name in ('execution.json','native-envelope.json'):
                r=json.loads((folder/name).read_bytes());c.need(r['exit']==0 and r['retired'] and not r['errors'],'retired compiler envelope')
    folder=c.BASE/'control-B';body=c.diagnostic((folder/'native/observation/stdout.log').read_text(),'HC074_CLEAN_COST:')
    c.need(body['rows']==json.loads((c.BASE/'input/expected-caller.json').read_bytes()) and body['mode']=='control' and body['variant']=='B','complete unchanged 30-cell semantics')
    c.need(body['allocation_wrapper'] is body['test_cfg'] is False and len(body['timings'])==0,'no cost cohort')
    try:a.validate(body,'B','control')
    except ValueError as error:c.need(str(error)=='no suspend/raw-clock discrepancy','exact first environment refusal')
    else:raise ValueError('first failed control must not be promoted')
    for name,exit_code in (('execution.json',1),('native-envelope.json',1),('native/observation/execution.json',0)):
        r=json.loads((folder/name).read_bytes())
        c.need(r['exit']==exit_code and r['retired'] and not r['errors'] and r['bound_seconds']==1200 and r['capacity_poll_seconds']==r['cleanup_seconds']==5,'complete original retirement and bounds')
    call=json.loads((folder/'native/observation/intent.json').read_bytes());image=json.loads((c.BASE/'build-B/native/artifact.json').read_bytes())
    c.need(call['argv']==[image['path'],'control'] and call['image_sha256']==image['sha256'],'original built clean image')
    for phase in ('control-P','control-Q',*[a+'-'+str(i) for a in ('cpu','memory') for i in range(1,31)]):
        c.need(not (c.BASE/phase).exists() and not (c.BASE/('launch-'+phase)).exists(),'no continuation after first control refusal')
    elapsed=body['elapsed'];difference=elapsed['raw']-elapsed['mono']
    return dict(complete=True,classification='blocked-first-cross-clock-control',clock_control_pass=False,
        CPU_qualified=False,RSS_qualified=False,allocator_retention_qualified=False,cohort_processes_dispatched=0,
        successful_build_check_clippy_phases=9,semantic_rows_matched=30,first_control_retained=True,
        elapsed_ns=elapsed,raw_minus_monotonic_ns=difference,relative_discrepancy=abs(difference)/elapsed['raw'],
        discrepancy_limit=c.protocol()['monotonic_raw_drift_fraction'],
        attribution='one short unequal-read window; saved deltas cannot distinguish clock-rate drift from read-order/scheduling skew',
        generic_component_gate_pass=False,root_adopted=False,release_admitted=False)


if __name__=='__main__':
    folder=c.BASE/'diagnosis';folder.mkdir(exist_ok=False);result=review();c.write(folder,'result.json',result);print(json.dumps(result))
