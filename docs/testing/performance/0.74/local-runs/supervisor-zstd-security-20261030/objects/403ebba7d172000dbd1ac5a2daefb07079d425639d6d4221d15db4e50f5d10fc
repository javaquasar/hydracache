"""Targeted Windows formatting after the retained whole-workspace OS206 refusal."""
import json
import os
from pathlib import Path
import subprocess
import sys
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'supervisor-zstd-security-074'))
import common as c

OUT=c.BASE/'format-package-194'

def command():
    tool,env,suffix=c.environment('194')
    return [str(tool/'cargo.exe'),'fmt','--manifest-path',str(c.ROOT/'Cargo.toml'),'-p',
      'hydracache-long-run-supervisor-074','--','--check'],env

def main():
    OUT.mkdir(exist_ok=False);before=c.source();head=c.snapshot();argv,_=command()
    c.write(OUT,'intent.json',dict(argv=argv,source=before,git=head,producer_sha256=c.sha(Path(__file__).read_bytes()),bound=1200,floor=4*1024**3,poll=5,cleanup=5,retry=False,full_workspace_format=False))
    with (OUT/'stdout.log').open('xb') as stdout,(OUT/'stderr.log').open('xb') as stderr:
        child=subprocess.run([sys.executable,'-B',str(Path(__file__)),'worker'],cwd=c.ROOT,
          stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr,timeout=1210)
    execution=json.loads((OUT/'execution.json').read_bytes())
    ok=child.returncode==execution['exit']==0 and execution['retired'] and not execution['errors'] and c.source()==before and c.snapshot()==head
    c.write(OUT,'result.json',dict(complete=ok,full_workspace_format=False,release_admitted=False))
    print(json.dumps(dict(targeted_format_passed=ok)));raise SystemExit(0 if ok else 1)

if __name__=='__main__':
    c.need(os.name=='nt','Windows-only focused formatter')
    if len(sys.argv)==2:
        c.need(sys.argv[1]=='worker','fixed worker');argv,env=command();os.environ.clear();os.environ.update(env)
        result=c.owned._owned(c.owned.load_job().WindowsJob(),argv,str(c.ROOT),c.capacity,bound=1200)
        c.write(OUT,'execution.json',result);raise SystemExit(result['exit'])
    c.need(len(sys.argv)==1,'no command overrides');main()
