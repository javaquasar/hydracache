"""Exclusive first streams before imports/preflight; only fixed phases."""
from pathlib import Path
import json
import subprocess
import sys

ROOT=Path(__file__).resolve().parents[2];assert len(sys.argv)==2;label=sys.argv[1]
assert label in {'prepare','format','stage'}|{a+'-'+v for a in ('build','control','check','clippy') for v in ('B','P','Q')}|{a+'-'+str(i) for a in ('cpu','memory') for i in range(1,31)}
out=ROOT/'target/chitchat-cost-20261029-r1'/('launch-'+label);out.mkdir(parents=True,exist_ok=False)
(out/'producer.py').write_bytes(Path(__file__).read_bytes())
with (out/'stdout.log').open('xb') as stdout,(out/'stderr.log').open('xb') as stderr:
    result=dict(complete=False,retry=False)
    try:
        child=subprocess.run([sys.executable,'-B',str(Path(__file__).with_name('run.py')),label],cwd=ROOT,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,timeout=1220)
        result.update(exit=child.returncode,complete=child.returncode==0)
    except BaseException as error:result['first_refusal']=dict(type=type(error).__name__,reason=str(error))
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf8')
print(json.dumps(dict(phase=label,**result)));raise SystemExit(0 if result['complete'] else 1)
