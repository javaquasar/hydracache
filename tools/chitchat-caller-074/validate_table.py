"""Independent canonical-entry preflight, with first r1 failure retained."""
from pathlib import Path
import hashlib
import json


def validate(table):
    if table['seed']!=0x0741028 or len(table['cases'])!=30:raise ValueError('fixed table accounting')
    for case in table['cases']:
        active=False;has_kv=False;has_max=False;version=0;nodes=set();keys=set()
        for request in case['requests']:
            if request['kind']=='node':
                if request['id'] in nodes:raise ValueError('duplicate node: '+case['name'])
                nodes.add(request['id']);active=True;has_kv=False;has_max=False;version=0;keys=set()
            elif not active:raise ValueError('node required: '+case['name'])
            elif request['kind']=='kv':
                if has_max or request['version']<=version or request['key'] in keys:raise ValueError('noncanonical KV order: '+case['name'])
                has_kv=True;version=request['version'];keys.add(request['key'])
            elif request['kind']=='max':
                if has_kv or has_max:raise ValueError('max-version is for an empty node: '+case['name'])
                has_max=True;version=request['version']
            else:raise ValueError('unknown operation')


if __name__=='__main__':
    root=Path(__file__).resolve().parents[2];tools=Path(__file__).parent
    out=root/'target/chitchat-caller-20261029-r1/canonical-input-review';out.mkdir(exist_ok=False)
    (out/'producer.py').write_bytes(Path(__file__).read_bytes());result={}
    try:validate(json.loads((tools/'cases-r1.json').read_bytes()))
    except ValueError as error:result['first_table_refusal']=str(error)
    else:raise ValueError('original noncanonical table must refuse')
    validate(json.loads((tools/'cases-r2.json').read_bytes()))
    source=root/'target/chitchat-caller-20261029-r1/fixtures/B/src'
    result.update(complete=True,cohort_dispatched=False,current_Q_performance_observed=False,
        first_failure_stdout_sha256=hashlib.sha256((root/'target/chitchat-caller-20261029-r1/control-B/native/6/stdout.log').read_bytes()).hexdigest(),
        source={n:hashlib.sha256((source/n).read_bytes()).hexdigest() for n in ('delta.rs','state.rs')})
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf8')
    print('canonical caller input: first r1 refusal retained; r2 preflight passed')
