"""Exclusive first SSH streams; read-only host preflight, no dispatch."""
from pathlib import Path
import hashlib
import json
import subprocess
import sys

ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).resolve().parent
OUT=ROOT/'target/chitchat-host-cost-20261029';OUT.mkdir(exist_ok=False)
argv=['ssh','-T','-o','BatchMode=yes','-o','StrictHostKeyChecking=yes','-o','ConnectTimeout=15',
    '-o','ServerAliveInterval=5','-o','ServerAliveCountMax=2','hydracache-ax42','sudo -n /usr/bin/python3 -']
raw=(HERE/'host.py').read_bytes()
(OUT/'host-preflight-producer.py').write_bytes(raw)
(OUT/'preflight-intent.json').write_text(json.dumps(dict(argv=argv,source_sha256=hashlib.sha256(raw).hexdigest(),bound_seconds=45,read_only=True,retry=False),indent=2)+'\n',encoding='utf8')
with (OUT/'preflight.stdout.log').open('xb') as stdout,(OUT/'preflight.stderr.log').open('xb') as stderr:
    result=dict(complete=False,retry=False,read_only=True)
    try:
        child=subprocess.run(argv,input=raw,stdout=stdout,stderr=stderr,timeout=45,cwd=ROOT)
        result.update(exit=child.returncode,complete=child.returncode==0)
    except BaseException as error:result['refusal']=dict(type=type(error).__name__,reason=str(error))
    (OUT/'preflight-result.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf8')
print(json.dumps(result));raise SystemExit(0 if result['complete'] else 1)
