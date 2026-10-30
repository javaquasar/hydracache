"""First deterministic resource observation; stop F if finish output grows."""
import json
import os
from pathlib import Path
import re
import shutil
import sys
import material as m
p=m.p;c=p.c
sys.path.insert(0,str(p.HERE));import run as w
ORIGINAL=p.BASE
p.BASE=ORIGINAL/'retention'
SUFFIX='\n#[cfg(test)]\n#[path="retention_probe.rs"]\nmod retained_finish;\n#[cfg(test)]\nconst RETENTION_VARIANT:&str="{v}";\n'

def prepare():
    c.need(p.EXECUTION_ALLOWED and not (ORIGINAL/'STOP.json').exists(),'F declined; no new resource preparation')
    p.BASE.mkdir(exist_ok=False)
    held={}
    for v in ('B','F'):
        origin=ORIGINAL/'fixtures'/v;folder=p.BASE/'fixtures'/v;shutil.copytree(origin,folder)
        held[v]=c.tree(origin)
        path=folder/'src/serialize.rs';path.write_bytes(path.read_bytes()+SUFFIX.format(v=v).encode())
        (folder/'src/retention_probe.rs').write_bytes((m.HERE/'retention_probe.rs').read_bytes())
    c.write(p.BASE,'preparation.json',dict(original=held,resource_observations_started_before=False,
      repetitions=2,order=['B','F'],scope='returned Vec capacity only',threshold_increase_allowed=0,root_adopted=False))

original_source=w.source
def source():
    value=original_source()
    for path in m.HERE.iterdir():
        if path.is_file():value[path.relative_to(c.ROOT).as_posix()]=c.sha(path.read_bytes())
    held=json.loads((p.BASE/'preparation.json').read_bytes())['original']
    for v,tree in held.items():
        c.need(c.tree(ORIGINAL/'fixtures'/v)==tree,'whole unchanged original fixture')
        folder=p.BASE/'fixtures'/v
        for name,digest in tree.items():
            raw=(folder/name).read_bytes()
            if name=='src/serialize.rs':
                suffix=SUFFIX.format(v=v).encode();c.need(raw.endswith(suffix),'one test-only module suffix');raw=raw[:-len(suffix)]
            c.need(c.sha(raw)==digest,'probe production source byte identity: '+name)
    return value
w.source=source
original_command=w.command
def command(mode):
    c.need(p.EXECUTION_ALLOWED and not (ORIGINAL/'STOP.json').exists(),'F declined; no repeated resource observation')
    action,v,compiler=w.parts(mode)
    if action!='probe':return original_command(mode)
    _,env,_=c.environment(compiler)
    return [str(w.image(v,compiler)),'--exact','serialize::retained_finish::retained_finish_output','--test-threads=1','--show-output'],env,5
w.command=command
original_accounting=w.accounting
def accounting(mode):
    action,v,compiler=w.parts(mode)
    if action!='probe':return original_accounting(mode)
    raw=(w.phase(mode)/'stdout.log').read_text().replace('\r\n','\n')
    catalog=json.loads((w.phase(w.label('catalog',v,compiler))/'result.json').read_bytes())['catalog']
    c.need(re.findall(r'^test (.+?) \.\.\. (ok|FAILED)$',raw,re.M)==[('serialize::retained_finish::retained_finish_output','ok')],'one actual resource probe')
    c.need(re.findall(r'^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',raw,re.M)==[('1','0','0','0',str(len(catalog)-1))],'complete resource case accounting')
    rows=[s.split('HC074_RETAINED_FINISH:',1)[1] for s in raw.splitlines() if 'HC074_RETAINED_FINISH:' in s]
    c.need(len(rows)==1,'one whole resource report');report=json.loads(rows[0])
    c.need(report['variant']==v and report['records'][0]==report['records'][1] and len(report['records'])==2,'actual runtime variant and inner repetition')
    return dict(passed=1,failed=0,observation=report)
w.accounting=accounting

def analyze():
    values={v:json.loads((p.BASE/('probe-'+v)/'result.json').read_bytes()) for v in ('B','F')}
    c.need(all(row['complete'] for row in values.values()),'first complete resource cases')
    b,f=[values[v]['observation'] for v in ('B','F')]
    for key in ('seed','name','threshold','budget','payload','scope','CPU_qualified','RSS_qualified'):
        c.need(b[key]==f[key],'same original resource input/scope')
    c.need(b['seed']==0x0741028 and b['threshold']==64 and b['budget']==167,'frozen first stateful case')
    before,after=b['records'][0],f['records'][0]
    c.need((before['finish_len'],before['wire'])==(after['finish_len'],after['wire']),'same complete accepted work')
    delta=after['finish_capacity']-before['finish_capacity']
    result=dict(complete=True,resource_gate_pass=delta<=0,baseline_finish_capacity=before['finish_capacity'],
      candidate_finish_capacity=after['finish_capacity'],delta_bytes=delta,observations_per_image=2,
      decision='declined-first-retained-output-regression' if delta>0 else 'checkpoint-only',
      full_allocation_gate_pass=False,CPU_qualified=False,RSS_qualified=False,root_adopted=False,release_admitted=False,
      further_measurements_allowed=delta<=0)
    c.write(p.BASE,'analysis.json',result);print(json.dumps(result))

if __name__=='__main__':
    if sys.argv[1]=='prepare':prepare()
    elif sys.argv[1]=='analyze':analyze()
    elif len(sys.argv)==3:
        c.need(sys.argv[2]=='worker','fixed worker');argv,env,bound=command(sys.argv[1]);os.environ.clear();os.environ.update(env)
        _,v,_=w.parts(sys.argv[1]);own=c.owned._owned(c.owned.load_job().WindowsJob(),argv,str(p.BASE/'fixtures'/v),c.capacity,bound=bound)
        c.write(w.phase(sys.argv[1]),'execution.json',own);raise SystemExit(own['exit'])
    else:
        # The qualified owner launches its own module. Point __file__ at this
        # maintained adapter so both controller and worker install these bindings.
        w.__file__=__file__;w.main(sys.argv[1])
